use rustc::lint::{LintArray, LintPass, EarlyContext, EarlyLintPass};
use syntax::ast::{self, Block, Expr, ExprKind, Stmt, StmtKind};
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

fn empty_block(block: &Block) -> P<Block> {
    P(Block {
        stmts: vec![],
        id: block.id,
        rules: block.rules,
        span: block.span,
    })
}

fn remove_blocks(stmt: &Stmt) -> P<Stmt> {
    let mut output = stmt.clone();

    // Extract the expression if there is one
    {
        let expr = match stmt.node {
            StmtKind::Expr(ref expr) |
            StmtKind::Semi(ref expr) => expr,
            _ => return P(output),
        };

        // Check to see if the expression contains a block
        let node = match expr.node {
            ExprKind::If(ref a, ref block, ref c) =>
            ExprKind::If(a.clone(), empty_block(block), c.clone()),

            ExprKind::IfLet(ref a, ref b, ref block, ref c) =>
            ExprKind::IfLet(a.clone(), b.clone(), empty_block(block), c.clone()),

            ExprKind::While(ref a, ref block, ref c) =>
            ExprKind::While(a.clone(), empty_block(block), c.clone()),

            ExprKind::WhileLet(ref a, ref b, ref block, ref c) =>
            ExprKind::WhileLet(a.clone(), b.clone(), empty_block(block), c.clone()),

            ExprKind::ForLoop(ref a, ref b, ref block, ref c) =>
            ExprKind::ForLoop(a.clone(), b.clone(), empty_block(block), c.clone()),

            ExprKind::Loop(ref block, ref c) =>
            ExprKind::Loop(empty_block(block), c.clone()),

            ExprKind::Block(ref block) =>
            ExprKind::Block(empty_block(block)),

            ExprKind::Catch(ref block) =>
            ExprKind::Catch(empty_block(block)),

            ref x => x.clone(),
        };

        let expr2 = Expr {
            id: expr.id,
            node: node,
            span: expr.span,
            attrs: expr.attrs.clone(),
        };

        output.node = match stmt.node {
            StmtKind::Expr(_) => StmtKind::Expr(P(expr2)),
            StmtKind::Semi(_) => StmtKind::Semi(P(expr2)),
            _ => panic!(),
        };
    }
    P(output)
}

fn check_block(block: &Block) -> DependencyTree {
    let mut deptree: DependencyTree = vec![];
    for stmt in &block.stmts {
        match stmt.node {
            // A local let ?
            StmtKind::Local(ref local) => {
                if let Some(ref expr) = local.init {
                    deptree.push(DependencyNode::Expr(remove_blocks(&stmt), vec![]));
                    let node_id = deptree.len() - 1;
                    let dep_strs = check_expr(&mut deptree, &expr.deref(), node_id);
                    // TODO: Examine dep_strs to find the statement indicies
                }

                //println!("Pat Attrs: {:?}", local.pat.node)
            },

            // A line in a function
            StmtKind::Expr(ref expr) |
            StmtKind::Semi(ref expr) => {
                deptree.push(DependencyNode::Expr(remove_blocks(&stmt), vec![]));
                let node_id = deptree.len() - 1;
                let dep_strs = check_expr(&mut deptree, &expr.deref(), node_id);
                // TODO: Examine dep_strs to find the statement indicies

            },


            StmtKind::Item(ref item) => println!("ITEM: {:?}", item),

            // Macros should be expanded by this point
            StmtKind::Mac(_) => unimplemented!(),
        }
    }
    deptree
}

fn check_expr(deptree: &mut DependencyTree, expr: &Expr, node_id: usize) -> Vec<String> { // -> List of variables that is read/modified
    let subexprs: Vec<P<Expr>> = {
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
                deptree.push(DependencyNode::Block(subdeptree, vec![node_id]));
                // TODO: Examine subdeptree for external dependencies and update vec![node_id]
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
                deptree.push(DependencyNode::Block(subdeptree, vec![node_id]));
                // TODO: Examine subdeptree for external dependencies and update vec![node_id]
                vec![expr1.clone()]
            },

            ExprKind::Loop(ref block1, _) |
            ExprKind::Block(ref block1) |
            ExprKind::Catch(ref block1) => {
                let subdeptree = check_block(block1);
                deptree.push(DependencyNode::Block(subdeptree, vec![node_id]));
                // TODO: Examine subdeptree for external dependencies and update vec![node_id]
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

    // TODO: Create list of stuff that is touched
    let mut dependencies = vec![];
    for subexpr in &subexprs {
        dependencies.extend(check_expr(deptree, subexpr, node_id));
    }

    // TODO: Return our dependency list to include those statements
    dependencies
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

                let deptree = check_block(&_block);
                println!("DEPTREE:");
                for node in &deptree {
                    println!("{:?}", node);
                }

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
