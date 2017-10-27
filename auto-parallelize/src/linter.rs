use rustc::lint::{LintArray, LintPass, EarlyContext, EarlyLintPass};
use syntax::ast;
use syntax_pos::Span;
use syntax::visit::{self, FnKind};

use AutoParallelize;
use CompilerStage;

impl LintPass for AutoParallelize {
    fn get_lints(&self) -> LintArray {
        lint_array!()
    }
}

impl EarlyLintPass for AutoParallelize {
    fn check_fn(&mut self, _context: &EarlyContext, _fnkind: visit::FnKind, _fndecl: &ast::FnDecl, _span: Span, _nodeid: ast::NodeId) {
        // Only need to analyse function during the analysis stage
        if self.compiler_stage != CompilerStage::Analysis {
            self.save();
            return;
        }

        match _fnkind {
            FnKind::ItemFn(ident, generics, unsafety, spanned, abi, visibility, block) =>
            println!("\n[auto-parallelize] check_fn(context, ItemFn: {:?}, {:?}, {:?}, {})", block, _fndecl, _span, _nodeid),

            /// fn foo(&self)
            FnKind::Method(ident, methodSig, visibility, block) =>
            println!("\n[auto-parallelize] check_fn(context, Method: {:?}, {:?}, {:?}, {})", block, _fndecl, _span, _nodeid),

            /// |x, y| body
            FnKind::Closure(body) =>
            println!("\n[auto-parallelize] check_fn(context, Closure: {:?}, {:?}, {:?}, {})", body, _fndecl, _span, _nodeid),
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
            self.delete(); // TODO: Remove this line to enable modifications
            match self.compiler_stage {
                CompilerStage::Analysis => {
                    println!("[auto-parallelize] Recompile to apply parallelization modifications");
                    ::std::process::exit(1);
                },
                CompilerStage::Modification => {
                    println!("[auto-parallelize] Parallelized compilation complete");
                    self.delete();
                },
            }
        }
    }
}
