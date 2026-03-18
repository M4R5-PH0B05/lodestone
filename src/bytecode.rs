// ─────────────────────────────────────────────────────────────────────────────
// bytecode.rs — Heuristic side-detection via JVM constant-pool scanning
//
// Java .class files store every class reference as a plain UTF-8 string in
// their constant pool.  We don't need a full bytecode parser — we can extract
// those strings from raw bytes in a single pass and match them against known
// client-only or server-only class prefixes/names.
//
// Signal priority (highest wins):
//   1. @OnlyIn(Dist.CLIENT) / @Environment(EnvType.CLIENT) annotations   → Client
//   2. @OnlyIn(Dist.DEDICATED_SERVER) / @Environment(EnvType.SERVER)     → Server
//   3. References to client-exclusive classes (RenderSystem, Screen, …)  → Client
//   4. References to dedicated-server-exclusive classes                   → Server
//   5. No evidence found                                                  → None
// ─────────────────────────────────────────────────────────────────────────────

use std::io::Read;

// ── Signal tables ─────────────────────────────────────────────────────────────

/// Class-name prefixes/exact strings that only exist on the client dist.
const CLIENT_CLASS_SIGNALS: &[&str] = &[
    // Minecraft client core
    "net/minecraft/client/Minecraft",
    "net/minecraft/client/renderer/",
    "net/minecraft/client/gui/screens/",
    "net/minecraft/client/gui/components/",
    "net/minecraft/client/KeyMapping",
    "net/minecraft/client/player/LocalPlayer",
    "net/minecraft/client/Camera",
    "net/minecraft/client/Options",
    "net/minecraft/client/resources/model/",
    "net/minecraft/client/sounds/",
    "net/minecraft/client/particle/",
    "net/minecraft/client/multiplayer/ClientLevel",
    "net/minecraft/client/multiplayer/ClientPacketListener",
    // Blaze3D / LWJGL rendering — never present server-side
    "com/mojang/blaze3d/systems/RenderSystem",
    "com/mojang/blaze3d/vertex/",
    "com/mojang/blaze3d/platform/",
    "org/lwjgl/opengl/",
    "org/lwjgl/glfw/",
    // Fabric client API
    "net/fabricmc/fabric/api/client/",
    "net/fabricmc/fabric/impl/client/",
    // NeoForge / Forge client events
    "net/neoforged/neoforge/client/",
    "net/minecraftforge/client/",
    "net/minecraftforge/fml/client/",
];

/// Class-name prefixes/exact strings that only exist on the dedicated server.
const SERVER_CLASS_SIGNALS: &[&str] = &[
    "net/minecraft/server/dedicated/DedicatedServer",
    "net/minecraft/server/dedicated/Settings",
    "net/minecraft/server/dedicated/DedicatedPlayerList",
    "net/minecraft/server/rcon/",
    // Fabric server API
    "net/fabricmc/fabric/api/event/lifecycle/v1/ServerLifecycleEvents",
    // NeoForge / Forge dedicated-server events
    "net/neoforged/neoforge/server/",
    "net/minecraftforge/fml/server/",
];

/// Annotation descriptor strings that declare a class/member as client-only.
/// These appear literally in the RuntimeVisibleAnnotations attribute.
const CLIENT_ANNOTATION_SIGNALS: &[&str] = &[
    // Forge / NeoForge
    "net/minecraftforge/api/distmarker/OnlyIn",
    "net/neoforged/api/distmarker/OnlyIn",
    // Fabric
    "net/fabricmc/api/Environment",
    // The *value* field that would follow in the annotation — sanity check
    // that we're reading CLIENT not SERVER (checked separately below)
    "Dist.CLIENT",
    "EnvType.CLIENT",
    "CLIENT",   // the enum constant value stored as a UTF-8 string
];

/// Annotation strings that declare server-only.
const SERVER_ANNOTATION_SIGNALS: &[&str] = &[
    "net/minecraftforge/api/distmarker/OnlyIn",
    "net/neoforged/api/distmarker/OnlyIn",
    "net/fabricmc/api/Environment",
    "Dist.DEDICATED_SERVER",
    "EnvType.SERVER",
    "DEDICATED_SERVER",
    "SERVER",
];

// ── Public types ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectedSide {
    Client,
    Server,
    Both,       // signals for both found — unusual but possible
    Unknown,    // no evidence
}

/// How confident we are in the detection result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Confidence {
    /// Explicit annotation (`@OnlyIn`, `@Environment`)
    Annotation,
    /// Reference to a class that cannot exist outside that dist
    ClassReference,
    /// No signal found
    None,
}

#[derive(Debug, Clone)]
pub struct BytecodeEvidence {
    pub side:       DetectedSide,
    pub confidence: Confidence,
    /// A representative signal string (for display in the UI)
    pub signal:     Option<String>,
    /// How many .class files were scanned inside the jar
    pub classes_scanned: usize,
}

impl BytecodeEvidence {
    pub fn unknown() -> Self {
        Self {
            side: DetectedSide::Unknown,
            confidence: Confidence::None,
            signal: None,
            classes_scanned: 0,
        }
    }
}

// ── Core scanning ─────────────────────────────────────────────────────────────

/// Scan every .class file inside a jar for side-detection signals.
/// Returns `None` if the jar cannot be opened as a zip archive.
pub fn analyse_jar(path: &str) -> Option<BytecodeEvidence> {
    let file = std::fs::File::open(path).ok()?;
    let mut archive = zip::ZipArchive::new(file).ok()?;

    let mut client_signals:      Vec<String> = Vec::new();
    let mut server_signals:      Vec<String> = Vec::new();
    let mut client_annotations:  Vec<String> = Vec::new();
    let mut server_annotations:  Vec<String> = Vec::new();
    let mut classes_scanned = 0usize;

    for i in 0..archive.len() {
        let mut entry = match archive.by_index(i) {
            Ok(e) => e,
            Err(_) => continue,
        };

        // Only scan .class files
        if !entry.name().ends_with(".class") {
            continue;
        }
        classes_scanned += 1;

        // Read raw bytes — we scan the constant pool directly
        let mut bytes = Vec::new();
        if entry.read_to_end(&mut bytes).is_err() {
            continue;
        }

        scan_class_bytes(
            &bytes,
            &mut client_signals,
            &mut server_signals,
            &mut client_annotations,
            &mut server_annotations,
        );
    }

    // ── Determine result from accumulated signals ──────────────────────────

    // Annotations beat class references
    let (side, confidence, signal) = if !client_annotations.is_empty()
        && server_annotations.is_empty()
    {
        (
            DetectedSide::Client,
            Confidence::Annotation,
            client_annotations.first().cloned(),
        )
    } else if !server_annotations.is_empty() && client_annotations.is_empty() {
        (
            DetectedSide::Server,
            Confidence::Annotation,
            server_annotations.first().cloned(),
        )
    } else if !client_signals.is_empty() && server_signals.is_empty() {
        (
            DetectedSide::Client,
            Confidence::ClassReference,
            client_signals.first().cloned(),
        )
    } else if !server_signals.is_empty() && client_signals.is_empty() {
        (
            DetectedSide::Server,
            Confidence::ClassReference,
            server_signals.first().cloned(),
        )
    } else if !client_signals.is_empty() && !server_signals.is_empty() {
        // Both — treat as Both (universal mod)
        (
            DetectedSide::Both,
            Confidence::ClassReference,
            Some(format!(
                "client: {}, server: {}",
                client_signals[0],
                server_signals[0]
            )),
        )
    } else {
        (DetectedSide::Unknown, Confidence::None, None)
    };

    Some(BytecodeEvidence {
        side,
        confidence,
        signal,
        classes_scanned,
    })
}

// ── Low-level constant-pool extraction ───────────────────────────────────────
//
// JVM .class constant pool format (JVMS §4.4):
//   Magic:      4 bytes  (CA FE BA BE)
//   Minor ver:  2 bytes
//   Major ver:  2 bytes
//   Pool count: 2 bytes  (N)
//   Pool[1..N-1] entries, each starting with a 1-byte tag:
//     1  = Utf8   → u16 length, then <length> bytes of MUTF-8
//     3  = Integer → 4 bytes
//     4  = Float   → 4 bytes
//     5  = Long    → 8 bytes (takes two slots)
//     6  = Double  → 8 bytes (takes two slots)
//     7  = Class   → u16 name_index
//     8  = String  → u16 string_index
//     9  = Fieldref    → u16 + u16
//     10 = Methodref   → u16 + u16
//     11 = InterfaceMethodref → u16 + u16
//     12 = NameAndType → u16 + u16
//     15 = MethodHandle → u8 + u16
//     16 = MethodType  → u16
//     17 = Dynamic     → u16 + u16
//     18 = InvokeDynamic → u16 + u16
//     19 = Module      → u16
//     20 = Package     → u16
//
// We only need the Utf8 entries (tag 1).  We walk the pool stopping when we
// run out of bytes, collecting every Utf8 string, then match them.

fn read_u16(bytes: &[u8], pos: usize) -> Option<usize> {
    if pos + 1 >= bytes.len() { return None; }
    Some(((bytes[pos] as usize) << 8) | (bytes[pos + 1] as usize))
}

fn scan_class_bytes(
    bytes: &[u8],
    client_signals:     &mut Vec<String>,
    server_signals:     &mut Vec<String>,
    client_annotations: &mut Vec<String>,
    server_annotations: &mut Vec<String>,
) {
    // Validate magic
    if bytes.len() < 10 || &bytes[0..4] != b"\xCA\xFE\xBA\xBE" {
        return;
    }

    let pool_count = match read_u16(bytes, 8) {
        Some(n) => n,
        None => return,
    };

    let mut pos = 10usize; // byte offset after pool_count
    let mut i = 1usize;   // constant pool is 1-indexed, slot 0 unused

    while i < pool_count && pos < bytes.len() {
        let tag = bytes[pos];
        pos += 1;

        match tag {
            1 => {
                // Utf8 — the one we care about
                let len = match read_u16(bytes, pos) {
                    Some(l) => l,
                    None => return,
                };
                pos += 2;
                if pos + len > bytes.len() { return; }
                let s = std::str::from_utf8(&bytes[pos..pos + len])
                    .unwrap_or("");
                classify_utf8(s, client_signals, server_signals,
                               client_annotations, server_annotations);
                pos += len;
                i += 1;
            }
            3 | 4 => { pos += 4; i += 1; }             // Integer, Float
            5 | 6 => { pos += 8; i += 2; }             // Long, Double (take 2 slots)
            7 | 8 | 16 | 19 | 20 => { pos += 2; i += 1; } // Class, String, MethodType, Module, Package
            9 | 10 | 11 | 12 | 17 | 18 => { pos += 4; i += 1; } // *ref, NameAndType, Dynamic, InvokeDynamic
            15 => { pos += 3; i += 1; }                // MethodHandle
            _ => {
                // Unknown tag — we can't safely continue
                return;
            }
        }
    }
}

fn classify_utf8(
    s: &str,
    client_signals:     &mut Vec<String>,
    server_signals:     &mut Vec<String>,
    client_annotations: &mut Vec<String>,
    server_annotations: &mut Vec<String>,
) {
    // Cap per-class signal lists to avoid scanning forever on huge jars
    if client_signals.len() + server_signals.len() >= 32 { return; }

    // Annotation check first (higher priority)
    let is_annotation_descriptor = s.contains("OnlyIn")
        || s.contains("Environment")
        || s == "CLIENT"
        || s == "DEDICATED_SERVER"
        || s == "EnvType";

    if is_annotation_descriptor {
        // Client annotation signals
        if s.contains("OnlyIn") || s.contains("Environment") || s == "EnvType" {
            // The annotation class itself — we record it; the value (CLIENT /
            // DEDICATED_SERVER) comes separately in the pool
            // Just record the presence for now; combine with value below
        }
        if s == "CLIENT" || s.contains("EnvType.CLIENT") || s.contains("Dist.CLIENT") {
            if client_annotations.len() < 4 {
                client_annotations.push(s.to_string());
            }
            return;
        }
        if s == "DEDICATED_SERVER"
            || s.contains("EnvType.SERVER")
            || s.contains("Dist.DEDICATED_SERVER")
        {
            if server_annotations.len() < 4 {
                server_annotations.push(s.to_string());
            }
            return;
        }
    }

    // Class reference signals
    for sig in CLIENT_CLASS_SIGNALS {
        if s.starts_with(sig) || s.contains(sig) {
            if client_signals.len() < 8 {
                // Store just the matched signal, not the full (potentially long) string
                client_signals.push(sig.to_string());
            }
            return;
        }
    }
    for sig in SERVER_CLASS_SIGNALS {
        if s.starts_with(sig) || s.contains(sig) {
            if server_signals.len() < 8 {
                server_signals.push(sig.to_string());
            }
            return;
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_class(utf8_strings: &[&str]) -> Vec<u8> {
        // Minimal valid class file with a hand-built constant pool
        let mut pool: Vec<u8> = Vec::new();
        let mut count: u16 = 1;

        for s in utf8_strings {
            let bytes = s.as_bytes();
            pool.push(1); // Utf8 tag
            pool.push((bytes.len() >> 8) as u8);
            pool.push((bytes.len() & 0xFF) as u8);
            pool.extend_from_slice(bytes);
            count += 1;
        }

        let mut out = Vec::new();
        out.extend_from_slice(b"\xCA\xFE\xBA\xBE"); // magic
        out.extend_from_slice(&[0u8, 0]);            // minor
        out.extend_from_slice(&[0u8, 63]);           // major (Java 19)
        out.push((count >> 8) as u8);
        out.push((count & 0xFF) as u8);
        out.extend_from_slice(&pool);
        out
    }

    #[test]
    fn detects_rendersystem_as_client() {
        let bytes = make_class(&["com/mojang/blaze3d/systems/RenderSystem"]);
        let mut cs = Vec::new(); let mut ss = Vec::new();
        let mut ca = Vec::new(); let mut sa = Vec::new();
        scan_class_bytes(&bytes, &mut cs, &mut ss, &mut ca, &mut sa);
        assert!(!cs.is_empty(), "should detect RenderSystem as client signal");
        assert!(ss.is_empty());
    }

    #[test]
    fn detects_client_annotation() {
        let bytes = make_class(&["CLIENT"]);
        let mut cs = Vec::new(); let mut ss = Vec::new();
        let mut ca = Vec::new(); let mut sa = Vec::new();
        scan_class_bytes(&bytes, &mut cs, &mut ss, &mut ca, &mut sa);
        assert!(!ca.is_empty(), "should detect CLIENT annotation signal");
    }

    #[test]
    fn detects_dedicated_server() {
        let bytes = make_class(&["net/minecraft/server/dedicated/DedicatedServer"]);
        let mut cs = Vec::new(); let mut ss = Vec::new();
        let mut ca = Vec::new(); let mut sa = Vec::new();
        scan_class_bytes(&bytes, &mut cs, &mut ss, &mut ca, &mut sa);
        assert!(!ss.is_empty(), "should detect DedicatedServer as server signal");
        assert!(cs.is_empty());
    }
}
