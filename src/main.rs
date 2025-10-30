// PROJECT: **LODESTONE**
// OUTLINE: A GUI-Based application which uses community driven selection in order to identify client and server
//          side based MC mods. It will use 'modules' that can be loaded that will flag certain mods with tags.
//          There will be a 'default' module, which will identify 'client' and 'server' side mods.
//          Custom modules could isolate dependencies for certain mods, mods for certain modpacks, common
//          incompatibilities between mods, etc...
//          All modules will be in JSON format.
//          The module header will contain a name, creator and version.
//          The content will be the mod id, e.g. 'create' and the type, e.g. 'client' or 'server'.
//          The application will have multiple functions, however the primary ones are:
//              - A 'remove all' button of a certain tag, for example to remove all client side mods.
//              - A way to tag mods yourself, if a mod is not recognised by the current module.
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
//                      - ModType: Enum. (e.g. 'client' )
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
// PROJECT FLOW: Disclosed in 'DRAWIO' diagram in root directory.


enum DefaultTags {
    Client,
    Server,
    Both,
    Unknown
}

struct Mod {
    mod_id: String,
    mod_version: f64,
    mod_type: DefaultTags
}

struct Module {
    module_name: String,
    module_version: f64,
    module_author: String,
    mods: Vec<Mod>
}

fn main() {
    println!("Hello, world!");
}
