#![feature(plugin_registrar, rustc_private, slice_patterns)]

#[macro_use]
extern crate rustc;
extern crate syntax;
extern crate rustc_plugin;

use rustc_plugin::Registry;

use syntax::ext::base::SyntaxExtension::{MultiModifier};
use syntax::symbol::Symbol;

mod roman_numerals;
use roman_numerals::*;

mod syntax_extension;
use syntax_extension::*;

mod lint_extension;
use lint_extension::*;

#[plugin_registrar]
pub fn plugin_registrar(reg: &mut Registry) {
    reg.register_macro("rn", expand_rn);
    reg.register_syntax_extension(Symbol::intern("para"), MultiModifier(Box::new(example_extension)));
    reg.register_early_lint_pass(Box::new(LintExample{}));
    //reg.register_late_lint_pass(Box::new(LintExample{}));
}
