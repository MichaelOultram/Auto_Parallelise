use rustc::lint::{LintArray, LintPass, EarlyContext, EarlyLintPass};
use syntax::ast;
use syntax_pos::Span;
use syntax::visit::{self, FnKind};

use serde_json;

use AutoParallelise;
use CompilerStage;
use parallel_stages::dependency_analysis;
use plugin::shared_state::{Function};

impl LintPass for AutoParallelise {
    fn get_lints(&self) -> LintArray {
        lint_array!()
    }
}

impl EarlyLintPass for AutoParallelise {
    fn check_fn(&mut self, _context: &EarlyContext, _fnkind: visit::FnKind, _fndecl: &ast::FnDecl, _span: Span, _nodeid: ast::NodeId) {
        // Only need to analyse function during the analysis stage
        if !self.config.plugin_enabled || self.compiler_stage != CompilerStage::Analysis {
            self.save();
            return;
        }

        match _fnkind {
            // fn foo()
            FnKind::ItemFn(ident, _, _, _, _, block) |
            // fn foo(&self), i.e. obj.foo();
            FnKind::Method(ident, _, _, block) => {
                eprintln!("\n\n{:?}", _fndecl);
                let ident_name: String = ident.name.to_string();
                let ident_ctxt: String = format!("{:?}", ident.ctxt);
                let input_types = vec![]; // TODO
                for ref arg in &_fndecl.inputs {
                    eprintln!("ARG: {:?}, {:?}", arg.ty.node, arg.pat);
                }

                let deptree = dependency_analysis::analyse_block(&block);
                eprintln!("DEPTREE:");
                for node in &deptree {
                    let node_json = match serde_json::to_string_pretty(&node) {
                        Ok(obj) => obj,
                        Err(why) => panic!("Unable to convert deptree to JSON: {}", why),
                    };
                    eprintln!("{}", node_json);
                }

                // convert deptree into encoded_deptree
                let encoded_deptree = dependency_analysis::encode_deptree(&deptree);

                eprintln!("ENCODED_DEPTREE:");
                eprintln!("{:?}", encoded_deptree);

                self.functions.push(Function {
                    ident_name: ident_name,
                    ident_ctxt: ident_ctxt,

                    output_type: None,

                    is_unsafe: false, //TODO
                    called_functions: vec![], // TODO
                    input_types: input_types,

                    encoded_deptree: encoded_deptree,
                });
            },

            // |x, y| body
            FnKind::Closure(_body) => {}, //unimplemented!(),
        }
        self.save();
    }

    // Used to detect when the EarlyLintPass is over
    // TODO: Check this works for all programs
    fn enter_lint_attrs(&mut self, _: &EarlyContext, _: &[ast::Attribute]) {
        self.linter_level += 1;
    }
    fn exit_lint_attrs(&mut self, _: &EarlyContext, _: &[ast::Attribute]) {
        self.linter_level -= 1;
        if self.linter_level == 0 {
            self.save();
            match self.compiler_stage {
                CompilerStage::Analysis => {
                    eprintln!("[auto_parallelise] Recompile to apply parallelization modifications");
                    ::std::process::exit(1);
                },
                CompilerStage::Modification => {
                    eprintln!("[auto_parallelise] Parallelised Compilation Complete");
                    self.delete();
                    // Sometimes compile works, sometimes not. Instead always fail, and use script to copy to a new crate without auto_parallelise
                    // TODO: Remove when nightly compiler bug affecting macros is fixed
                    if self.config.plugin_enabled {
                        ::std::process::exit(1);
                    }
                },
            }
        }
    }
}
