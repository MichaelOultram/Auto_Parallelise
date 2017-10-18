use syntax::codemap::Span;
use syntax::ast::{BindingMode, ItemKind, MetaItem, FunctionRetTy, Stmt, StmtKind, PatKind, ExprKind};
use syntax::ast::Mutability::*;
use syntax::ext::base::{SyntaxExtension, ExtCtxt, Annotatable};

use AutoParallelise;

impl AutoParallelise {
    pub fn gen_syntax_extension(&mut self) -> SyntaxExtension {
        unimplemented!() // TODO
    }

}
