use rustc::lint::{LintArray, LintPass, EarlyContext, EarlyLintPass};
use syntax::ast;
use syntax_pos::Span;
use syntax::visit::{self, FnKind};

use AutoParallelise;
use CompilerStage;
use dependency_analysis;
use shared_state::{Function};

impl LintPass for AutoParallelise {
    fn get_lints(&self) -> LintArray {
        lint_array!()
    }
}

impl EarlyLintPass for AutoParallelise {
    fn check_fn(&mut self, _context: &EarlyContext, _fnkind: visit::FnKind, _fndecl: &ast::FnDecl, _span: Span, _nodeid: ast::NodeId) {
        // Only need to analyse function during the analysis stage
        if self.compiler_stage != CompilerStage::Analysis {
            self.save();
            return;
        }

        match _fnkind {
            // fn foo()
            FnKind::ItemFn(_ident, _unsafety, _spanned, _abi, _visibility, _block) => {
                //println!("\n[auto_parallelise] check_fn(context, ItemFn: {:?}, {:?}, {:?}, {})", _block, _fndecl, _span, _nodeid);
                println!("\n\n{:?}", _fndecl);
                let ident_name: String = _ident.name.to_string();
                let ident_ctxt: String = format!("{:?}", _ident.ctxt);
                let input_types = vec![]; // TODO
                for ref arg in &_fndecl.inputs {
                    println!("ARG: {:?}, {:?}", arg.ty.node, arg.pat);
                }

                let deptree = dependency_analysis::analyse_block(&_block);
                println!("DEPTREE:");
                for node in &deptree {
                    println!("{:?}", node);
                }

                // convert deptree into encoded_deptree
                let encoded_deptree = dependency_analysis::encode_deptree(&deptree);

                println!("ENCODED_DEPTREE:");
                println!("{:?}", encoded_deptree);

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

            // fn foo(&self), i.e. obj.foo();
            FnKind::Method(_ident, _method_sig, _visibility, _block) =>
            println!("\n[auto_parallelise] check_fn(context, Method: {:?}, {:?}, {:?}, {})", _block, _fndecl, _span, _nodeid),

            // |x, y| body
            FnKind::Closure(_body) =>
            println!("\n[auto_parallelise] check_fn(context, Closure: {:?}, {:?}, {:?}, {})", _body, _fndecl, _span, _nodeid),
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
                    println!("[auto_parallelise] Recompile to apply parallelization modifications");
                    ::std::process::exit(1);
                },
                CompilerStage::Modification => {
                    println!("[auto_parallelise] parallelised compilation complete");
                    self.delete();
                    ::std::process::exit(1); // TODO: REMOVE
                },
            }
        }
    }
}