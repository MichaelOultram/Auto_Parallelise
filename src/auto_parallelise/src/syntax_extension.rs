use syntax::codemap::Span;
use syntax::ast::{self, ItemKind, Ident, Stmt};
use syntax::ext::base::{MultiItemModifier, ExtCtxt, Annotatable};
use syntax::ext::build::AstBuilder;

use serde_json;

use AutoParallelise;
use CompilerStage;
use dependency_analysis;
use shared_state::Function;
use scheduler;

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
                    let mut stmts = quote_stmt!(cx, {}).unwrap();
                    for ((to_a, to_b), (from_a, from_b)) in synclines {
                        let line_name = format!("syncline_{}_{}_{}_{}", to_a, to_b, from_a, from_b);
                        let sx = Ident::from_str(&format!("{}_send", line_name));
                        let rx = Ident::from_str(&format!("{}_receive", line_name));
                        stmts = quote_stmt!(cx, {
                            $stmts
                            let ($sx, $rx) = (2, 3);
                        }).unwrap();
                    }

                    println!("Body: {:?}", stmts);

                    let new_func = quote_item!(cx,
                        fn $func_ident() -> u32 {
                            $stmts
                        }
                    ).unwrap();
                    output.push(Annotatable::Item(new_func));
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
