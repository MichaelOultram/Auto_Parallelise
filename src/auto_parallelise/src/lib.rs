#![feature(quote, plugin_registrar, rustc_private, use_extern_macros)]
extern crate serde;
#[macro_use] extern crate serde_derive;
extern crate serde_json;

#[macro_use] extern crate rustc;
extern crate syntax;
extern crate syntax_pos;
extern crate serialize;
extern crate rustc_plugin;

use rustc_plugin::Registry;
use syntax::ext::base::SyntaxExtension::{MultiModifier};
use syntax::symbol::Symbol;

use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;

#[macro_use] mod utils;
mod parallel_stages;
mod plugin;
mod rendering;
mod tests;

use plugin::shared_state::*;

static SAVE_FILE: &'static str = ".autoparallelise";
static CONFIG_FILE: &'static str = "autoparallelise.config";

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    // Try to load AutoParallelise
    let mut obj = AutoParallelise::load();
    let stage = match obj.compiler_stage {
        CompilerStage::Analysis => 1,
        CompilerStage::Modification => 2,
    };

    if obj.config.enabled {
        obj.config.enabled = reg.args().len() == 0;
    }
    eprintln!("[auto_parallelise] Stage {} of 2 - {:?}", stage, obj.compiler_stage);
    if !obj.config.enabled {
        eprintln!("[auto_parallelise] Plugin Disabled")
    }
    // Second pass uses the syntax extension
    reg.register_syntax_extension(Symbol::intern("autoparallelise"), MultiModifier(Box::new(obj.clone())));

    // First pass uses the linter
    reg.register_early_lint_pass(Box::new(obj));
}

fn read_file(filename: &str) -> Option<String> {
    // Attempt to open .auto-parallise file
    let path = Path::new(filename);
    let maybe_file = File::open(&path);

    // If the file cannot be open, this is a new run
    if let Err(_) = maybe_file {
        return None;
    }
    let mut file = maybe_file.unwrap();

    // Read the file contents into a string
    let mut s = String::new();
    if let Err(why) = file.read_to_string(&mut s) {
        //panic!("Failed to open {}: {}", path.display(), why)
        return None;
    }
    return Some(s);
}

impl AutoParallelise {
    fn new() -> Self {
        AutoParallelise {
            compiler_stage: CompilerStage::Analysis,
            linter_level: 0,
            functions: vec![],
            config: Config::default(),
        }
    }

    pub fn load() -> Self {
        let mconfig = read_file(CONFIG_FILE);
        let mobj = read_file(SAVE_FILE);

        // Extract config if it exists otherwise use default
        let config = match mconfig {
            Some(ref json) => match serde_json::from_str(json) {
                Ok(config) => config,
                Err(why) => panic!("Unable to parse {} as json: {}", CONFIG_FILE, why),
            },
            None => Config::default(),
        };

        // Try to convert it the string to an AutoParallelise object
        let mut obj : AutoParallelise = match mobj {
            Some(ref json) => match serde_json::from_str(json) {
                Ok(obj) => obj,
                Err(why) => panic!("Unable to parse {} as json: {}", SAVE_FILE, why),
            },
            None => AutoParallelise::new(),
        };

        obj.config = config;
        obj.linter_level = 0;
        obj
    }

    pub fn save(&mut self) {
        // Save it so that modification happens next
        let stage = self.compiler_stage;
        self.compiler_stage = CompilerStage::Modification;

        let path = Path::new(SAVE_FILE);

        // Try to convert the object to json
        let obj_json = match serde_json::to_string_pretty(&self) {
            Ok(obj) => obj,
            Err(why) => panic!("Unable to convert AutoParallelise to JSON: {}", why),
        };

        // Open the file in write-only mode
        let mut file = match File::create(&path) {
            Err(why) => panic!("Failed to open {}: {}", path.display(), why),
            Ok(file) => file,
        };

        // Write obj_json into the file
        if let Err(why) = file.write_all(obj_json.as_bytes()) {
            panic!("Failed to write {}: {}", path.display(), why);
        }
        self.compiler_stage = stage;
    }

    pub fn delete(&self) {
        match fs::remove_file(SAVE_FILE) {
            Ok(_) => {},
            Err(why) => panic!("Failed to delete {}: {}", SAVE_FILE, why),
        }
    }
}
