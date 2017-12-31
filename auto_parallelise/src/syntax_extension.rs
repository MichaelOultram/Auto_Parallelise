use syntax::codemap::Span;
use syntax::ptr;
use syntax::ast::{self, ItemKind};
use syntax::ext::base::{MultiItemModifier, ExtCtxt, Annotatable};
use syntax_pos::symbol::Symbol;

use AutoParallelise;
use CompilerStage;
use dependency_analysis;

impl MultiItemModifier for AutoParallelise {
    fn expand(&self, _ecx: &mut ExtCtxt, _span: Span, _meta_item: &ast::MetaItem, _item: Annotatable) -> Vec<Annotatable> {
        // Only make changes when on the Modification stage
        if self.compiler_stage != CompilerStage::Modification {
            return vec![_item];
        }

        //println!("\n[auto-parallelize] expand(_ecx, {:?}, {:?}, {:?})", _span, _meta_item, _item);
        let item2;
        if let Annotatable::Item(ref item) = _item {
            //println!("\n\n{:?}", item.ident); // Function Name
            // Creates an identical function with a 2 at the end of the name
            let mut ident2 = item.ident.clone();
            ident2.name = Symbol::intern(&format!("{}_parallel", item.ident.name));
            item2 = Annotatable::Item(ptr::P(ast::Item {
                ident: ident2,
                attrs: item.attrs.clone(),
                id: item.id.clone(),
                node: item.node.clone(),
                vis: item.vis.clone(),
                span: item.span.clone(),
                tokens: item.tokens.clone(),
            }));

            println!("{:?}", item.id); // Function Id
            if let ItemKind::Fn(ref _fndecl, ref _unsafety, ref _constness, ref _abi, ref _generics, ref _block) = item.node {
                println!("{:?}", _fndecl); // Function decl
                println!("{:?}", _block); // Function block
                println!("{:?}", dependency_analysis::check_block(&_block));
            } else {
                panic!("ItemKind was not FN");
            }
        } else {
            panic!("Annotatable was not Item");
        }

        vec![_item, item2]
    }
}
