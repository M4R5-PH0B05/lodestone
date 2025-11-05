// PROJECT: **LODESTONE**
// OUTLINE: A GUI-Based application which uses community driven selection to identify client and server-side-based MC mods. It will use 'modules' that can be loaded that will flag certain mods with tags.
//          There will be a 'default' modul e, which will identify 'client' and 'server' side mods.
//          Custom modules could isolate dependencies for certain mods, mods for certain modpacks, common
//          incompatibilities between mods, etc...
//          All modules will be in JSON format.
//          The module header will contain a name, creator and version.
//          The content will be the mod id, e.g. 'create' and the type, e.g. 'client' or 'server'.
//          The application will have multiple functions, however, the primary ones are:
//             - A 'remove all' button of a certain tag, for example, to remove all client side mods.
//              - A way to tag mods yourself, if a mod is not recognized by the current module.
//              - There will then be a way to submit this to the module author, and they may choose to
//                integrate it into the current module.
//              - There will be a button to move all mods of a certain type to a different directory.
//              - There will be a button to ZIP all mods of a certain type.
//              - There will be a button to write all names of mods of a certain type to a .txt file.
//          TBD
//
// DATA STRUCTURES: A 'Mod' STRUCT will be created. It will have the following attributes:
//                      - ModID: String ( e.g. 'create' )
//                      - ModVersion: f64 ( e.g. 1.23 )
//                      - ModType: Enum. (e.g. 'client')
//                  As mentioned before, there will be an ENUM called default_tags. It will have 4 values:
//                      - Client
//                      - Server
//                      - Both
//                      - Unknown
//                  There will also be a 'Module' STRUCT. It will have the following attributes:
//                      - ModuleName: String
//                      - ModuleVersion: f64
//                      - ModuleAuthor: String
//                      - Mods: Vector<Mod>
//
// PROJECT FLOW: Disclosed in the 'DRAWIO' diagram in the root directory.
//               JSON file will be loaded. ALl mods will be read into mod structs, all mod structs
//               Will be put into vector, which will be loaded into module struct.
//               When an operation is chosen, the directory will be searched by all mods in vector.
//               When a mod is found, an operation will be conducted.
//

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum DefaultTags {
    Unknown,
    Client,
    Server,
    Both
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
enum ModTypes {
    Unknown,
    Forge,
    NeoForge,
    Fabric,
    Quilt
}

#[derive(Debug, Clone, Serialize, Deserialize)]
// Previously also had mod_id, but that is being used as the key
// For the B-Tree storage solution, so the mod_id property
// was removed to prevent data duplication
struct Mod {
    // changed to String to match test.json values like "0.5.8.29"
    mod_version: String,
    mod_tag: DefaultTags,
    // changed from String to ModTypes so mod_type uses the enum
    mod_type: ModTypes
}
#[derive(Debug, Deserialize, Serialize)]
struct ModuleHeader {
    module_name: String,
    // changed to f64 to match Module.module_version (and the numeric value in test.json)
    module_version: f64,
    module_author: String
}
#[derive(Debug, Deserialize, Serialize)]
struct ModuleJson {
    header: ModuleHeader,
    // Previously a vector could be a hashmap, although mod ver would have to be removed.
    // I believe a B-Tree is optimal
    mods: BTreeMap<String, Mod>
}
#[derive(Debug)]
struct Module {
    module_name: String,
    module_version: f64,
    module_author: String,
    mods: BTreeMap<String,Mod>
}


// Helper function to take input and return string
fn input_str(print: &str) -> String{
    // Print message
    println!("{}",print);
    // Create storage variable
    let mut input_str = String::new();
    // Read input
    io::stdin()
        .read_line(&mut input_str)
        .expect("Failed to read line");
    // Return input
    input_str.trim().to_string()
}

// Helper function to take input and return integer
fn input_num(prompt: &str) -> i32 {
    // Enter Loop
    loop {
        // Print message
        print!("{} ", prompt);
        io::stdout().flush().ok();
        // Create storage variable
        let mut input_str = String::new();
        // Try and read input
        if let Err(_) = io::stdin().read_line(&mut input_str) {
            println!("Couldn't read input. Please try again.");
            continue;
        }
        // Trim it
        let trimmed = input_str.trim();
        // Convert it to an integer
        match trimmed.parse::<i32>() {
            Ok(n) => return n,
            Err(_) => println!("`\nInvalid input: `{}`. Please enter a whole number.", trimmed),
        }
    }
}
fn get_jar_files(dir_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut jar_files = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        // Only collect .jar files (uncomment the branch below if you want to filter by extension)
        if path.extension().and_then(|s| s.to_str()) == Some("jar") {
            if let Some(file_name) = path.file_name() {
                jar_files.push(file_name.to_string_lossy().to_string());
            }
        }
    }

    Ok(jar_files)
}

// Helper function to get mod ID, mod Type and detected version from the JAR file
// Now returns the detected type as ModTypes and detected version as Option<String>
fn get_mod_id_and_type(path: &str) -> Result<Option<(String, ModTypes, Option<String>)>, Box<dyn std::error::Error>> {
    // tiny helpers to parse versions as strings
    fn parse_toml_version(v: &toml::Value) -> Option<String> {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        if let Some(f) = v.as_float() {
            return Some(f.to_string());
        }
        if let Some(i) = v.as_integer() {
            return Some(i.to_string());
        }
        None
    }
    fn parse_json_version(v: &serde_json::Value) -> Option<String> {
        if let Some(s) = v.as_str() {
            return Some(s.to_string());
        }
        if let Some(n) = v.as_f64() {
            return Some(n.to_string());
        }
        None
    }

    // Open the file from the path
    let file = fs::File::open(path)?;
    // Unzip it (A jar file is basically a ZIP)
    let mut archive = zip::ZipArchive::new(file)?;
    // Iterate over the length and produce all file paths in the JAR directory
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name();

        if name.ends_with("mods.toml") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            println!("Found Forge-style mods.toml: \n{}", contents);
            // Heuristic: if contents mention "neoforge" assume NeoForge, otherwise Forge
            let lower = contents.to_lowercase();
            let found_type = if lower.contains("neoforge") || lower.contains("neo-forge") {
                ModTypes::NeoForge
            } else {
                ModTypes::Forge
            };
            let parsed: toml::Value = toml::from_str(&contents)?;
            let mod_id = parsed
                .get("mods")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|mod_entry| mod_entry.get("modId"))
                .and_then(|id| id.as_str())
                .map(String::from);
            let detected_version = parsed
                .get("mods")
                .and_then(|v| v.as_array())
                .and_then(|arr| arr.first())
                .and_then(|mod_entry| {
                    // try common keys: "version" or "modVersion"
                    mod_entry.get("version")
                        .or_else(|| mod_entry.get("modVersion"))
                })
                .and_then(|ver| parse_toml_version(ver));
            return Ok(mod_id.map(|id| (id, found_type, detected_version)));

        } else if name.ends_with("fabric.mod.json") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            println!("Found Fabric mods.json: \n{}", contents);
            let parsed: serde_json::Value = serde_json::from_str(&contents)?;
            let mod_id = parsed
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from);
            let detected_version = parsed.get("version").and_then(|v| parse_json_version(v));
            return Ok(mod_id.map(|id| (id, ModTypes::Fabric, detected_version)));

        } else if name.ends_with("mcmod.info") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            println!("Found mcmod.info: \n{}", contents);
            // mcmod.info is a legacy JSON array; treat as Forge
            let parsed: serde_json::Value = serde_json::from_str(&contents)?;
            let mod_id = parsed
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|mod_entry| mod_entry.get("modid"))
                .and_then(|id| id.as_str())
                .map(String::from);
            let detected_version = parsed
                .as_array()
                .and_then(|arr| arr.first())
                .and_then(|mod_entry| mod_entry.get("version"))
                .and_then(|v| parse_json_version(v));
            return Ok(mod_id.map(|id| (id, ModTypes::Forge, detected_version)));
        }
    }

    // If nothing matched in the archive, return None
    Ok(None)
}

impl Module {
    fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let json_string = fs::read_to_string(path)?;
        let json_data: ModuleJson = serde_json::from_str(&json_string)?;
        // Convert ModuleJson to Module
        Ok(Self::from_json(json_data))
    }
    fn from_json(json_data: ModuleJson) -> Self {
        Self {
            module_name: json_data.header.module_name,
            // header.module_version is now f64
            module_version: json_data.header.module_version,
            module_author: json_data.header.module_author,
            mods: json_data.mods,
        }
    }

    // Get a mod by its ID
    fn get_mod_type(&self,mod_id: &str) -> Option<&DefaultTags> {
        self.mods.get(mod_id).map(|m| &m.mod_tag)
    }

    // Get all mods with a certain Tag
    fn get_mods_by_type(&self, tag: &DefaultTags) -> Vec<&Mod> {
        // use direct equality now that DefaultTags derives PartialEq
        self.mods
            .values()
            .filter(|m| m.mod_tag == *tag)
            .collect()
    }

    //Print Info
    fn print_info(&self) {
        println!("Module: {}", self.module_name);
        println!("Version: {}", self.module_version);
        println!("Author: {}", self.module_author);
        println!("Total mods: {}", self.mods.len());
        println!("\nMods (alphabetically):");
        for (mod_id, mod_entry) in &self.mods {
            println!("  {} v{} - {:?}", mod_id, mod_entry.mod_version, mod_entry.mod_tag);
        }
    }

}


// New helper: look for other modules in ./modules and let the user choose (0 = defaults)
fn choose_module_file() -> String {
    let mut module_files: Vec<String> = Vec::new();

    // Try reading ./modules directory; if it doesn't exist or empty, we fall back to default
    if let Ok(entries) = fs::read_dir("modules") {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                    module_files.push(format!("modules/{}", fname));
                }
            }
        }
    }

    if module_files.is_empty() {
        println!("No additional modules found, loading default 'test.json'.");
        return "test.json".to_string();
    }

    println!("Found additional module files:");
    println!("  0) Default (test.json)");
    for (i, f) in module_files.iter().enumerate() {
        println!("  {}) {}", i + 1, f);
    }

    loop {
        let choice = input_num("Select module number (0 for default):");
        if choice == 0 {
            return "test.json".to_string();
        }
        let idx = (choice - 1) as usize;
        if idx < module_files.len() {
            return module_files[idx].clone();
        }
        println!("Invalid selection, try again.");
    }
}


// Parse DefaultTags from a string (case-insensitive)
fn parse_default_tag(s: &str) -> DefaultTags {
    match s.to_lowercase().as_str() {
        "client" => DefaultTags::Client,
        "server" => DefaultTags::Server,
        "both" => DefaultTags::Both,
        _ => DefaultTags::Unknown,
    }
}

// Create a new module JSON file with header and empty mods map
fn new_module(file_path: &str, name: &str, version: f64, author: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Build new ModuleJson
    let header = ModuleHeader {
        module_name: name.to_string(),
        module_version: version,
        module_author: author.to_string(),
    };
    let mods: BTreeMap<String, Mod> = BTreeMap::new();
    let module_json = ModuleJson {
        header,
        mods,
    };
    // Write to the file
    let file = fs::File::create(file_path)?;
    serde_json::to_writer_pretty(file, &module_json)?;
    Ok(())
}

// Add a mod entry to an existing module file
fn add_mod_to_module(
    file_path: &str,
    mod_id: &str,
    mod_version: &str,
    mod_tag: DefaultTags,
    mod_type: ModTypes,
) -> Result<(), Box<dyn std::error::Error>> {
    let json_string = fs::read_to_string(file_path)?;
    let mut module_json: ModuleJson = serde_json::from_str(&json_string)?;
    let new_mod = Mod {
        mod_version: mod_version.to_string(),
        mod_tag,
        mod_type,
    };
    module_json.mods.insert(mod_id.to_string(), new_mod);
    let file = fs::File::create(file_path)?;
    serde_json::to_writer_pretty(file, &module_json)?;
    Ok(())
}

// Remove a mod by ID from an existing module file. Returns true if removed.
fn remove_mod_from_module(file_path: &str, mod_id: &str) -> Result<bool, Box<dyn std::error::Error>> {
    let json_string = fs::read_to_string(file_path)?;
    let mut module_json: ModuleJson = serde_json::from_str(&json_string)?;
    let removed = module_json.mods.remove(mod_id).is_some();
    if removed {
        let file = fs::File::create(file_path)?;
        serde_json::to_writer_pretty(file, &module_json)?;
    }
    Ok(removed)
}

// Edit an existing mod. Any Option that is None leaves that field unchanged.
// Returns true if the mod existed and was updated.
fn edit_mod_in_module(
    file_path: &str,
    mod_id: &str,
    new_version: Option<&str>,
    new_tag: Option<DefaultTags>,
    new_type: Option<ModTypes>,
) -> Result<bool, Box<dyn std::error::Error>> {
    let json_string = fs::read_to_string(file_path)?;
    let mut module_json: ModuleJson = serde_json::from_str(&json_string)?;
    if let Some(mod_entry) = module_json.mods.get_mut(mod_id) {
        if let Some(v) = new_version {
            mod_entry.mod_version = v.to_string();
        }
        if let Some(t) = new_tag {
            mod_entry.mod_tag = t;
        }
        if let Some(tp) = new_type {
            mod_entry.mod_type = tp;
        }
        let file = fs::File::create(file_path)?;
        serde_json::to_writer_pretty(file, &module_json)?;
        return Ok(true);
    }
    Ok(false)
}


// Zip all jar files whose modules are tagged with `tag`. Returns the number of files added to zip.
fn zip_files_with_tag(
    dir: &str,
    jar_to_modid: &BTreeMap<String, String>,
    module: &Module,
    tag: DefaultTags,
    output_zip: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    use zip::write::FileOptions;
    let out_file = fs::File::create(output_zip)?;
    let mut zip = zip::ZipWriter::new(out_file);
    let mut count = 0usize;
    for (jar, modid) in jar_to_modid {
        if let Some(mod_entry) = module.mods.get(modid) {
            if mod_entry.mod_tag == tag {
                let path = Path::new(dir).join(jar);
                if path.is_file() {
                    let mut f = fs::File::open(&path)?;
                    let mut buffer = Vec::new();
                    f.read_to_end(&mut buffer)?;
                    zip.start_file(jar, FileOptions::default())?;
                    zip.write_all(&buffer)?;
                    count += 1;
                }
            }
        }
    }
    zip.finish()?;
    Ok(count)
}

// Delete all jar files whose modules are tagged with `tag`. Returns number deleted.
fn delete_files_with_tag(
    dir: &str,
    jar_to_modid: &BTreeMap<String, String>,
    module: &Module,
    tag: DefaultTags,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut count = 0usize;
    for (jar, modid) in jar_to_modid {
        if let Some(mod_entry) = module.mods.get(modid) {
            if mod_entry.mod_tag == tag {
                let path = Path::new(dir).join(jar);
                if path.is_file() {
                    fs::remove_file(path)?;
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

// Write all original jar filenames (one per line) for mods with `tag` to `out_file`.
fn write_names_with_tag(
    dir: &str,
    jar_to_modid: &BTreeMap<String, String>,
    module: &Module,
    tag: DefaultTags,
    out_file: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    let mut file = fs::File::create(out_file)?;
    let mut count = 0usize;
    for (jar, modid) in jar_to_modid {
        if let Some(mod_entry) = module.mods.get(modid) {
            if mod_entry.mod_tag == tag {
                writeln!(file, "{}", jar)?;
                count += 1;
            }
        }
    }
    Ok(count)
}

// Move all jar files whose modules are tagged with `tag` to `dest_dir`. Returns the number moved.
fn move_files_with_tag(
    dir: &str,
    jar_to_modid: &BTreeMap<String, String>,
    module: &Module,
    tag: DefaultTags,
    dest_dir: &str,
) -> Result<usize, Box<dyn std::error::Error>> {
    fs::create_dir_all(dest_dir)?;
    let mut count = 0usize;
    for (jar, modid) in jar_to_modid {
        if let Some(mod_entry) = module.mods.get(modid) {
            if mod_entry.mod_tag == tag {
                let src = Path::new(dir).join(jar);
                let dst = Path::new(dest_dir).join(jar);
                if src.is_file() {
                    // Try and rename, fallback to copy+remove
                    match fs::rename(&src, &dst) {
                        Ok(_) => { count += 1; }
                        Err(_) => {
                            fs::copy(&src, &dst)?;
                            fs::remove_file(&src)?;
                            count += 1;
                        }
                    }
                }
            }
        }
    }
    Ok(count)
}

fn main() {
    // Choose which module JSON to load (default or other modules found in ./modules)
    let module_path = choose_module_file();

    // Load the module from the chosen path
    match Module::from_file(&module_path) {
        Ok(module) => {
            println!("Module '{}' successfully loaded.", module_path);
            let directory = input_str("Please navigate to the chosen directory:");
            // Get all JAR files from the chosen directory
            let results = get_jar_files(&directory);
            println!("The following JAR files were found in the chosen directory: ");
            // Unwrap and safely print all JAR files
            // Store tuples of (jar_filename, mod_id, detected_mod_type, detected_version)
            let mut mod_entries: Vec<(String, String, ModTypes, Option<String>)> = Vec::new();
             match results {
    // TESTING URL: /Users/morganbennett/Documents/curseforge/minecraft/Instances/testing/mods
                Ok(list) => {
                    for result in list {
                        // keep the original jar filename
                        let jar_name = result.clone();
                        let path = directory.clone() + "/" + &jar_name;
                        match get_mod_id_and_type(&path) {
                            Ok(Some((id, detected_type, detected_version))) => {
                                // store jar filename + detected metadata
                                mod_entries.push((jar_name.clone(), id, detected_type, detected_version));
                            }
                            Ok(None) => {
                                // No detectable metadata inside this jar -> treat as no match (silent)
                            }
                            Err(e) => {
                                eprintln!("Error reading {} : {:?}", jar_name, e);
                            }
                        }
                    }

                    println!("There were {} mods with identifiable metadata.", mod_entries.len());

                    // Build mapping from the original jar filename -> mod id
                    let mut jar_to_modid: BTreeMap<String, String> = BTreeMap::new();
                    for (jar, mod_id, _, _) in &mod_entries {
                        jar_to_modid.insert(jar.clone(), mod_id.clone());
                    }
                    println!("Jar -> ModID mapping ({} entries):", jar_to_modid.len());
                    for (jar, mod_id) in &jar_to_modid {
                        println!("  {} -> {}", jar, mod_id);
                    }

                    // ---------- Interactive operations by tag ----------
                    println!("\nOperations available for tagged mods:");
                    println!("  1) Zip all files with a tag");
                    println!("  2) Delete all files with a tag");
                    println!("  3) Write filenames of files with a tag to a text file");
                    println!("  4) Move all files with a tag to another directory");
                    println!("  0) Skip");

                    let choice = input_num("Select operation number:");
                    if choice != 0 {
                        let tag_input = input_str("Enter tag (Client, Server, Both, Unknown):");
                        let tag = parse_default_tag(tag_input.trim());
                        match choice {
                            1 => {
                                let out_zip = input_str("Enter output zip filename (e.g. selected.zip):");
                                match zip_files_with_tag(&directory, &jar_to_modid, &module, tag, &out_zip) {
                                    Ok(n) => println!("Zipped {} files to {}", n, out_zip),
                                    Err(e) => eprintln!("Zip error: {}", e),
                                }
                            }
                            2 => {
                                let confirm = input_str("Delete matched files from disk? Type YES to confirm:");
                                if confirm == "YES" {
                                    match delete_files_with_tag(&directory, &jar_to_modid, &module, tag) {
                                        Ok(n) => println!("Deleted {} files.", n),
                                        Err(e) => eprintln!("Delete error: {}", e),
                                    }
                                } else {
                                    println!("Delete cancelled.");
                                }
                            }
                            3 => {
                                let out_file = input_str("Enter output filename for names (e.g. names.txt):");
                                match write_names_with_tag(&directory, &jar_to_modid, &module, tag, &out_file) {
                                    Ok(n) => println!("Wrote {} names to {}", n, out_file),
                                    Err(e) => eprintln!("Write error: {}", e),
                                }
                            }
                            4 => {
                                let dest = input_str("Enter destination directory:");
                                match move_files_with_tag(&directory, &jar_to_modid, &module, tag, &dest) {
                                    Ok(n) => println!("Moved {} files to {}", n, dest),
                                    Err(e) => eprintln!("Move error: {}", e),
                                }
                            }
                            _ => println!("Unknown selection."),
                        }
                    }

                    // Only print full matches (id present in module, detected_version Some and equals, and type equals)
                    let mut match_count = 0;

                    // iterate to find full matches
                    for (jar, id, detected_type, detected_version) in &mod_entries {
                        if let Some(mod_struct) = module.mods.get(id) {
                            if let Some(v) = detected_version {
                                // compare strings (module.Mod.mod_version is now String) and enum equality for type
                                if v == &mod_struct.mod_version && detected_type == &mod_struct.mod_type {
                                    match_count += 1;
                                    println!(
                                        "FULL MATCH: JAR: {} | MOD ID: {} | SIDE: {:?} | MODULE TYPE: {:?} | DETECTED TYPE: {:?} | VERSION: {}",
                                        jar, id, mod_struct.mod_tag, mod_struct.mod_type, detected_type, v
                                    );
                                }
                            }
                            // if detected_version is None or types/versions don't match, silently skip (no output)
                        }
                        // if mod not in module, silently skip (no output)
                    }

                     if match_count == 0 {
                         println!("No full matches found.");
                     } else {
                         println!("{} full matches found.", match_count);
                     }

                 }
                 Err(e) => {
                     eprintln!("Error: {:?}", e)
                 }
             }
            // // Extracts all ModIDs from Module
            //

        }
        Err(e) => {
            eprintln!("Error loading module '{}': {}", module_path, e);
        }
    }
}
