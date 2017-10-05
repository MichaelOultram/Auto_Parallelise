use syntax::codemap::Span;
use syntax::ast::{BindingMode, ItemKind, MetaItem, FunctionRetTy, Stmt, StmtKind, PatKind, ExprKind};
use syntax::ast::Mutability::*;
use syntax::ext::base::{ExtCtxt, Annotatable};

fn print_stmt(stmt: &Stmt) {
    match &stmt.node {
        &StmtKind::Local(ref local) => {
            let (bindmode, var_name) = match &local.pat.node {
                &PatKind::Ident(ref bindmode, ref _span, ref _pat) => {
                    (bindmode.clone(), format!("{:?}", _pat))
                },
                _ => (BindingMode::ByRef(Mutable), format!("none")),
            };
            println!("    let {:?} {} = {:?}", bindmode, var_name, local.pat.node);
        },
        &StmtKind::Item(ref expr) => {
            println!("    {:?}", expr);
        },
        &StmtKind::Expr(ref expr) => {
            if let ExprKind::Call(ref fncall, ref _args) = expr.node {
                if let ExprKind::Path(_, ref fnpath) = fncall.node {
                    println!("    expr {:?}", fnpath);

                }
            }
        },
        &StmtKind::Semi(ref expr) => {
            println!("    blah {:?}", expr);
        },
        &StmtKind::Mac(ref expr) => {
            println!("    {:?}", expr);
        },
    }
}

pub fn example_extension(_exc: &mut ExtCtxt, _span: Span, _meta_item: &MetaItem, _item: Annotatable) -> Vec<Annotatable> {
    if let Annotatable::Item(ref i) = _item {
        if let ItemKind::Fn(ref _fndecl, ref _normal, ref _span, ref _rust, ref _generics, ref body) = i.node {
            let output = if let FunctionRetTy::Ty(ref t) = _fndecl.output {
                format!("-> {:?} ", t)
            } else {
                format!("")
            };
            println!("FN({:?}) {}{{", _fndecl.inputs, output);

            for ref stmt in &body.stmts {
                print_stmt(stmt);
            }
            println!("}}\n");
        }
    } else {
        unimplemented!()
    }
    vec![_item]
}
