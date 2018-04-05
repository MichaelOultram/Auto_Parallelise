use syntax::ptr::P;
use syntax::ast::{self, Local, Stmt, StmtKind, Expr, ExprKind, Block, Ident, Item, ItemKind, Path, PathSegment, Pat, PatKind};
use syntax::codemap::dummy_spanned;
use syntax::ext::base::{ExtCtxt};
use syntax_pos::{BytePos, Span};
use std::ops::Deref;

use parallel_stages::{dependency_analysis, scheduler, deconstructor};
use self::dependency_analysis::{Environment, PathName, StmtID, DependencyNode};
use self::scheduler::{Schedule, ScheduleTree};
use plugin::shared_state::Config;

use serde_json;

pub fn create_block(cx: &mut ExtCtxt, stmts: Vec<Stmt>, stmtid: Option<StmtID>) -> Block {
    let block = quote_block!(cx, {});
    let span = if let Some((lo, hi)) = stmtid {
        Span::new(BytePos(lo), BytePos(hi), block.span.ctxt())
    } else {
        block.span
    };
    Block {
        stmts: stmts,
        id: block.id,
        rules: block.rules,
        span: span,
        recovered: block.recovered,
    }
}

pub fn create_path(pathname: PathName) -> Path {
    let mut segments = vec![];
    for segment_ident in pathname {
        let segment = PathSegment::from_ident(segment_ident, Span::default());
        segments.push(segment);
    }
    Path {
        span: Span::default(),
        segments: segments,
    }
}

pub fn create_function(cx: &mut ExtCtxt, item: &Item, func_name: &str, join_handle: bool, body: Block) -> (Ident, Item) {
    if let ItemKind::Fn(ref fndecl, ref unsafety, ref constness, ref abi, ref generics, _) = item.node {
        let mut new_fndecl = fndecl.deref().clone();
        eprintln!("new_fndecl: {:?}", new_fndecl);
        if join_handle {
            let new_ty = match new_fndecl.output {
                ast::FunctionRetTy::Default(_) => quote_ty!(cx, ::std::thread::JoinHandle<()>),
                ast::FunctionRetTy::Ty(ty) => quote_ty!(cx, ::std::thread::JoinHandle<$ty>),
            };
            new_fndecl.output = ast::FunctionRetTy::Ty(new_ty);
        }

        let new_func = ItemKind::Fn(P(new_fndecl), *unsafety, *constness, *abi, generics.clone(), P(body));
        let ident = Ident::from_str(func_name);
        let item = Item {
            attrs: item.attrs.clone(),
            id: item.id,
            ident: ident.clone(),
            node: new_func,
            span: item.span,
            tokens: item.tokens.clone(),
            vis: item.vis.clone(),
        };
        (ident, item)
    } else {
        panic!("Invalid ItemKind given to create_function")
    }
}

fn create_thread(cx: &mut ExtCtxt, lo: u32, hi: u32, thread_contents: Vec<Stmt>) -> (Ident, Stmt){
    let thread_sname = format!("thread_{}_{}", lo, hi);
    let thread_name = Ident::from_str(&thread_sname);
    let thread_block = create_block(cx, thread_contents, None);
    let thread_stmt = quote_stmt!(cx, let $thread_name = std::thread::spawn(move || $thread_block);).unwrap();
    (thread_name, thread_stmt)
}

fn envtuple_expr(cx: &mut ExtCtxt, env: &Environment) -> P<Expr> {
    let mut tuple = quote_expr!(cx, ()).deref().clone();
    if let ExprKind::Tup(ref mut exprl) = tuple.node {
        for var in env.clone().into_iter() {
            let path = create_path(var);
            exprl.push(quote_expr!(cx, $path));
        }
    } else {
        panic!("was not tup")
    }
    eprintln!("ENV: {:?}, TUPLE: {:?}", env, tuple);
    P(tuple)
}

fn envtuple_pat(cx: &mut ExtCtxt, env: &Environment) -> P<Pat> {
    let mut tuple = quote_pat!(cx, ()).deref().clone();
    if let PatKind::Tuple(ref mut pats, _) = tuple.node {
        for var in env.clone().into_iter() {
            let ident = var[0];
            let spanned_ident = dummy_spanned(ident);
            pats.push(quote_pat!(cx, mut $spanned_ident));
        }
    } else {
        panic!("was not tup")
    }
    eprintln!("ENV: {:?}, TUPLE: {:?}", env, tuple);
    P(tuple)
}

fn syncline_name(stmtid1: &StmtID, stmtid2: &StmtID, env: &Environment) -> String {
    let &(to_a, to_b) = stmtid1;
    let &(from_a, from_b) = stmtid2;
    let mut line_name = format!("syncline_{}_{}_{}_{}", to_a, to_b, from_a, from_b);
    for path in env.clone().into_depstr() {
        for (var, marks) in path {
            line_name.push_str(&format!("_{}", var));
            for mark in marks {
                line_name.push_str(&format!("{}", mark));
            }
        }
    }
    line_name
}

pub fn spawn_from_schedule<'a>(config: &Config, cx: &mut ExtCtxt, schedule: Schedule) -> Vec<Stmt> {
    // Gather all the synclines and create them as variables
    let synclines = schedule.get_all_synclines();
    eprintln!("Synclines:\n{:?}\n", synclines);
    let mut stmts = vec![];
    for (ref stmtid1, ref stmtid2, ref env) in synclines.clone() {
        let line_name = syncline_name(stmtid1, stmtid2, env);
        let sx = Ident::from_str(&format!("{}_send", line_name));
        let rx = Ident::from_str(&format!("{}_receive", line_name));
        let stmt = quote_stmt!(cx, let ($sx, $rx) = std::sync::mpsc::channel()).unwrap();
        stmts.push(stmt);
    }
    let mut body: Vec<Stmt> = spawn_from_schedule_helper(config, cx, schedule.list(), &synclines);
    stmts.append(&mut body);
    stmts
}

pub fn create_seq_fn(cx: &mut ExtCtxt, seq_fn_name: &String, parident: &Ident, item: &Item) -> (Ident, Item) {
    let mut seqcall =  {
        if let ItemKind::Fn(ref fndecl, _, _, _, _, _) = item.node {
            let seqcall = quote_stmt!(cx,let output = $parident();).unwrap();
            if let StmtKind::Local(ref local) = seqcall.node {
                let expr = local.init.clone().unwrap();
                if let ExprKind::Call(ref callname, ref exprl) = expr.node {
                    let mut exprl = exprl.clone();
                    // Add all variables into exprl
                    for arg in &fndecl.inputs {
                        let pat = arg.pat.deref().clone();
                        let env = deconstructor::check_pattern(&mut vec![], &pat.node);
                        assert!(env.len() == 1);
                        let patexpr = create_path(env.get(0).unwrap().clone()).clone();
                        exprl.push(quote_expr!(cx, patexpr));
                    }
                    // Reconstruct statement
                    Stmt {
                        id: seqcall.id.clone(),
                        span: seqcall.span.clone(),
                        node: StmtKind::Local(P(Local {
                            attrs: local.attrs.clone(),
                            id: local.id.clone(),
                            pat: local.pat.clone(),
                            span: local.span.clone(),
                            ty: local.ty.clone(),
                            init: Some(P(Expr {
                                attrs: expr.attrs.clone(),
                                id: expr.id.clone(),
                                node: ExprKind::Call(callname.clone(), exprl),
                                span: expr.span.clone(),
                            })),
                        })),
                    }
                } else {
                    panic!("{:?}", expr)
                }
            } else {
                panic!("{:?}", seqcall)
            }
        } else {
            panic!("{:?}", item)
        }
    };
    let seqstmt = quote_stmt!(cx, output.join().unwrap()).unwrap();
    let seqstmts = vec![seqcall, seqstmt];
    let seqblock = create_block(cx, seqstmts, None);
    create_function(cx, item, seq_fn_name, false, seqblock)
}

fn unwrap_stmts_to_blocks(stmts: &Vec<Stmt>) -> Vec<Block> {
    eprintln!("unwrap_stmts_to_blocks({:?})", stmts);
    let mut output = vec![];
    for id in 0..stmts.len() {
        // Work in pairs as stmt is in style of:
        // let return_value = block; return_value; let return_value = block; return_value;
        if id % 2 == 0 { // Even id's only
            let stmt = &stmts[id];
            let expr = {
                if let StmtKind::Local(ref local) = stmt.node {
                    if let Some(ref expr) = local.init {
                        if let ExprKind::Block(ref outer_block) = expr.deref().node {
                            let inner_stmts = &outer_block.deref().stmts;
                            assert!(inner_stmts.len() == 1, format!("Inner Stmts has {} statments: {:?}", inner_stmts.len(), inner_stmts));
                            let inner_stmt = &inner_stmts[0];
                            match inner_stmt.node {
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
                            }
                        } else {
                            panic!("Was not a let = block: {:?}", stmt);
                        }
                    } else {
                        panic!("Was just a let pattern;: {:?}", stmt);
                    }
                } else {
                    panic!("Was not a let return_value: {:?}", stmt);
                }
            };
            if let ExprKind::Block(ref block) = expr.deref().node {
                output.push(block.deref().clone());
            } else {
                panic!("Was not a block: {:?}", stmt);
            }
        }
    }
    output
}

fn spawn_from_schedule_helper<'a>(config: &Config, cx: &mut ExtCtxt, sch: &Vec<ScheduleTree<'a>>, all_synclines: &Vec<(StmtID, StmtID, &Environment)>) -> Vec<Stmt> {
    let mut output = vec![];
    let mut threads = vec![];
    let mut add_return_value = false;

    for i in 0..sch.len() {
        if let ScheduleTree::SyncTo(ref stmtid1, ref stmtid2, ref env) = sch[i] {
            let line_name = syncline_name(stmtid1, stmtid2, env);
            let sx = Ident::from_str(&format!("{}_send", line_name));
            let envexpr = envtuple_expr(cx, env);
            let prereq = quote_stmt!(cx, $sx.send($envexpr).unwrap();).unwrap();
            output.push(prereq);
        } else {
            let mut thread_contents = vec![];
            // Add prereqs and create a schedule for children
            let ((lo, hi), mut children) = match sch[i] {
                ScheduleTree::Block(ref prereqs, ref spanning_tree, _) |
                ScheduleTree::Node(ref prereqs, ref spanning_tree) => {
                    eprintln!("{:?} paths: {:?}", spanning_tree.node.get_stmtid(), spanning_tree.node.get_env());

                    // Add prereqs
                    let ref stmtid2 = spanning_tree.node.get_stmtid();
                    for &(ref stmtid1, ref sync_env) in prereqs {
                        let line_name = syncline_name(stmtid1, stmtid2, sync_env);
                        let rx = Ident::from_str(&format!("{}_receive", line_name));
                        let prereq = if sync_env.len() > 0 {
                            let envexpr = envtuple_pat(cx, sync_env);
                            quote_stmt!(cx, let $envexpr = $rx.recv().unwrap();).unwrap()
                        } else {
                            quote_stmt!(cx, $rx.recv().unwrap();).unwrap()
                        };
                        thread_contents.push(prereq);
                    }

                    // Spawn children after node
                    let children = spawn_from_schedule_helper(config, cx, &spanning_tree.children, all_synclines);

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
                    match spanning_tree.node {
                        &DependencyNode::Block(ref stmtid, _, _, _) => {
                            // Add block to the schedule
                            let mut inner_block = spawn_from_schedule_helper(config, cx, schedule.list(), all_synclines);
                            let exprblock = create_block(cx, inner_block, Some(*stmtid));
                            let mut mnode_stmt = spanning_tree.node.get_stmt();
                            let stmt = quote_stmt!(cx, $exprblock).unwrap();
                            thread_contents.push(stmt);
                        },
                        &DependencyNode::ExprBlock(ref exprblockstmt, _, _, _) => {
                            // Add block to the schedule
                            eprintln!("ScheduleTree Block ExprBlock: {:?}", exprblockstmt);

                            let mut mnode_stmt = spanning_tree.node.get_stmt();
                            let stmt =
                                if let Some(node_stmt) = mnode_stmt {
                                    // If the block has no external dependencies, then it can be run in parallel
                                    let (ref inenv, _) = schedule.get_env();
                                    // TODO: Send inenv so that for loops can be parallelised.
                                    exprblock_into_statement(config, cx, node_stmt.deref().clone(), &schedule, inenv, all_synclines)
                                } else {
                                    // Convert inner schedules into blocks
                                    let mut inner_blocks_stmts = vec![];
                                    for inner_schedule_tree in schedule.list() {
                                        let inner_schedule: Vec<ScheduleTree<'a>> = vec![inner_schedule_tree.clone()];
                                        let mut inner_block_stmt = spawn_from_schedule_helper(config, cx, &inner_schedule, all_synclines);
                                        inner_blocks_stmts.append(&mut inner_block_stmt);
                                    }
                                    let mut inner_blocks = unwrap_stmts_to_blocks(&inner_blocks_stmts);
                                    let exprblock = create_block(cx, inner_blocks_stmts, Some(stmtID!(exprblockstmt)));
                                    quote_stmt!(cx, $exprblock).unwrap()
                                };
                            thread_contents.push(stmt);
                        },
                        _ => panic!("Unexpected node type in block {:?}", spanning_tree.node),
                    }
                }
                ScheduleTree::SyncTo(_, _, _) => panic!("Unreachable case"),
            };

            // Add children
            thread_contents.append(&mut children);

            if i == sch.len() - 1 {
                // Last uses the current thread
                if true { // TODO: threads.len() > 0 {
                    // Place in a block so that we can get the correct return_value
                    let return_block = create_block(cx, thread_contents, None);
                    let let_stmt = quote_stmt!(cx, let return_value = $return_block;).unwrap();
                    output.push(let_stmt);
                    add_return_value = true;
                } else {
                    // No threads to join so don't need messy return_value
                    // TODO: This breaks unwrap_stmts_to_blocks
                    output.append(&mut thread_contents);
                }
            } else {
                // All execpt the last is put into a concurrent thread
                let (thread_name, thread_stmt) = create_thread(cx, lo, hi, thread_contents);
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

    if add_return_value {
        output.push(quote_stmt!(cx, return_value).unwrap());
    }

    output
}

fn exprblock_into_statement<'a>(config: &Config, cx: &mut ExtCtxt, exprstmt: Stmt, inner_schedule: &Schedule<'a>, inenv: &Environment, all_synclines: &Vec<(StmtID, StmtID, &Environment)>) -> Stmt {

    // Convert inner schedules into blocks
    let mut inner_blocks_stmts = vec![];
    for inner_schedule_tree in inner_schedule.list() {
        let inner_schedule: Vec<ScheduleTree<'a>> = vec![inner_schedule_tree.clone()];
        let mut inner_block_stmt = spawn_from_schedule_helper(config, cx, &inner_schedule, all_synclines);
        inner_blocks_stmts.append(&mut inner_block_stmt);
    }
    let mut inner_blocks = unwrap_stmts_to_blocks(&inner_blocks_stmts);


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
        // One block
        ExprKind::While(ref a, ref empty_block, ref b) => {
            let exprblock = inner_blocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::While(a.clone(), P(exprblock), b.clone())
        } ,
        ExprKind::WhileLet(ref a, ref b, ref empty_block, ref c) => {
            let exprblock = inner_blocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::WhileLet(a.clone(), b.clone(), P(exprblock), c.clone())
        },
        ExprKind::Loop(ref empty_block, ref a) => {
            let exprblock = inner_blocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::Loop(P(exprblock), a.clone())
        },
        ExprKind::Block(ref empty_block) => {
            let exprblock = inner_blocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::Block(P(exprblock))
        },
        ExprKind::Catch(ref empty_block) => {
            let exprblock = inner_blocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::Catch(P(exprblock))
        },

        // Two blocks (maybe)
        ExprKind::If(ref a, ref empty_block, ref c) => {
            let thenblock = inner_blocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(thenblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(thenblock)));
            let elseblock = if let &Some(ref expr) = c {
                let expr = expr.deref();
                let exprblock = inner_blocks.remove(0);
                assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
                Some(P(Expr {
                    id: expr.id.clone(),
                    node: ExprKind::Block(P(exprblock)),
                    span: expr.span.clone(),
                    attrs: expr.attrs.clone(),
                }))
            } else {
                c.clone()
            };
            ExprKind::If(a.clone(), P(thenblock), elseblock)
        },
        ExprKind::IfLet(ref a, ref b, ref empty_block, ref c) => {
            let thenblock = inner_blocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(thenblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(thenblock)));
            let elseblock = if let &Some(ref expr) = c {
                let expr = expr.deref();
                let exprblock = inner_blocks.remove(0);
                assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
                Some(P(Expr {
                    id: expr.id.clone(),
                    node: ExprKind::Block(P(exprblock)),
                    span: expr.span.clone(),
                    attrs: expr.attrs.clone(),
                }))
            } else {
                c.clone()
            };
            ExprKind::IfLet(a.clone(), b.clone(), P(thenblock), elseblock)
        },

        // Special parallel loop iterations (maybe)
        ExprKind::ForLoop(ref a, ref b, ref empty_block, ref c) => {
            let exprblock = inner_blocks.remove(0);
            eprintln!("exprblock in forloop: {:?}", exprblock);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            if config.parallel_for_loops {
                // Only attempt for _ in 0..10 {} kind (as copy trait probably implemented?)
                if let ExprKind::Range(_,_,_) = b.deref().node {
                    // Create a forward and backward mutable inenv without the a variable
                    let mut forward_inenv = inenv.clone();
                    let a_env = deconstructor::check_pattern(&mut vec![], &a.deref().node);
                    forward_inenv.remove_env(a_env);
                    eprintln!("Possible FORLOOP Parallelisation: {:?}", forward_inenv);
                    let mut backward_inenv = vec![];
                    let mut return_inenv = forward_inenv.clone();
                    let mut additional_synclines = vec![];

                    // Create a copy of inner_schedule as we might change our mind on for loop parallelisation
                    let mut adapted_inner_schedule = inner_schedule.clone();

                    // Start from top of subtree and add inenv as dependencies
                    // Do not enter ExprBlocks as they are not guarenteed to be run
                    adapted_inner_schedule.navigate_forward_avoid_exprblock(&mut (&mut forward_inenv, &mut backward_inenv), &|&mut (ref mut forward_inenv, ref mut backward_inenv), tree| {
                        let menv = if let Some(spanning_tree) = tree.get_spanning_tree() {
                            let &(ref env, _) = spanning_tree.node.get_env();
                            Some(env.clone())
                        } else {
                            None
                        };
                        if let Some(env) = menv {
                            for var in env.into_iter() {
                                if forward_inenv.contains(&var) {
                                    // Found first use of var: add a dependency syncline
                                    let syncline_env = Environment::new(vec![var.clone()]);
                                    forward_inenv.remove_env(syncline_env.clone());
                                    backward_inenv.push((var, tree.get_spanning_tree().unwrap().node.get_stmtid()));
                                    tree.get_deps_mut().unwrap().push((stmtID!(exprstmt), syncline_env));
                                }
                            }
                        }
                    });
                    assert!(forward_inenv.len() == 0, format!("Did not consume all of forward_inenv: {:?}", forward_inenv));

                    // Start from bottom of subtree and add inenv as releasing
                    adapted_inner_schedule.navigate_backward_avoid_exprblock(&mut (&mut backward_inenv, &mut additional_synclines), &|&mut (ref mut backward_inenv, ref mut additional_synclines), tree| {
                        let menv = if let Some(spanning_tree) = tree.get_spanning_tree() {
                            let &(ref env, _) = spanning_tree.node.get_env();
                            Some(env.clone())
                        } else {
                            None
                        };
                        if let Some(ref env) = menv {
                            for ref var in env.clone().into_iter() {
                                let mut mdep_stmtid = None;
                                for &(ref this_var, ref this_dep_stmtid) in backward_inenv.iter() {
                                    if Environment::new(vec![var.clone()]).contains(this_var) {
                                        mdep_stmtid = Some(this_dep_stmtid);
                                    }
                                }

                                if let Some(dep_stmtid) = mdep_stmtid {
                                    // Variable was found in backward_inenv: Add a SyncTo line
                                    let from = stmtID!(exprstmt);
                                    let to = *dep_stmtid;
                                    let syncline_env = Environment::new(vec![var.clone()]);
                                    additional_synclines.push((from, to, syncline_env.clone()));
                                    let syncline = ScheduleTree::SyncTo(from, to, syncline_env);
                                    tree.get_spanning_tree_mut().unwrap().children.insert(0,syncline);
                                }
                            }
                            // Remove from backward_inenv as we only need to find the var once
                            backward_inenv.retain(|&(ref elem, _)| !env.contains(elem));
                        }
                    });
                    let adapted_inner_schedule_json = match serde_json::to_string_pretty(&adapted_inner_schedule) {
                        Ok(obj) => obj,
                        Err(why) => panic!("Unable to convert AutoParallelise to JSON: {}", why),
                    };
                    eprintln!("Adpated_Inner_Schedule:\n{}\n", adapted_inner_schedule_json);
                    assert!(backward_inenv.len() == 0, format!("Did not consume all of backward_inenv: {:?}", backward_inenv));

                    // TODO: If a dependency is first line, and release is last line, do not parallelise (not worth it)

                    // Add some code before the loop, at the beginning of each iteration, and at the end to deal with synclines between iterations
                    let mut start_stmts = vec![];
                    let mut iteration_stmts = vec![];
                    let mut send_stmts = vec![];
                    let mut collection_stmts = vec![];
                    for &(ref stmtid1, ref stmtid2, ref env) in &additional_synclines {
                        let line_name = syncline_name(stmtid1, stmtid2, env);
                        // Start
                        let sx0 = Ident::from_str(&format!("{}_send_0", line_name));
                        let rx0 = Ident::from_str(&format!("{}_receive_0", line_name));
                        let rxi = Ident::from_str(&format!("{}_receive_i", line_name));
                        start_stmts.push(quote_stmt!(cx, let ($sx0, $rx0) = std::sync::mpsc::channel();).unwrap());
                        start_stmts.push(quote_stmt!(cx, let mut $rxi = $rx0;).unwrap());
                        // Iteration
                        let sx = Ident::from_str(&format!("{}_send", line_name));
                        let rx = Ident::from_str(&format!("{}_receive", line_name));
                        let rxn = Ident::from_str(&format!("{}_receive_new", line_name));
                        iteration_stmts.push(quote_stmt!(cx, let ($sx, $rxn) = std::sync::mpsc::channel();).unwrap());
                        iteration_stmts.push(quote_stmt!(cx, let $rx = $rxi;).unwrap());
                        iteration_stmts.push(quote_stmt!(cx, $rxi = $rxn;).unwrap());
                        // Send
                        let send_stmt = if env.len() > 0 {
                            let envexpr = envtuple_expr(cx, env);
                            quote_stmt!(cx, $sx0.send($envexpr).unwrap();).unwrap()
                        } else {
                            quote_stmt!(cx, $sx0.send(()).unwrap();).unwrap()
                        };
                        send_stmts.push(send_stmt);
                        // Collection
                        let collection_stmt = if env.len() > 0 {
                            let envexpr = envtuple_pat(cx, env);
                            quote_stmt!(cx, let $envexpr = $rxi.recv().unwrap();).unwrap()
                        } else {
                            quote_stmt!(cx, $rxi.recv().unwrap();).unwrap()
                        };
                        collection_stmts.push(collection_stmt);
                    }

                    // Create a new thread block out of new inner_schedule
                    let mut adapted_all_synclines = adapted_inner_schedule.get_all_synclines();
                    let mut adapted_inner_blocks_stmts = vec![];
                    for inner_schedule_tree in adapted_inner_schedule.list() {
                        let inner_schedule: Vec<ScheduleTree<'a>> = vec![inner_schedule_tree.clone()];
                        let mut inner_block_stmt = spawn_from_schedule_helper(config, cx, &inner_schedule, &adapted_all_synclines);
                        adapted_inner_blocks_stmts.append(&mut inner_block_stmt);
                    }
                    let adapted_inner_block = create_block(cx, adapted_inner_blocks_stmts, None);
                    let thread_block = quote_stmt!(cx, ::std::thread::spawn(move || $adapted_inner_block);).unwrap();
                    iteration_stmts.push(thread_block);
                    iteration_stmts.push(quote_stmt!(cx, ()).unwrap());

                    // Create block out of iteration_stmts
                    let iteration_block = create_block(cx, iteration_stmts, None);

                    // Reconstruct for loop with new block
                    let for_loop_exprnode = ExprKind::ForLoop(a.clone(), b.clone(), P(iteration_block.clone()), c.clone());
                    let mut for_loop_expr = expr.deref().clone();
                    for_loop_expr.node = for_loop_exprnode;
                    let for_loop_expr = P(for_loop_expr);
                    let for_loop_stmt = quote_stmt!(cx, $for_loop_expr).unwrap();

                    // Construct a block containing start, for_loop, end stmts
                    let mut start_end_stmts = vec![];
                    start_end_stmts.append(&mut start_stmts);
                    start_end_stmts.push(for_loop_stmt);
                    start_end_stmts.append(&mut send_stmts);
                    start_end_stmts.append(&mut collection_stmts);
                    if return_inenv.len() > 0 {
                        let envexpr = envtuple_expr(cx, &return_inenv);
                        start_end_stmts.push(quote_stmt!(cx, $envexpr).unwrap());
                    }
                    let start_end_block = create_block(cx, start_end_stmts, None);

                    // Combine start_end_block with a let statement
                    let stmt = if return_inenv.len() > 0 {
                        let envexpr = envtuple_pat(cx, &return_inenv);
                        quote_stmt!(cx, let $envexpr = $start_end_block;).unwrap()
                    } else {
                        quote_stmt!(cx, $start_end_block;).unwrap()
                    };

                    // Special Case: Return early
                    return stmt;
                }
            }
            ExprKind::ForLoop(a.clone(), b.clone(), P(exprblock), c.clone())
        },

        // Any number of blocks
        ExprKind::Match(ref expr1, ref arml) => {
            let mut new_arms = vec![];
            for arm in arml {
                // Replace arm if it is a block
                if let ExprKind::Block(ref empty_block) = arm.body.deref().node {
                    let expr = arm.body.deref();
                    // Find block in inner_blocks with the same id as empty_block
                    let exprblock = inner_blocks.remove(0);
                    assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
                    new_arms.push(ast::Arm {
                        attrs: arm.attrs.clone(),
                        pats: arm.pats.clone(),
                        guard: arm.guard.clone(),
                        body: P(Expr {
                            id: expr.id.clone(),
                            node: ExprKind::Block(P(exprblock)),
                            span: expr.span.clone(),
                            attrs: expr.attrs.clone(),
                        }),
                    })
                } else {
                    new_arms.push(arm.clone())
                }
            }
            ExprKind::Match(expr1.clone(), new_arms)
        },
        _ => panic!("Unexpected ExprKind: {:?}", expr.node),
    };

    assert!(inner_blocks.len() == 0, format!("Did not consume all of inner_blocks: {:?}", inner_blocks));

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
