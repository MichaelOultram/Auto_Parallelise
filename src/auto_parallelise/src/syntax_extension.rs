use syntax::codemap::Span;
use syntax::ptr::P;
use syntax::ast::{self, Stmt, StmtKind, Expr, ExprKind, Block, Item, ItemKind, Ident};
use syntax::ext::base::{MultiItemModifier, ExtCtxt, Annotatable};
use syntax::print::pprust;
use std::ops::Deref;

use serde_json;

use AutoParallelise;
use CompilerStage;
use dependency_analysis::{self, DependencyNode};
use shared_state::Function;
use scheduler::{self, ScheduleTree};

fn create_block(cx: &mut ExtCtxt, stmts: Vec<Stmt>) -> Block {
    let block = quote_block!(cx, {});
    Block {
        stmts: stmts,
        id: block.id,
        rules: block.rules,
        span: block.span,
        recovered: block.recovered,
    }
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
                    stmts.append(&mut spawn_from_schedule(cx, schedule.list()));

                    let new_block = create_block(cx, stmts);

                    // Convert function into use new_block
                    let new_func = ItemKind::Fn(_fndecl.clone(), *_unsafety, *_constness, *_abi, _generics.clone(), P(new_block));

                    let new_item = Item {
                        attrs: item.attrs.clone(),
                        id: item.id,
                        ident: item.ident,
                        node: new_func,
                        span: item.span,
                        tokens: item.tokens.clone(),
                        vis: item.vis.clone(),
                    };
                    // Prints the function
                    println!("converted_function:\n{}\n", pprust::item_to_string(&new_item));

                    let anno_item = Annotatable::Item(P(new_item));
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

fn spawn_from_schedule<'a>(cx: &mut ExtCtxt, sch: &Vec<ScheduleTree<'a>>) -> Vec<Stmt> {
    let mut output = vec![];
    let mut threads = vec![];

    for i in 0..sch.len() {
        if let ScheduleTree::SyncTo(ref stmtid1, ref stmtid2, _) = sch[i] {
            let &(to_a, to_b) = stmtid1;
            let &(from_a, from_b) = stmtid2;
            let line_name = format!("syncline_{}_{}_{}_{}", to_a, to_b, from_a, from_b);
            let sx = Ident::from_str(&format!("{}_send", line_name));
            let prereq = quote_stmt!(cx, $sx.send(()).unwrap();).unwrap();
            output.push(prereq);
        } else {
            let mut thread_contents = vec![];
            // Add prereqs and create a schedule for children
            let ((lo, hi), mut children) = match sch[i] {
                ScheduleTree::Block(ref prereqs, ref spanning_tree, _) |
                ScheduleTree::Node(ref prereqs, ref spanning_tree) => {
                    println!("{:?} paths: {:?}", spanning_tree.node.get_stmtid(), spanning_tree.node.get_env());

                    // Add prereqs
                    let (from_a, from_b) = spanning_tree.node.get_stmtid();
                    for &(to_a, to_b) in prereqs {
                        let line_name = format!("syncline_{}_{}_{}_{}", to_a, to_b, from_a, from_b);
                        let rx = Ident::from_str(&format!("{}_receive", line_name));
                        let prereq = quote_stmt!(cx, $rx.recv().unwrap();).unwrap();
                        thread_contents.push(prereq);
                    }

                    // Spawn children after node
                    let children = spawn_from_schedule(cx, &spanning_tree.children);

                    // Return node id
                    (spanning_tree.node.get_stmtid(), children)
                }
                ScheduleTree::SyncTo(_, _, _) => panic!("Unreachable case"),
            };

            // Add the current item
            match sch[i] {
                ScheduleTree::Node(_, ref spanning_tree) => {
                    // Copy statement of node
                    let stmt = spanning_tree.node.get_stmt().unwrap().deref().clone();
                    thread_contents.push(stmt);
                }
                ScheduleTree::Block(_, ref spanning_tree, ref schedule) => {
                    // Add block to the schedule
                    let mut inner_block = spawn_from_schedule(cx, schedule.list());
                    let exprblock = create_block(cx, inner_block);
                    let mut mnode_stmt = spanning_tree.node.get_stmt();
                    let stmt = if let Some(node_stmt) = mnode_stmt {
                        exprblock_into_statement(node_stmt.deref().clone(), exprblock)
                    } else {
                        quote_stmt!(cx, $exprblock).unwrap()
                    };
                    thread_contents.push(stmt);
                }
                ScheduleTree::SyncTo(_, _, _) => panic!("Unreachable case"),
            };

            // Add children
            thread_contents.append(&mut children);

            if i == sch.len() - 1 {
                // Last uses the current thread
                output.append(&mut thread_contents);
            } else {
                // All execpt the last is put into a concurrent thread
                let thread_sname = format!("thread_{}_{}", lo, hi);
                let thread_name = Ident::from_str(&thread_sname);
                let thread_block = create_block(cx, thread_contents);
                let thread_stmt = quote_stmt!(cx, let $thread_name = std::thread::spawn(move || $thread_block);).unwrap();
                output.push(thread_stmt);
                threads.push(thread_name);
            }
        }
    }

    // Join all threads
    for thread in threads {
        let thread_stmt = quote_stmt!(cx, $thread.join().unwrap();).unwrap();
        output.push(thread_stmt);
    }

    output
}

fn exprblock_into_statement(exprstmt: Stmt, exprblock: Block) -> Stmt {
    // Extract expr
    let expr = match exprstmt.node {
        StmtKind::Local(ref local) =>
            if let Some(ref expr) = local.init {
                expr
            } else {
                panic!("No expression");
            },

        // A line in a function
        StmtKind::Expr(ref expr) |
        StmtKind::Semi(ref expr) => expr,
        _ => panic!("Unexpected StmtKind"),
    };

    // Create new exprnode with exprblock
    let new_exprnode = match expr.node {
        ExprKind::If(ref a, _, ref c) => ExprKind::If(a.clone(), P(exprblock), c.clone()),
        ExprKind::IfLet(ref a, ref b, _, ref c) => ExprKind::IfLet(a.clone(), b.clone(), P(exprblock), c.clone()),
        ExprKind::While(ref a, _, ref b) => ExprKind::While(a.clone(), P(exprblock), b.clone()) ,
        ExprKind::WhileLet(ref a, ref b, _, ref c) => ExprKind::WhileLet(a.clone(), b.clone(), P(exprblock), c.clone()),
        ExprKind::ForLoop(ref a, ref b, _, ref c) => ExprKind::ForLoop(a.clone(), b.clone(), P(exprblock), c.clone()),
        ExprKind::Loop(_, ref a) => ExprKind::Loop(P(exprblock), a.clone()),
        ExprKind::Block(_) => ExprKind::Block(P(exprblock)),
        ExprKind::Catch(_) => ExprKind::Catch(P(exprblock)),
        _ => panic!("Unexpected ExprKind"),
    };

    // Create new expr
    let mut new_expr = expr.deref().clone();
    new_expr.node = new_exprnode;

    // Wrap new_exprnode in a new stmtkind
    let new_stmtkind = match exprstmt.node {
        StmtKind::Local(ref local) => {
            let mut local = local.deref().clone();
            local.init = Some(P(new_expr));
            StmtKind::Local(P(local))
        },
        StmtKind::Expr(_) => StmtKind::Expr(P(new_expr)),
        StmtKind::Semi(_) => StmtKind::Semi(P(new_expr)),
        _ => panic!("Unexpected StmtKind"),
    };

    // Create new statment out of stmtkind
    let mut new_stmt = exprstmt.clone();
    new_stmt.node = new_stmtkind;

    // Return new statement
    new_stmt
}
