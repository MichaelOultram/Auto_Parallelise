use syntax::ptr::P;
use syntax::ast::{self, ItemKind};
use syntax::ext::base::{MultiItemModifier, ExtCtxt, Annotatable};
use syntax_pos::Span;
use syntax::print::pprust;

use serde_json;

use AutoParallelise;
use CompilerStage;

use parallel_stages::{dependency_analysis, reconstructor, scheduler};
use rendering::dot;
use plugin::shared_state::Function;


impl MultiItemModifier for AutoParallelise {
    fn expand(&self, cx: &mut ExtCtxt, _span: Span, _meta_item: &ast::MetaItem, _item: Annotatable) -> Vec<Annotatable> {
        // Only make changes when on the Modification stage
        if !self.config.plugin_enabled || self.compiler_stage != CompilerStage::Modification {
            return vec![_item];
        }
        let mut output = vec![];
        // Unwrap item
        if let Annotatable::Item(ref item) = _item {
            // Find function name and the analysed function
            let func_ident = item.ident;
            let func_name = func_ident.name.to_string();
            eprintln!("\n\n{:?}", func_name); // Function Id

            if let ItemKind::Fn(ref _fndecl, ref _unsafety, ref _constness, ref _abi, ref _generics, ref _block) = item.node {
                eprintln!("{:?}", _fndecl); // Function decl
                eprintln!("Unsafety: {}", _unsafety);

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
                    dependency_analysis::replace_dependencies(&mut base_deptree, &analysed_function.encoded_deptree);

                    eprintln!("DEPTREE:");
                    for node in &base_deptree {
                        let node_json = match serde_json::to_string_pretty(&node) {
                            Ok(obj) => obj,
                            Err(why) => panic!("Unable to convert deptree to JSON: {}", why),
                        };
                        eprintln!("{}", node_json);
                    }

                    eprintln!("DOT deptree output:");
                    eprintln!("{}", dot::deptree_to_dot(&base_deptree));

                    // Produce a schedule
                    let schedule = scheduler::create_schedule(&base_deptree);
                    let schedule_json = match serde_json::to_string_pretty(&schedule) {
                        Ok(obj) => obj,
                        Err(why) => panic!("Unable to convert AutoParallelise to JSON: {}", why),
                    };
                    eprintln!("SCHEDULE:\n{}\n", schedule_json);

                    eprintln!("DOT schedule output:");
                    eprintln!("{}", dot::schedule_to_dot(&schedule));

                    // Convert schedule into multi-threadded code
                    let parstmts = reconstructor::spawn_from_schedule(cx, schedule);
                    let parblock = reconstructor::create_block(cx, parstmts);
                    if self.config.parallel_function_body {
                        // Surround function body in a thread
                        let parthread = quote_stmt!(cx, ::std::thread::spawn(move || $parblock)).unwrap();
                        let parthreadblock = reconstructor::create_block(cx, vec![parthread]);
                        // Convert function into use new_block
                        let (parident, parfunction) = reconstructor::create_function(cx, item, &format!("{}_parallel", func_name), true, parthreadblock);
                        let (seqident, seqfunction) = reconstructor::create_seq_fn(cx, &func_name, &parident, &item);
                        // Prints the function
                        println!("{}\n{}\n", pprust::item_to_string(&parfunction), pprust::item_to_string(&seqfunction));

                        output.push(Annotatable::Item(P(parfunction)));
                        output.push(Annotatable::Item(P(seqfunction)));
                    } else {
                        // Do not surround in a thread
                        let (parident, parfunction) = reconstructor::create_function(cx, item, &func_name, false, parblock);
                        // Prints the function
                        println!("{}\n", pprust::item_to_string(&parfunction));

                        output.push(Annotatable::Item(P(parfunction)));
                    }

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
