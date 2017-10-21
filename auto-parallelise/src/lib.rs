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

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

mod linter;
mod syntax_extension;

static SAVE_FILE: &'static str = ".auto-parallelise";

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    // Try to load AutoParallelise
    let obj = AutoParallelise::load();
    println!("[auto-parallelise] Compiler plugin loaded");

    // Second pass uses the syntax extension
    reg.register_syntax_extension(Symbol::intern("auto_parallelise"), MultiModifier(Box::new(obj)));

    // First pass uses the linter
    reg.register_late_lint_pass(Box::new(obj));
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum CompilerStage {
    Analysis, // First Stage
    Modification, // Second Stage
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct AutoParallelise {
    compiler_stage: CompilerStage,
}

impl AutoParallelise {
    pub fn new() -> Self {
        AutoParallelise {
            compiler_stage: CompilerStage::Analysis,
        }
    }

    pub fn load() -> Self {
        // Attempt to open .auto-parallise file
        let path = Path::new(SAVE_FILE);
        let maybe_file = File::open(&path);

        // If the file cannot be open, this is a new run
        if let Err(_) = maybe_file {
            return AutoParallelise::new()
        }
        let mut file = maybe_file.unwrap();

        // Read the file contents into a string
        let mut s = String::new();
        if let Err(why) = file.read_to_string(&mut s) {
            panic!("Failed to open {}: {}", path.display(), why)
        }

        // Try to convert it the string to an AutoParallelise object
        let mut obj : AutoParallelise = match serde_json::from_str(&s) {
            Ok(obj) => obj,
            Err(why) => panic!("Failed to read {}: {}", path.display(), why),
        };

        if obj.compiler_stage == CompilerStage::Analysis {
            // Last stage was Analysis, this stage should parallelise
            obj.compiler_stage = CompilerStage::Modification;
            obj
        } else {
            // Last stage was Modification, so we need to start from scratch
            AutoParallelise::new()
        }
    }

    pub fn save(&self) {
        let path = Path::new(SAVE_FILE);

        // Try to convert the object to json
        let obj_json = match serde_json::to_string(&self) {
            Ok(obj) => obj,
            Err(why) => panic!("Unable to convert AutoParellise to JSON: {}", why),
        };

        // Open the file in write-only mode
        let mut file = match File::create(&path) {
            Err(why) => panic!("Failed to open {}: {}", path.display(), why),
            Ok(file) => file,
        };

        // Write obj_json into the file
        match file.write_all(obj_json.as_bytes()) {
            Err(why) => panic!("Failed to write {}: {}", path.display(), why),
            Ok(_) => (),
        }
    }
}
