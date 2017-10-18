#![feature(plugin_registrar, rustc_private, slice_patterns)]

#[macro_use] extern crate serde_derive;

#[macro_use] extern crate rustc;
extern crate syntax;
extern crate syntax_pos;
extern crate rustc_plugin;

use rustc_plugin::Registry;

use syntax::ext::base::SyntaxExtension::{MultiModifier};
use syntax::symbol::Symbol;

mod linter;
mod syntax_extension;


#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    // Try to load AutoParallelise
    let mut obj = AutoParallelise::new();
    println!("[auto-parallelise] Compiler plugin loaded");

    // Second pass uses the syntax extension
    reg.register_syntax_extension(Symbol::intern("auto_parallelise"), obj.gen_syntax_extension());

    // First pass uses the linter
    reg.register_late_lint_pass(Box::new(obj));
}

#[derive(Clone, Copy, Serialize, Deserialize)]
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
        // TODO: Check if a file exists
        AutoParallelise {
            compiler_stage: CompilerStage::Analysis,
        }
    }
}
