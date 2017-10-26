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
            FnKind::ItemFn(.., block) | FnKind::Method(.., block) =>
            println!("[auto-parallelize] check_fn(context, block: {:?}, {:?}, {:?}, {})", block, _fndecl, _span, _nodeid),
            FnKind::Closure(body) =>
            println!("[auto-parallelize] check_fn(context, body: {:?}, {:?}, {:?}, {})", body, _fndecl, _span, _nodeid),
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
