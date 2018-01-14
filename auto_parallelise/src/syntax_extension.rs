use syntax::codemap::Span;
use syntax::ast::{self, ItemKind};
use syntax::ext::base::{MultiItemModifier, ExtCtxt, Annotatable};

use AutoParallelise;
use CompilerStage;
use dependency_analysis;
use shared_state::Function;

impl MultiItemModifier for AutoParallelise {
    fn expand(&self, _ecx: &mut ExtCtxt, _span: Span, _meta_item: &ast::MetaItem, _item: Annotatable) -> Vec<Annotatable> {
        // Only make changes when on the Modification stage
        if self.compiler_stage != CompilerStage::Modification {
            return vec![_item];
        }

        // Unwrap item
        if let Annotatable::Item(ref item) = _item {
            // Find function name and the analysed function
            let func_name = item.ident.name.to_string();
            println!("\n\n{:?}", func_name); // Function Id

            if let ItemKind::Fn(ref _fndecl, ref _unsafety, ref _constness, ref _abi, ref _generics, ref _block) = item.node {
                let mut maybe_analysed_function: Option<&Function> = None;

                println!("{:?}", _fndecl); // Function decl
                for func in &self.functions {
                    let name_match = func.ident_name == func_name;
                    // TODO: Check function arguments and return type for a full match (_fndecl)
                    if name_match {
                        maybe_analysed_function = Some(func);
                    }
                }
                if let Some(analysed_function) = maybe_analysed_function {
                    let mut base_deptree = dependency_analysis::analyse_block(&_block);
                    dependency_analysis::merge_dependencies(&mut base_deptree, &analysed_function.encoded_deptree);

                    println!("DEPTREE:");
                    for node in &base_deptree {
                        println!("{:?}", node);
                    }
                }



            } else {
                panic!("ItemKind was not FN");
            }
        } else {
            panic!("Annotatable was not Item");
        }

        vec![_item]
    }
}
