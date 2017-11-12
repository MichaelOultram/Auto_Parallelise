#![feature(plugin_registrar, rustc_private, slice_patterns)]

#[macro_use] extern crate serde_derive;
extern crate serde_json;

#[macro_use] extern crate rustc;
extern crate syntax;
extern crate syntax_pos;
extern crate rustc_plugin;

use rustc_plugin::Registry;
use syntax::ext::base::SyntaxExtension::{MultiModifier};
use syntax::symbol::Symbol;

use std::fs::{self, File};
use std::io::prelude::*;
use std::path::Path;

mod linter;
mod syntax_extension;
mod shared_state;
use shared_state::*;

static SAVE_FILE: &'static str = ".auto-parallelize";

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    // Try to load AutoParallelize
    let obj = AutoParallelize::load();
    let stage = match obj.compiler_stage {
        CompilerStage::Analysis => 1,
        CompilerStage::Modification => 2,
    };
    println!("[auto-parallelize] Stage {} of 2 - {:?}", stage, obj.compiler_stage);

    // Second pass uses the syntax extension
    reg.register_syntax_extension(Symbol::intern("auto_parallelize"), MultiModifier(Box::new(obj.clone())));

    // First pass uses the linter
    reg.register_early_lint_pass(Box::new(obj));
}

#[derive(Clone, Serialize, Deserialize)]
pub struct AutoParallelize {
    compiler_stage: CompilerStage,
    linter_level: u32, // Used to determine when linter has finished
    parellelized_functions: Vec<Function>,
}

impl AutoParallelize {
    fn new() -> Self {
        AutoParallelize {
            compiler_stage: CompilerStage::Analysis,
            linter_level: 0,
            parellelized_functions: vec![],
        }
    }

    pub fn load() -> Self {
        // TODO: Remove this line to enable modifications
        return AutoParallelize::new();

        // Attempt to open .auto-parallise file
        let path = Path::new(SAVE_FILE);
        let maybe_file = File::open(&path);

        // If the file cannot be open, this is a new run
        if let Err(_) = maybe_file {
            return AutoParallelize::new()
        }
        let mut file = maybe_file.unwrap();

        // Read the file contents into a string
        let mut s = String::new();
        if let Err(why) = file.read_to_string(&mut s) {
            panic!("Failed to open {}: {}", path.display(), why)
        }

        // Try to convert it the string to an AutoParallelize object
        let mut obj : AutoParallelize = match serde_json::from_str(&s) {
            Ok(obj) => obj,
            Err(why) => panic!("Failed to read {}: {}", path.display(), why),
        };

        obj.linter_level = 0;

        if obj.compiler_stage == CompilerStage::Analysis {
            // Last stage was Analysis, this stage should parallelise
            obj.compiler_stage = CompilerStage::Modification;
            obj
        } else {
            // Last stage was Modification, so we need to start from scratch
            AutoParallelize::new()
        }
    }

    pub fn save(&self) {
        let path = Path::new(SAVE_FILE);

        // Try to convert the object to json
        let obj_json = match serde_json::to_string(&self) {
            Ok(obj) => obj,
            Err(why) => panic!("Unable to convert AutoParellize to JSON: {}", why),
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
    }

    pub fn delete(&self) {
        match fs::remove_file(SAVE_FILE) {
            Ok(_) => {},
            Err(why) => panic!("Failed to delete {}: {}", SAVE_FILE, why),
        }
    }
}