use syntax::ptr::P;
use syntax::ast::{self, Local, Stmt, StmtKind, Expr, ExprKind, Block, Ident, Item, ItemKind, Path, PathSegment};
use syntax::ext::base::{ExtCtxt};
use syntax_pos::{BytePos, Span};
use std::ops::Deref;

use parallel_stages::{dependency_analysis, scheduler, deconstructor};
use self::dependency_analysis::{Environment, PathName, StmtID, DependencyNode};
use self::scheduler::{Schedule, ScheduleTree};

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

pub fn create_path(cx: &mut ExtCtxt, pathname: PathName) -> P<Expr> {
    let mut segments = vec![];
    for segment_ident in pathname {
        let segment = PathSegment::from_ident(segment_ident, Span::default());
        segments.push(segment);
    }
    let var_name = Path {
        span: Span::default(),
        segments: segments,
    };
    quote_expr!(cx,$var_name)
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

fn envtuple(cx: &mut ExtCtxt, env: Environment) -> P<Expr> {
    let mut tuple = quote_expr!(cx, ()).deref().clone();
    if let ExprKind::Tup(ref mut exprl) = tuple.node {
        for var in env.clone().into_iter() {
            exprl.push(create_path(cx, var));
        }
    } else {
        panic!("was not tup")
    }
    eprintln!("ENV: {:?}, TUPLE: {:?}", env, tuple);
    P(tuple)
}

fn syncline_env(synclines: &Vec<(StmtID, StmtID, &Environment)>, to_a: u32, to_b: u32, from_a: u32, from_b: u32) -> Environment {
    for &((ta, tb), (fa, fb), env) in synclines {
        if ta == to_a && tb == to_b && fa == from_a && fb == from_b {
            return env.clone();
        }
    }
    panic!("Cannot find syncline ({}_{}_{}_{}) and so cannot return environment", to_a, to_b, from_a, from_b)
}

pub fn spawn_from_schedule<'a>(cx: &mut ExtCtxt, schedule: Schedule) -> Vec<Stmt> {
    // Gather all the synclines and create them as variables
    let synclines = schedule.get_all_synclines();
    eprintln!("Synclines:\n{:?}\n", synclines);
    let mut stmts = vec![];
    for ((to_a, to_b), (from_a, from_b), _) in synclines.clone() {
        let line_name = format!("syncline_{}_{}_{}_{}", to_a, to_b, from_a, from_b);
        let sx = Ident::from_str(&format!("{}_send", line_name));
        let rx = Ident::from_str(&format!("{}_receive", line_name));
        let stmt = quote_stmt!(cx, let ($sx, $rx) = std::sync::mpsc::channel()).unwrap();
        stmts.push(stmt);
    }
    let mut body: Vec<Stmt> = spawn_from_schedule_helper(cx, schedule.list(), &synclines);
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
                        let patexpr = create_path(cx, env.get(0).unwrap().clone()).clone();
                        exprl.push(patexpr);
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

fn spawn_from_schedule_helper<'a>(cx: &mut ExtCtxt, sch: &Vec<ScheduleTree<'a>>, all_synclines: &Vec<(StmtID, StmtID, &Environment)>) -> Vec<Stmt> {
    let mut output = vec![];
    let mut threads = vec![];
    let mut add_return_value = false;

    for i in 0..sch.len() {
        if let ScheduleTree::SyncTo(ref stmtid1, ref stmtid2, ref env) = sch[i] {
            let &(to_a, to_b) = stmtid1;
            let &(from_a, from_b) = stmtid2;
            let line_name = format!("syncline_{}_{}_{}_{}", to_a, to_b, from_a, from_b);
            let sx = Ident::from_str(&format!("{}_send", line_name));
            let envexpr = envtuple(cx, env.clone());
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
                    let (from_a, from_b) = spanning_tree.node.get_stmtid();
                    for &(to_a, to_b) in prereqs {
                        let line_name = format!("syncline_{}_{}_{}_{}", to_a, to_b, from_a, from_b);
                        let rx = Ident::from_str(&format!("{}_receive", line_name));
                        let sync_env = syncline_env(all_synclines, to_a, to_b, from_a, from_b);
                        let prereq = if sync_env.len() > 0 {
                            let envexpr = envtuple(cx, sync_env);
                            quote_stmt!(cx, let $envexpr = $rx.recv().unwrap();).unwrap()
                        } else {
                            quote_stmt!(cx, $rx.recv().unwrap();).unwrap()
                        };
                        thread_contents.push(prereq);
                    }

                    // Spawn children after node
                    let children = spawn_from_schedule_helper(cx, &spanning_tree.children, all_synclines);

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
                            let mut inner_block = spawn_from_schedule_helper(cx, schedule.list(), all_synclines);
                            let exprblock = create_block(cx, inner_block, Some(*stmtid));
                            let mut mnode_stmt = spanning_tree.node.get_stmt();
                            let stmt = quote_stmt!(cx, $exprblock).unwrap();
                            thread_contents.push(stmt);
                        },
                        &DependencyNode::ExprBlock(ref exprblockstmt, _, _, _) => {
                            // Add block to the schedule
                            eprintln!("ScheduleTree Block ExprBlock: {:?}", exprblockstmt);
                            let mut inner_blocks_stmts = vec![];
                            for inner_schedule_tree in schedule.list() {
                                let inner_schedule: Vec<ScheduleTree<'a>> = vec![inner_schedule_tree.clone()];
                                let mut inner_block_stmt = spawn_from_schedule_helper(cx, &inner_schedule, all_synclines);
                                inner_blocks_stmts.append(&mut inner_block_stmt);
                            }
                            let mut inner_blocks = unwrap_stmts_to_blocks(&inner_blocks_stmts);
                            let mut mnode_stmt = spanning_tree.node.get_stmt();
                            let stmt =
                                if let Some(node_stmt) = mnode_stmt {
                                    // If the block has no external dependencies, then it can be run in parallel
                                    let (ref inenv, _) = schedule.get_env();
                                    // TODO: Send inenv so that for loops can be parallelised.
                                    exprblock_into_statement(node_stmt.deref().clone(), &mut inner_blocks, inenv)
                                } else {
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

fn exprblock_into_statement(exprstmt: Stmt, exprblocks: &mut Vec<Block>, inenv: &Environment) -> Stmt {
    eprintln!("exprblock_into_statement({:?}, {:?}, {:?})", exprstmt, exprblocks, inenv);
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
            let exprblock = exprblocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::While(a.clone(), P(exprblock), b.clone())
        } ,
        ExprKind::WhileLet(ref a, ref b, ref empty_block, ref c) => {
            let exprblock = exprblocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::WhileLet(a.clone(), b.clone(), P(exprblock), c.clone())
        },
        ExprKind::Loop(ref empty_block, ref a) => {
            let exprblock = exprblocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::Loop(P(exprblock), a.clone())
        },
        ExprKind::Block(ref empty_block) => {
            let exprblock = exprblocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::Block(P(exprblock))
        },
        ExprKind::Catch(ref empty_block) => {
            let exprblock = exprblocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            ExprKind::Catch(P(exprblock))
        },

        // Two blocks (maybe)
        ExprKind::If(ref a, ref empty_block, ref c) => {
            let thenblock = exprblocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(thenblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(thenblock)));
            let elseblock = if let &Some(ref expr) = c {
                let expr = expr.deref();
                let exprblock = exprblocks.remove(0);
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
            let thenblock = exprblocks.remove(0);
            assert!(stmtID!(empty_block) == stmtID!(thenblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(thenblock)));
            let elseblock = if let &Some(ref expr) = c {
                let expr = expr.deref();
                let exprblock = exprblocks.remove(0);
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
            let exprblock = exprblocks.remove(0);
            eprintln!("exprblock in forloop: {:?}", exprblock);
            assert!(stmtID!(empty_block) == stmtID!(exprblock), format!("stmtID!({:?}) == stmtID!({:?})", stmtID!(empty_block), stmtID!(exprblock)));
            if self.config.parallel_for_loops {
                if let ExprKind::Range(_,_,_) = b.deref().node {
                    eprintln!("Possible FORLOOP Parallelisation");

                    // Special Case: Return early
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
                    // Find block in exprblocks with the same id as empty_block
                    let exprblock = exprblocks.remove(0);
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

    assert!(exprblocks.len() == 0, format!("Did not consume all of exprblocks: {:?}", exprblocks));

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
