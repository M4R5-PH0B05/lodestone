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

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DefaultTags {
    Unknown,
    Client,
    Server,
    Both
}
#[derive(Debug, Clone, Serialize, Deserialize)]
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
    mod_version: f64,
    mod_tag: DefaultTags,
    mod_type: ModTypes
}
#[derive(Debug, Deserialize)]
struct ModuleHeader {
    module_name: String,
    module_version: f64,
    module_author: String
}
#[derive(Debug, Deserialize)]
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
fn get_jar_files(dir_path: &str) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let mut jar_files = Vec::new();

    for entry in fs::read_dir(dir_path)? {
        let entry = entry?;
        let path = entry.path();

        // if path.extension().and_then(|s| s.to_str()) == Some("jar") {
            if let Some(file_name) = path.file_name() {
                jar_files.push(file_name.to_string_lossy().to_string());
            // }
        }
    }

    Ok(jar_files)
}

// Helper function to get mod ID and mod Type from the JAR file
fn get_mod_id_and_type(path: &str) -> Result<(), Box<dyn std::error::Error>>{
    // Open the file from the path
    let file = std::fs::File::open(path)?;
    // Unzip it (A jar file is basically a ZIP)
    let mut archive = zip::ZipArchive::new(file)?;
    // Iterate over the length and produce all file paths in the JAR directory
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let name = file.name();

        if name.ends_with("mods.toml") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            println!("Found Forge mods.toml: \n{}", contents);

        } else if name.ends_with("fabric.mod.json") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            println!("Found Fabric mods.json: \n{}", contents);

        } else if name.ends_with("mcmod.info") {
            let mut contents = String::new();
            file.read_to_string(&mut contents)?;
            println!("Found ? \n{}", contents);

        }
    }
    Ok(())
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
        self.mods
            .values()
            .filter(|m| matches!(&m.mod_tag, t if std::mem::discriminant(t) == std::mem::discriminant(tag)))
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


fn main() {
    // Load the module from test.json
    match Module::from_file("test.json") {
        Ok(module) => {
            println!("Default module successfully loaded.");
            let directory = input_str("Please navigate to the chosen directory:");
            // Get all JAR files from the chosen directory
            let results = get_jar_files(&directory);
            println!("The following JAR files were found in the chosen directory: ");
            // Unwrap and safely print all JAR files
            match results {
                Ok(list) => {
                    for result in list {
                        println!("{}",result);
                    }
                }
                Err(e) => {
                    eprintln!("Error: {:?}",e)
                }
            }
            // Extracts all ModIDs from Module
            for (key,value) in &module.mods {
                // Key - Mod ID, Value - ModStruct of ver and type
                println!("{} | {:?}",key,value);
            }

            let test_file = input_str("\nPlease enter a path to a FILE:");
            let results = get_mod_id_and_type(&test_file);
            println!("{:?}",results);

        }
        Err(e) => {
            eprintln!("Error loading module: {}", e);
        }
    }
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