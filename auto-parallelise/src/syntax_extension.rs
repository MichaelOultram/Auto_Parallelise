use syntax::codemap::Span;
use syntax::ast;
use syntax::ext::base::{MultiItemModifier, ExtCtxt, Annotatable};

use AutoParallelise;

impl MultiItemModifier for AutoParallelise {
    fn expand(&self, _ecx: &mut ExtCtxt, _span: Span, _meta_item: &ast::MetaItem, _item: Annotatable) -> Vec<Annotatable> {
        vec![_item]
    }
}
