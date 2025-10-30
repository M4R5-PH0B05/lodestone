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

#[derive(Debug, Clone, Serialize, Deserialize)]
enum DefaultTags {
    Client,
    Server,
    Both,
    Unknown
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Mod {
    mod_id: String,
    mod_version: f64,
    mod_type: DefaultTags
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
    mods: Vec<Mod>
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

impl Module {
    fn from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let json_string = fs::read_to_string(path)?;
        let json_data: ModuleJson = serde_json::from_str(&json_string)?;
        // Convert ModuleJson to Module
        Ok(Self::from_json(json_data))
    }
    fn from_json(json_data: ModuleJson) -> Self {
        let mods: BTreeMap<String, Mod> = json_data.mods
            .into_iter()
            .map(|m| (m.mod_id.clone(), m))
            .collect();

        Self {
            module_name: json_data.header.module_name,
            module_version: json_data.header.module_version,
            module_author: json_data.header.module_author,
            mods,
        }
    }

    // Get a mod by its ID
    fn get_mod_type(&self,mod_id: &str) -> Option<&DefaultTags> {
        self.mods.get(mod_id).map(|m| &m.mod_type)
    }

    // Get all mods with a certain Tag
    fn get_mods_by_type(&self, tag: &DefaultTags) -> Vec<&Mod> {
        self.mods
            .values()
            .filter(|m| matches!(&m.mod_type, t if std::mem::discriminant(t) == std::mem::discriminant(tag)))
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
            println!("  {} v{} - {:?}", mod_id, mod_entry.mod_version, mod_entry.mod_type);
        }
    }
}


fn main() {
    // Load the module from test.json
    match Module::from_file("test.json") {
        Ok(module) => {
            // // Print module information
            // module.print_info();
            //
            // println!("\n--- Testing Lookups ---");
            //
            // // Test individual mod lookup
            // if let Some(tag) = module.get_mod_type("mod2") {
            //     println!("mod2 is: {:?}", tag);
            // }
            //
            // // Get all client-side mods
            // let client_mods = module.get_mods_by_type(&DefaultTags::Client);
            // println!("\nClient-side mods:");
            // for mod_entry in client_mods {
            //     println!("{}", mod_entry.mod_id);
            // }
            // readDir();
            let results = get_jar_files("/Users/morganbennett/Downloads");
            for result in results {
                println!("{:?}",result);
            }
        }
        Err(e) => {
            eprintln!("Error loading module: {}", e);
        }
    }
}
