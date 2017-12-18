use rustc::lint::{LintArray, LintPass, EarlyContext, EarlyLintPass};
use syntax::ast::{self, Block, Expr, ExprKind, StmtKind};
use syntax::ptr::P;
use syntax_pos::Span;
use syntax::visit::{self, FnKind};
use std::ops::Deref;

use AutoParallelise;
use CompilerStage;
use *;

impl LintPass for AutoParallelise {
    fn get_lints(&self) -> LintArray {
        lint_array!()
    }
}

fn check_block(block: &Block) -> DependencyTree {
    let mut deptree: DependencyTree = vec![];
    for stmt in &block.stmts {
        match stmt.node {
            // A local let ?
            StmtKind::Local(ref local) => {
                if let Some(ref expr) = local.init {
                    let node = DependencyNode::Expr(P(stmt.clone()), vec![]);
                    deptree.push(node);
                    check_expr(&mut deptree, &expr.deref());
                }

                println!("Pat Attrs: {:?}", local.pat.node)
            },

            // A line in a function
            StmtKind::Expr(ref expr) |
            StmtKind::Semi(ref expr) => {
                let node = DependencyNode::Expr(P(stmt.clone()), vec![]);
                deptree.push(node);
                check_expr(&mut deptree, &expr.deref());
            },


            StmtKind::Item(ref item) => println!("{:?}", item),

            // Macros should be expanded by this point
            StmtKind::Mac(_) => unimplemented!(),
        }
    }
    deptree
}

fn check_expr(deptree: &mut DependencyTree, expr: &Expr) {
    let subexprs: Vec<P<Expr>> = {
        let node_id = deptree.len() - 1; // Last element in deptree is the current statement
        let mut node = &mut deptree[node_id]; // Dependencies of expr should be added to this node
        println!("expr.node: {:?}", expr.node);
        match expr.node {
            ExprKind::Box(ref expr1) |
            ExprKind::Unary(_, ref expr1) |
            ExprKind::Cast(ref expr1, _) |
            ExprKind::Type(ref expr1, _) |
            ExprKind::Field(ref expr1, _) |
            ExprKind::TupField(ref expr1, _) |
            ExprKind::AddrOf(_, ref expr1) |
            ExprKind::Paren(ref expr1) |
            ExprKind::Try(ref expr1) => vec![expr1.clone()],

            ExprKind::InPlace(ref expr1, ref expr2) |
            ExprKind::Binary(_, ref expr1, ref expr2) |
            ExprKind::Assign(ref expr1, ref expr2) |
            ExprKind::AssignOp(_, ref expr1, ref expr2) |
            ExprKind::Index(ref expr1, ref expr2) |
            ExprKind::Repeat(ref expr1, ref expr2) => vec![expr1.clone(), expr2.clone()],

            ExprKind::Array(ref exprl) |
            ExprKind::Tup(ref exprl)   |
            ExprKind::MethodCall(_, ref exprl) => exprl.clone(),

            ExprKind::Call(ref expr1, ref exprl) => {
                 let mut exprs = exprl.clone();
                 exprs.push(expr1.clone());
                 exprs
            },

            ExprKind::Break(_, ref mexpr1) |
            ExprKind::Ret(ref mexpr1) |
            ExprKind::Struct(_, _, ref mexpr1) | // fields
            ExprKind::Yield(ref mexpr1) => {
                if let &Some(ref expr1) = mexpr1 {
                    vec![expr1.clone()]
                } else {
                    vec![]
                }
            },

            ExprKind::If(ref expr1, ref block1, ref mexpr2) |
            ExprKind::IfLet(_, ref expr1, ref block1, ref mexpr2) => {
                let subdeptree = check_block(block1);
                // TODO: Use subdeptree
                if let &Some(ref expr2) = mexpr2 {
                    vec![expr1.clone(), expr2.clone()]
                } else {
                    vec![expr1.clone()]
                }
            },

            ExprKind::While(ref expr1, ref block1, _) |
            ExprKind::WhileLet(_, ref expr1, ref block1, _) |
            ExprKind::ForLoop(_, ref expr1, ref block1, _) => {
                let subdeptree = check_block(block1);
                // TODO: Use subdeptree
                vec![expr1.clone()]
            },

            ExprKind::Loop(ref block1, _) |
            ExprKind::Block(ref block1) |
            ExprKind::Catch(ref block1) => {
                let subdeptree = check_block(block1);
                // TODO: Use subdeptree
                vec![]
            },

            ExprKind::Match(ref expr1, ref arml) => {
                // TODO: Use arml
                vec![expr1.clone()]
            },

            ExprKind::Closure(_, ref fndecl, ref expr1, _) => {
                // TODO: Use fndecl
                vec![expr1.clone()]
            },

            ExprKind::Range(ref mexpr1, ref mexpr2, _) => {
                let mut exprs = vec![];
                if let &Some(ref expr1) = mexpr1 {
                    exprs.push(expr1.clone())
                }
                if let &Some(ref expr2) = mexpr2 {
                    exprs.push(expr2.clone())
                }
                exprs
            },

            // Unused expressions, panic if used
            _ => vec![],
        }
    };

    for subexpr in &subexprs {
        check_expr(deptree, subexpr);
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
            FnKind::ItemFn(_ident, _generics, _unsafety, _spanned, _abi, _visibility, _block) => {
                //println!("\n[auto_parallelise] check_fn(context, ItemFn: {:?}, {:?}, {:?}, {})", _block, _fndecl, _span, _nodeid);
                println!("\n\n{:?}", _fndecl);
                let ident_name: String = _ident.name.to_string();
                let ident_ctxt: String = format!("{:?}", _ident.ctxt);
                let input_types = vec![];
                for ref arg in &_fndecl.inputs {
                    println!("ARG: {:?}, {:?}", arg.ty.node, arg.pat);
                }

                check_block(&_block);

                self.parallelised_functions.push(Function {
                    ident_name: ident_name,
                    ident_ctxt: ident_ctxt,
                    input_types: input_types,
                    output_type: None,
                });
            },

            // fn foo(&self)
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
                },
            }
        }
    }
}
