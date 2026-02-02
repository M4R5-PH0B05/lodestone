use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use iced::alignment;
use iced::theme::Theme;
use iced::widget::{
    button, column, container, pick_list, row, scrollable, text, text_input, Space,
};
use iced::{Color, Element, Font, Length, Settings, Size, Task};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum DefaultTags {
    Unknown,
    Client,
    Server,
    Both,
}

impl std::fmt::Display for DefaultTags {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DefaultTags::Unknown => write!(f, "Unknown"),
            DefaultTags::Client => write!(f, "Client"),
            DefaultTags::Server => write!(f, "Server"),
            DefaultTags::Both => write!(f, "Both"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
enum ModTypes {
    Unknown,
    Forge,
    NeoForge,
    Fabric,
    Quilt,
}

impl std::fmt::Display for ModTypes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ModTypes::Unknown => write!(f, "Unknown"),
            ModTypes::Forge => write!(f, "Forge"),
            ModTypes::NeoForge => write!(f, "NeoForge"),
            ModTypes::Fabric => write!(f, "Fabric"),
            ModTypes::Quilt => write!(f, "Quilt"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Mod {
    mod_version: String,
    mod_tag: DefaultTags,
    mod_type: ModTypes,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModuleHeader {
    module_name: String,
    module_version: f64,
    module_author: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct ModuleJson {
    header: ModuleHeader,
    mods: BTreeMap<String, Mod>,
}

#[derive(Debug)]
struct Module {
    module_name: String,
    module_version: f64,
    module_author: String,
    mods: BTreeMap<String, Mod>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Operation {
    Zip,
    Delete,
    WriteNames,
    Move,
}

impl std::fmt::Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Operation::Zip => write!(f, "Zip"),
            Operation::Delete => write!(f, "Delete"),
            Operation::WriteNames => write!(f, "Write Names"),
            Operation::Move => write!(f, "Move"),
        }
    }
}

#[derive(Debug, Clone)]
struct ScanResult {
    jar_name: String,
    mod_id: String,
    detected_type: ModTypes,
    detected_version: Option<String>,
    module_tag: Option<DefaultTags>,
    module_type: Option<ModTypes>,
    module_version: Option<String>,
    full_match: bool,
}

#[derive(Debug, Clone, Default)]
struct ScanSummary {
    jar_count: usize,
    identified_count: usize,
    full_match_count: usize,
}

#[derive(Debug, Clone)]
enum Message {
    RefreshModules,
    ModuleSelected(String),
    LoadModule,
    DirectoryChanged(String),
    BrowseDirectory,
    ScanDirectory,
    TagSelected(DefaultTags),
    OperationSelected(Operation),
    OutputChanged(String),
    RunOperation,
}

struct LodestoneApp {
    modules: Vec<String>,
    selected_module: Option<String>,
    module: Option<Module>,
    directory: String,
    tag: DefaultTags,
    operation: Operation,
    output_path: String,
    scan_results: Vec<ScanResult>,
    summary: ScanSummary,
    jar_to_modid: BTreeMap<String, String>,
    log: Vec<String>,
}

impl Default for LodestoneApp {
    fn default() -> Self {
        let modules = load_module_list();
        let selected_module = modules.first().cloned();
        Self {
            modules,
            selected_module,
            module: None,
            directory: String::new(),
            tag: DefaultTags::Client,
            operation: Operation::Zip,
            output_path: String::new(),
            scan_results: Vec::new(),
            summary: ScanSummary::default(),
            jar_to_modid: BTreeMap::new(),
            log: vec!["Welcome to Lodestone.".to_string()],
        }
    }
}

fn update(state: &mut LodestoneApp, message: Message) -> Task<Message> {
    match message {
        Message::RefreshModules => {
            state.modules = load_module_list();
            if let Some(selected) = &state.selected_module {
                if !state.modules.contains(selected) {
                    state.selected_module = state.modules.first().cloned();
                }
            } else {
                state.selected_module = state.modules.first().cloned();
            }
            push_log(&mut state.log, "Module list refreshed.".to_string());
        }
        Message::ModuleSelected(path) => {
            state.selected_module = Some(path);
        }
        Message::LoadModule => {
            if let Some(path) = &state.selected_module {
                match Module::from_file(path) {
                    Ok(module) => {
                        state.module = Some(module);
                        state.scan_results.clear();
                        state.summary = ScanSummary::default();
                        state.jar_to_modid.clear();
                        push_log(&mut state.log, format!("Loaded module: {}", path));
                    }
                    Err(e) => {
                        state.module = None;
                        push_log(&mut state.log, format!("Failed to load module {}: {}", path, e));
                    }
                }
            } else {
                push_log(&mut state.log, "Select a module before loading.".to_string());
            }
        }
        Message::DirectoryChanged(value) => {
            state.directory = value;
        }
        Message::BrowseDirectory => {
            if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                state.directory = folder.display().to_string();
                push_log(
                    &mut state.log,
                    format!("Selected directory: {}", state.directory),
                );
            }
        }
        Message::ScanDirectory => {
            if state.module.is_none() {
                push_log(&mut state.log, "Load a module before scanning.".to_string());
                return Task::none();
            }
            if state.directory.trim().is_empty() {
                push_log(
                    &mut state.log,
                    "Enter a directory path before scanning.".to_string(),
                );
                return Task::none();
            }

            match scan_directory(&state.directory, state.module.as_ref().unwrap()) {
                Ok((results, summary, jar_to_modid)) => {
                    state.scan_results = results;
                    state.summary = summary;
                    state.jar_to_modid = jar_to_modid;
                    push_log(
                        &mut state.log,
                        format!(
                        "Scan complete: {} jars, {} identified, {} full matches.",
                        state.summary.jar_count,
                        state.summary.identified_count,
                        state.summary.full_match_count
                    ),
                    );
                }
                Err(e) => {
                    push_log(&mut state.log, format!("Scan failed: {}", e));
                }
            }
        }
        Message::TagSelected(tag) => {
            state.tag = tag;
        }
        Message::OperationSelected(operation) => {
            state.operation = operation;
            state.output_path.clear();
        }
        Message::OutputChanged(value) => {
            state.output_path = value;
        }
        Message::RunOperation => {
            if state.module.is_none() {
                push_log(
                    &mut state.log,
                    "Load a module before running an operation.".to_string(),
                );
                return Task::none();
            }
            if state.jar_to_modid.is_empty() {
                push_log(
                    &mut state.log,
                    "Scan a directory before running an operation.".to_string(),
                );
                return Task::none();
            }

            let module = state.module.as_ref().unwrap();
            let tag = state.tag;
            let dir = state.directory.trim();

            match state.operation {
                Operation::Zip => {
                    if state.output_path.trim().is_empty() {
                        push_log(
                            &mut state.log,
                            "Provide an output zip filename.".to_string(),
                        );
                        return Task::none();
                    }
                    match zip_files_with_tag(
                        dir,
                        &state.jar_to_modid,
                        module,
                        tag,
                        state.output_path.trim(),
                    ) {
                        Ok(n) => push_log(&mut state.log, format!("Zipped {} files.", n)),
                        Err(e) => push_log(&mut state.log, format!("Zip error: {}", e)),
                    }
                }
                Operation::Delete => {
                    if state.output_path.trim() != "DELETE" {
                        push_log(
                            &mut state.log,
                            "Type DELETE in the confirmation box to remove files.".to_string(),
                        );
                        return Task::none();
                    }
                    match delete_files_with_tag(dir, &state.jar_to_modid, module, tag) {
                        Ok(n) => push_log(&mut state.log, format!("Deleted {} files.", n)),
                        Err(e) => push_log(&mut state.log, format!("Delete error: {}", e)),
                    }
                }
                Operation::WriteNames => {
                    if state.output_path.trim().is_empty() {
                        push_log(&mut state.log, "Provide an output filename.".to_string());
                        return Task::none();
                    }
                    match write_names_with_tag(
                        dir,
                        &state.jar_to_modid,
                        module,
                        tag,
                        state.output_path.trim(),
                    ) {
                        Ok(n) => push_log(&mut state.log, format!("Wrote {} names.", n)),
                        Err(e) => push_log(&mut state.log, format!("Write error: {}", e)),
                    }
                }
                Operation::Move => {
                    if state.output_path.trim().is_empty() {
                        push_log(
                            &mut state.log,
                            "Provide a destination directory.".to_string(),
                        );
                        return Task::none();
                    }
                    match move_files_with_tag(
                        dir,
                        &state.jar_to_modid,
                        module,
                        tag,
                        state.output_path.trim(),
                    ) {
                        Ok(n) => push_log(&mut state.log, format!("Moved {} files.", n)),
                        Err(e) => push_log(&mut state.log, format!("Move error: {}", e)),
                    }
                }
            }
        }
    }

    if state.log.len() > 200 {
        state.log.drain(0..state.log.len() - 200);
    }

    Task::none()
}

fn push_log(log: &mut Vec<String>, entry: String) {
    if log.last().map(|last| last == &entry).unwrap_or(false) {
        return;
    }
    log.push(entry);
    if log.len() > 200 {
        log.drain(0..log.len() - 200);
    }
}

fn view(state: &LodestoneApp) -> Element<'_, Message> {
    let header = container(
        row![
            column![
                text("Lodestone")
                    .size(36)
                    .style(text_color(Color::from_rgb(0.18, 0.16, 0.14))),
                text("Mod intelligence for Minecraft installs")
                    .size(16)
                    .style(text_color(Color::from_rgb(0.46, 0.42, 0.38))),
            ]
            .spacing(6)
            .align_x(alignment::Horizontal::Left),
            Space::with_width(Length::Fill),
            container(
                text(format!(
                    "{} jars · {} identified · {} full matches",
                    state.summary.jar_count,
                    state.summary.identified_count,
                    state.summary.full_match_count
                ))
                .size(14)
                .style(text_color(Color::from_rgb(0.52, 0.49, 0.46))),
            )
            .padding(12)
            .style(|_| pill_style()),
        ]
        .align_y(alignment::Vertical::Center),
    )
    .padding(24)
    .style(|_| header_style());

    let module_picker = pick_list(
        state.modules.clone(),
        state.selected_module.clone(),
        Message::ModuleSelected,
    )
    .placeholder("Select a module");

    let setup_card = container(
        column![
            text("Module + Scan")
                .size(18)
                .style(text_color(Color::from_rgb(0.22, 0.2, 0.18))),
            text("Load a module file and scan a mods folder.")
                .size(13)
                .style(text_color(Color::from_rgb(0.5, 0.46, 0.42))),
            module_picker,
            row![
                button(text("Refresh")).style(|theme, status| secondary_button(theme, status)).on_press(Message::RefreshModules),
                button(text("Load")).style(|theme, status| primary_button(theme, status)).on_press(Message::LoadModule),
            ]
            .spacing(12),
            row![
                text_input("/path/to/mods", &state.directory)
                    .on_input(Message::DirectoryChanged)
                    .padding(12)
                    .size(14),
                button(text("Browse"))
                    .style(|theme, status| secondary_button(theme, status))
                    .on_press(Message::BrowseDirectory),
            ]
            .spacing(10),
            button(text("Scan Directory"))
                .style(|theme, status| primary_button(theme, status))
                .on_press(Message::ScanDirectory),
        ]
        .spacing(14),
    )
    .padding(20)
    .style(|_| card_style());

    let tag_picker = pick_list(
        vec![
            DefaultTags::Client,
            DefaultTags::Server,
            DefaultTags::Both,
            DefaultTags::Unknown,
        ],
        Some(state.tag),
        Message::TagSelected,
    )
    .placeholder("Tag");

    let operation_picker = pick_list(
        vec![
            Operation::Zip,
            Operation::Delete,
            Operation::WriteNames,
            Operation::Move,
        ],
        Some(state.operation),
        Message::OperationSelected,
    )
    .placeholder("Operation");

    let output_label = match state.operation {
        Operation::Zip => "Output zip filename",
        Operation::Delete => "Type DELETE to confirm",
        Operation::WriteNames => "Output text filename",
        Operation::Move => "Destination directory",
    };

    let operations_card = container(
        column![
            text("Operations")
                .size(18)
                .style(text_color(Color::from_rgb(0.22, 0.2, 0.18))),
            text("Run actions on tagged mods.")
                .size(13)
                .style(text_color(Color::from_rgb(0.5, 0.46, 0.42))),
            row![tag_picker, operation_picker].spacing(12),
            text_input(output_label, &state.output_path)
                .on_input(Message::OutputChanged)
                .padding(12)
                .size(14),
            button(text("Run Operation"))
                .style(|theme, status| match state.operation {
                    Operation::Delete => danger_button(theme, status),
                    _ => primary_button(theme, status),
                })
                .on_press(Message::RunOperation),
        ]
        .spacing(14),
    )
    .padding(20)
    .style(|_| card_style());

    let left_panel = column![setup_card, operations_card]
        .spacing(18)
        .width(Length::FillPortion(2));

    let result_list = if state.scan_results.is_empty() {
        container(
            column![
                text("No scan results yet.")
                    .size(16)
                    .style(text_color(Color::from_rgb(0.46, 0.43, 0.4))),
                text("Load a module and scan a directory to populate results.")
                    .size(13)
                    .style(text_color(Color::from_rgb(0.56, 0.52, 0.48))),
            ]
            .spacing(8)
            .align_x(alignment::Horizontal::Center),
        )
        .padding(24)
        .style(|_| soft_card_style())
    } else {
        let mut list = column![].spacing(12);
        for result in &state.scan_results {
            let status_color = if result.full_match {
                Color::from_rgb(0.2, 0.6, 0.44)
            } else {
                Color::from_rgb(0.78, 0.42, 0.22)
            };

            let module_tag = result
                .module_tag
                .map(|t| t.to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let module_type = result
                .module_type
                .map(|t| t.to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            let module_version = result
                .module_version
                .clone()
                .unwrap_or_else(|| "-".to_string());

            let detected_version = result
                .detected_version
                .clone()
                .unwrap_or_else(|| "-".to_string());

            list = list.push(
                container(
                    column![
                        row![
                            text(&result.jar_name)
                                .size(16)
                                .style(text_color(Color::from_rgb(0.2, 0.18, 0.16))),
                            Space::with_width(Length::Fill),
                            text(if result.full_match { "Full match" } else { "Partial" })
                                .size(12)
                                .style(text_color(status_color)),
                        ]
                        .align_y(alignment::Vertical::Center),
                        text(format!("Mod ID: {}", result.mod_id))
                            .size(13)
                            .style(text_color(Color::from_rgb(0.48, 0.44, 0.4))),
                        row![
                            text(format!(
                                "Detected: {} v{}",
                                result.detected_type, detected_version
                            ))
                            .size(12)
                            .style(text_color(Color::from_rgb(0.52, 0.48, 0.44))),
                            Space::with_width(Length::Fill),
                            text(format!(
                                "Module: {} v{} · {}",
                                module_type, module_version, module_tag
                            ))
                            .size(12)
                            .style(text_color(Color::from_rgb(0.52, 0.48, 0.44))),
                        ],
                    ]
                    .spacing(6),
                )
                .padding(16)
                .style(|_| soft_card_style()),
            );
        }

        container(scrollable(list).height(Length::Fill))
            .padding(4)
            .style(|_| soft_card_style())
    };

    let log_list = {
        let mut list = column![].spacing(6);
        for entry in state.log.iter().rev().take(6) {
            list = list.push(
                text(entry)
                    .size(12)
                    .style(text_color(Color::from_rgb(0.5, 0.46, 0.42))),
            );
        }
        container(
            column![
                text("Activity")
                    .size(14)
                    .style(text_color(Color::from_rgb(0.36, 0.32, 0.28))),
                list,
            ]
            .spacing(8),
        )
        .padding(14)
        .style(|_| soft_card_style())
    };

    let right_panel = column![
        row![
            text("Results")
                .size(18)
                .style(text_color(Color::from_rgb(0.22, 0.2, 0.18))),
            Space::with_width(Length::Fill),
        ]
        .align_y(alignment::Vertical::Center),
        result_list,
        log_list,
    ]
    .spacing(16)
    .width(Length::FillPortion(3));

    let content = row![left_panel, right_panel]
        .spacing(20)
        .align_y(alignment::Vertical::Top)
        .padding(24);

    container(column![header, content])
        .style(|_| background_style())
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn main() -> iced::Result {
    iced::application("Lodestone", update, view)
        .theme(|_| Theme::Light)
        .default_font(Font::with_name("Fira Sans"))
        .window(iced::window::Settings {
            size: Size::new(1200.0, 780.0),
            min_size: Some(Size::new(980.0, 680.0)),
            ..Default::default()
        })
        .settings(Settings {
            antialiasing: true,
            ..Default::default()
        })
        .run_with(|| (LodestoneApp::default(), Task::none()))
}

impl Module {
    fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let json_string = fs::read_to_string(path)?;
        let json_data: ModuleJson = serde_json::from_str(&json_string)?;
        Ok(Self::from_json(json_data))
    }

    fn from_json(json_data: ModuleJson) -> Self {
        Self {
            module_name: json_data.header.module_name,
            module_version: json_data.header.module_version,
            module_author: json_data.header.module_author,
            mods: json_data.mods,
        }
    }
}

fn load_module_list() -> Vec<String> {
    let mut modules = Vec::new();
    if Path::new("test.json").exists() {
        modules.push("test.json".to_string());
    }

    if let Ok(entries) = fs::read_dir("modules") {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Some(fname) = path.file_name().and_then(|s| s.to_str()) {
                    modules.push(format!("modules/{}", fname));
                }
            }
        }
    }

    if modules.is_empty() {
        modules.push("test.json".to_string());
    }

    modules
}

fn scan_directory(
    directory: &str,
    module: &Module,
) -> Result<(Vec<ScanResult>, ScanSummary, BTreeMap<String, String>), Box<dyn std::error::Error>> {
    let jar_list = get_jar_files(directory)?;
    let mut scan_results = Vec::new();
    let mut jar_to_modid = BTreeMap::new();

    for jar_name in jar_list.iter() {
        let path = format!("{}/{}", directory, jar_name);
        if let Some((mod_id, detected_type, detected_version)) = get_mod_id_and_type(&path)? {
            jar_to_modid.insert(jar_name.clone(), mod_id.clone());

            let (module_tag, module_type, module_version, full_match) = if let Some(mod_entry) =
                module.mods.get(&mod_id)
            {
                let module_version = mod_entry.mod_version.clone();
                let full_match = detected_version
                    .as_ref()
                    .map(|v| v == &module_version && detected_type == mod_entry.mod_type)
                    .unwrap_or(false);
                (
                    Some(mod_entry.mod_tag),
                    Some(mod_entry.mod_type),
                    Some(module_version),
                    full_match,
                )
            } else {
                (None, None, None, false)
            };

            scan_results.push(ScanResult {
                jar_name: jar_name.clone(),
                mod_id,
                detected_type,
                detected_version,
                module_tag,
                module_type,
                module_version,
                full_match,
            });
        }
    }

    let summary = ScanSummary {
        jar_count: jar_list.len(),
        identified_count: scan_results.len(),
        full_match_count: scan_results.iter().filter(|r| r.full_match).count(),
    };

    Ok((scan_results, summary, jar_to_modid))
}

fn get_jar_files(dir_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut jar_files = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("jar") {
            if let Some(file_name) = path.file_name() {
                jar_files.push(file_name.to_string_lossy().to_string());
            }
        }
    }

    Ok(jar_files)
}

fn get_mod_id_and_type(
    path: &str,
) -> Result<Option<(String, ModTypes, Option<String>)>, Box<dyn std::error::Error>> {
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

    let file = fs::File::open(path)?;
    let mut archive = zip::ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name();

        if name.ends_with("mods.toml") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
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
                .and_then(|mod_entry| mod_entry.get("version").or_else(|| mod_entry.get("modVersion")))
                .and_then(parse_toml_version);
            return Ok(mod_id.map(|id| (id, found_type, detected_version)));
        } else if name.ends_with("fabric.mod.json") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            let parsed: serde_json::Value = serde_json::from_str(&contents)?;
            let mod_id = parsed
                .get("id")
                .and_then(|v| v.as_str())
                .map(String::from);
            let detected_version = parsed.get("version").and_then(parse_json_version);
            return Ok(mod_id.map(|id| (id, ModTypes::Fabric, detected_version)));
        } else if name.ends_with("mcmod.info") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
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
                .and_then(parse_json_version);
            return Ok(mod_id.map(|id| (id, ModTypes::Forge, detected_version)));
        }
    }

    Ok(None)
}

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
                let path = Path::new(dir).join(jar);
                if path.is_file() {
                    writeln!(file, "{}", jar)?;
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

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
                    match fs::rename(&src, &dst) {
                        Ok(_) => count += 1,
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

fn background_style() -> container::Style {
    container::Style {
        background: Some(Color::from_rgb(0.98, 0.965, 0.94).into()),
        text_color: Some(Color::from_rgb(0.2, 0.18, 0.16)),
        ..Default::default()
    }
}

fn header_style() -> container::Style {
    container::Style {
        background: Some(Color::from_rgb(0.985, 0.975, 0.96).into()),
        border: iced::border::Border {
            color: Color::from_rgb(0.86, 0.82, 0.78),
            width: 1.2,
            radius: 18.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgb(0.65, 0.58, 0.52),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

fn card_style() -> container::Style {
    container::Style {
        background: Some(Color::from_rgb(0.995, 0.985, 0.97).into()),
        border: iced::border::Border {
            color: Color::from_rgb(0.88, 0.84, 0.8),
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgb(0.7, 0.64, 0.58),
            offset: iced::Vector::new(0.0, 8.0),
            blur_radius: 22.0,
        },
        ..Default::default()
    }
}

fn soft_card_style() -> container::Style {
    container::Style {
        background: Some(Color::from_rgb(0.99, 0.98, 0.96).into()),
        border: iced::border::Border {
            color: Color::from_rgb(0.9, 0.86, 0.82),
            width: 1.0,
            radius: 14.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgb(0.74, 0.68, 0.6),
            offset: iced::Vector::new(0.0, 6.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

fn pill_style() -> container::Style {
    container::Style {
        background: Some(Color::from_rgb(0.95, 0.92, 0.88).into()),
        border: iced::border::Border {
            color: Color::from_rgb(0.86, 0.8, 0.74),
            width: 1.0,
            radius: 999.0.into(),
        },
        ..Default::default()
    }
}

fn text_color(color: Color) -> impl Fn(&Theme) -> iced::widget::text::Style {
    move |_| iced::widget::text::Style {
        color: Some(color),
    }
}

fn primary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Color::from_rgb(0.78, 0.5, 0.3).into()),
        text_color: Color::from_rgb(0.99, 0.98, 0.96),
        border: iced::border::Border {
            color: Color::from_rgb(0.78, 0.5, 0.3),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgb(0.6, 0.4, 0.28),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Color::from_rgb(0.84, 0.56, 0.34).into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Color::from_rgb(0.7, 0.44, 0.26).into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Color::from_rgb(0.88, 0.82, 0.78).into()),
            text_color: Color::from_rgb(0.66, 0.62, 0.58),
            ..base
        },
        button::Status::Active => base,
    }
}

fn secondary_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Color::from_rgb(0.94, 0.9, 0.86).into()),
        text_color: Color::from_rgb(0.34, 0.3, 0.26),
        border: iced::border::Border {
            color: Color::from_rgb(0.86, 0.8, 0.74),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgb(0.76, 0.7, 0.62),
            offset: iced::Vector::new(0.0, 3.0),
            blur_radius: 8.0,
        },
        ..Default::default()
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Color::from_rgb(0.96, 0.92, 0.88).into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Color::from_rgb(0.9, 0.86, 0.82).into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Color::from_rgb(0.92, 0.9, 0.88).into()),
            text_color: Color::from_rgb(0.64, 0.6, 0.56),
            ..base
        },
        button::Status::Active => base,
    }
}

fn danger_button(_theme: &Theme, status: button::Status) -> button::Style {
    let base = button::Style {
        background: Some(Color::from_rgb(0.82, 0.38, 0.32).into()),
        text_color: Color::from_rgb(0.98, 0.96, 0.95),
        border: iced::border::Border {
            color: Color::from_rgb(0.82, 0.38, 0.32),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: iced::Shadow {
            color: Color::from_rgb(0.66, 0.34, 0.28),
            offset: iced::Vector::new(0.0, 4.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    };

    match status {
        button::Status::Hovered => button::Style {
            background: Some(Color::from_rgb(0.88, 0.44, 0.36).into()),
            ..base
        },
        button::Status::Pressed => button::Style {
            background: Some(Color::from_rgb(0.7, 0.32, 0.28).into()),
            ..base
        },
        button::Status::Disabled => button::Style {
            background: Some(Color::from_rgb(0.9, 0.86, 0.84).into()),
            text_color: Color::from_rgb(0.7, 0.66, 0.64),
            ..base
        },
        button::Status::Active => base,
    }
}
