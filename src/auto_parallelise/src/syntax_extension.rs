use syntax::codemap::Span;
use syntax::ptr::P;
use syntax::ast::{self, Stmt, Block, Item, ItemKind, Ident};
use syntax::ext::base::{MultiItemModifier, ExtCtxt, Annotatable};
use syntax::ext::build::AstBuilder;

use serde_json;

use AutoParallelise;
use CompilerStage;
use dependency_analysis;
use shared_state::Function;
use scheduler;

fn create_block(block: &Block, stmts: Vec<Stmt>) -> P<Block> {
    P(Block {
        stmts: stmts,
        id: block.id,
        rules: block.rules,
        span: block.span,
        recovered: block.recovered,
    })
}

impl MultiItemModifier for AutoParallelise {
    fn expand(&self, cx: &mut ExtCtxt, _span: Span, _meta_item: &ast::MetaItem, _item: Annotatable) -> Vec<Annotatable> {
        // Only make changes when on the Modification stage
        if self.compiler_stage != CompilerStage::Modification {
            return vec![_item];
        }
        let mut output = vec![];
        // Unwrap item
        if let Annotatable::Item(ref item) = _item {
            // Find function name and the analysed function
            let func_ident = item.ident;
            let func_name = func_ident.name.to_string();
            println!("\n\n{:?}", func_name); // Function Id

            if let ItemKind::Fn(ref _fndecl, ref _unsafety, ref _constness, ref _abi, ref _generics, ref _block) = item.node {
                println!("{:?}", _fndecl); // Function decl

                // Find function from analysed stage
                let mut maybe_analysed_function: Option<&Function> = None;
                for func in &self.functions {
                    if func.ident_name == func_name {
                        maybe_analysed_function = Some(func);
                    }
                }
                if let Some(analysed_function) = maybe_analysed_function {
                    // Merge the dependency trees
                    let mut base_deptree = dependency_analysis::analyse_block(&_block);
                    dependency_analysis::merge_dependencies(&mut base_deptree, &analysed_function.encoded_deptree);

                    println!("DEPTREE:");
                    for node in &base_deptree {
                        println!("{:?}", node);
                    }

                    // Produce a schedule
                    let schedule = scheduler::create_schedule(&base_deptree);
                    let schedule_json = match serde_json::to_string_pretty(&schedule) {
                        Ok(obj) => obj,
                        Err(why) => panic!("Unable to convert AutoParallelise to JSON: {}", why),
                    };
                    println!("SCHEDULE:\n{}\n", schedule_json);

                    // Convert schedule into multi-threadded code

                    // Gather all the synclines and create them as variables
                    let synclines = schedule.get_all_synclines();
                    println!("Synclines:\n{:?}\n", synclines);
                    let mut stmts = vec![];
                    for ((to_a, to_b), (from_a, from_b)) in synclines {
                        let line_name = format!("syncline_{}_{}_{}_{}", to_a, to_b, from_a, from_b);
                        let sx = Ident::from_str(&format!("{}_send", line_name));
                        let rx = Ident::from_str(&format!("{}_receive", line_name));
                        let stmt = quote_stmt!(cx, let ($sx, $rx) = std::sync::mpsc::channel()).unwrap();
                        stmts.push(stmt);
                    }

                    let new_block = create_block(_block, stmts);
                    println!("Body: {:?}", new_block);

                    // Convert function into use new_block
                    let new_func = ItemKind::Fn(_fndecl.clone(), *_unsafety, *_constness, *_abi, _generics.clone(), new_block);
                    println!("new_func: {:?}", new_func);
                    let new_item = Item {
                        attrs: item.attrs.clone(),
                        id: item.id,
                        ident: item.ident,
                        node: new_func,
                        span: item.span,
                        tokens: item.tokens.clone(),
                        vis: item.vis.clone(),
                    };
                    println!("new_item: {:?}", new_item);

                    let anno_item = Annotatable::Item(P(new_item));
                    println!("new_item: {:?}", anno_item);

                    output.push(anno_item);
                } else {
                    panic!("{} was not found as an analysed function", func_name);
                }
            } else {
                panic!("ItemKind was not FN");
            }
        } else {
            panic!("Annotatable was not Item");
        }

        output
        //vec![_item]
    }
}
