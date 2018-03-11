use syntax::ast::{Block, Expr, ExprKind, Stmt, StmtKind, Path, PatKind, PathSegment};
use syntax::ptr::P;
use std::ops::Deref;
use std::vec;
use syntax::print::pprust;
use syntax_pos::Span;

use parallel_stages::dependency_analysis::{analyse_block_with_env, DependencyNode, DependencyTree, Environment, InOutEnvironment, PathName};

fn empty_block(block: &Block) -> P<Block> {
    P(Block {
        stmts: vec![],
        id: block.id,
        rules: block.rules,
        span: block.span,
        recovered: block.recovered,
    })
}

fn read_path(path: &Path) -> Option<PathName> {
    eprintln!("path_global: {}", path.is_global());
    let mut output = vec![];
    for segment in &path.segments {
        output.push(segment.identifier);
    }


    let mut segments = vec![];
    for segment_ident in &output {
        let segment = PathSegment::from_ident(segment_ident.clone(), Span::default());
        segments.push(segment);
    }
    let var_name = Path {
        span: Span::default(),
        segments: segments,
    };
    // Ignore paths that are too long
    if output.len() > 1 {
        None
    } else {
        eprintln!("read_path: {} -> {:?} -> {}", pprust::path_to_string(path), output, pprust::path_to_string(&var_name));
        Some(output)
    }
}

fn remove_blocks(stmt: &Stmt) -> P<Stmt> {
    let mut output = stmt.clone();

    // Extract the expression if there is one
    {
        let expr = match stmt.node {
            StmtKind::Expr(ref expr) |
            StmtKind::Semi(ref expr) => expr,
            _ => return P(output),
        };

        let expr2 =  remove_blocks_expr(expr);

        output.node = match stmt.node {
            StmtKind::Expr(_) => StmtKind::Expr(expr2),
            StmtKind::Semi(_) => StmtKind::Semi(expr2),
            _ => panic!(),
        };
    }
    P(output)
}

fn remove_blocks_expr(expr: &Expr) -> P<Expr> {
    let node = match expr.node {
        ExprKind::If(ref a, ref block, ref mc) => {
            let clean_a = remove_blocks_expr(a.deref());
            let clean_c = if let &Some(ref c) = mc {
                Some(remove_blocks_expr(c.deref()))
            } else {
                None
            };
            ExprKind::If(clean_a, empty_block(block), clean_c)
        },

        ExprKind::IfLet(ref a, ref b, ref block, ref mc) => {
            let clean_b = remove_blocks_expr(b.deref());
            let clean_c = if let &Some(ref c) = mc {
                Some(remove_blocks_expr(c.deref()))
            } else {
                None
            };
            ExprKind::IfLet(a.clone(), clean_b, empty_block(block), clean_c)
        },

        ExprKind::While(ref a, ref block, ref c) =>
        ExprKind::While(remove_blocks_expr(a), empty_block(block), c.clone()),

        ExprKind::WhileLet(ref a, ref b, ref block, ref c) =>
        ExprKind::WhileLet(a.clone(), remove_blocks_expr(b), empty_block(block), c.clone()),

        ExprKind::ForLoop(ref a, ref b, ref block, ref c) =>
        ExprKind::ForLoop(a.clone(), b.clone(), empty_block(block), c.clone()),

        ExprKind::Loop(ref block, ref c) =>
        ExprKind::Loop(empty_block(block), c.clone()),

        ExprKind::Block(ref block) =>
        ExprKind::Block(empty_block(block)),

        ExprKind::Catch(ref block) =>
        ExprKind::Catch(empty_block(block)),

        ExprKind::Box(ref a) =>
        ExprKind::Box(remove_blocks_expr(a)),

        ExprKind::Unary(ref a, ref b) =>
        ExprKind::Unary(a.clone(), remove_blocks_expr(b)),

        ExprKind::Cast(ref a, ref b) =>
        ExprKind::Cast(remove_blocks_expr(a), b.clone()),

        ExprKind::Type(ref a, ref b) =>
        ExprKind::Type(remove_blocks_expr(a), b.clone()),

        ExprKind::Field(ref a, ref b) =>
        ExprKind::Field(remove_blocks_expr(a), b.clone()),

        ExprKind::TupField(ref a, ref b) =>
        ExprKind::TupField(remove_blocks_expr(a), b.clone()),

        ExprKind::Paren(ref a) =>
        ExprKind::Paren(remove_blocks_expr(a)),

        ExprKind::Try(ref a) =>
        ExprKind::Try(remove_blocks_expr(a)),

        ExprKind::AddrOf(ref a, ref b) =>
        ExprKind::AddrOf(a.clone(), remove_blocks_expr(b)),

        ExprKind::InPlace(ref a, ref b) =>
        ExprKind::InPlace(remove_blocks_expr(a), remove_blocks_expr(b)),

        ExprKind::Assign(ref a, ref b) =>
        ExprKind::Assign(remove_blocks_expr(a), remove_blocks_expr(b)),

        ExprKind::Index(ref a, ref b) =>
        ExprKind::Index(remove_blocks_expr(a), remove_blocks_expr(b)),

        ExprKind::Repeat(ref a, ref b) =>
        ExprKind::Repeat(remove_blocks_expr(a), remove_blocks_expr(b)),

        ExprKind::Binary(ref a, ref b, ref c) =>
        ExprKind::Binary(a.clone(), remove_blocks_expr(b), remove_blocks_expr(c)),

        ExprKind::AssignOp(ref a, ref b, ref c) =>
        ExprKind::AssignOp(a.clone(), remove_blocks_expr(b), remove_blocks_expr(c)),

        ExprKind::Array(ref a) => {
            let mut clean_a = vec![];
            for expr in a {
                clean_a.push(remove_blocks_expr(expr));
            }
            ExprKind::Array(clean_a)
        },

        ExprKind::Tup(ref a) => {
            let mut clean_a = vec![];
            for expr in a {
                clean_a.push(remove_blocks_expr(expr));
            }
            ExprKind::Tup(clean_a)
        },


        ExprKind::MethodCall(ref a, ref b) => {
            let mut clean_b = vec![];
            for expr in b {
                clean_b.push(remove_blocks_expr(expr));
            }
            ExprKind::MethodCall(a.clone(), clean_b)
        },
        ExprKind::Call(ref a, ref b) => {
            let mut clean_b = vec![];
            for expr in b {
                clean_b.push(remove_blocks_expr(expr));
            }
            ExprKind::Call(remove_blocks_expr(a), clean_b)
        },

        ExprKind::Break(ref a, ref mb) => {
            let clean_b = if let &Some(ref b) = mb {
                Some(remove_blocks_expr(b))
            } else {
                None
            };
            ExprKind::Break(a.clone(), clean_b)
        }

        ExprKind::Struct(ref a, ref b, ref mc) => {
            let clean_c = if let &Some(ref c) = mc {
                Some(remove_blocks_expr(c))
            } else {
                None
            };
            ExprKind::Struct(a.clone(), b.clone(), clean_c)
        }

        ExprKind::Ret(ref ma) => {
            let clean_a = if let &Some(ref a) = ma {
                Some(remove_blocks_expr(a))
            } else {
                None
            };
            ExprKind::Ret(clean_a)
        },
        ExprKind::Yield(ref ma) => {
            let clean_a = if let &Some(ref a) = ma {
                Some(remove_blocks_expr(a))
            } else {
                None
            };
            ExprKind::Yield(clean_a)
        },

        ExprKind::Match(ref a, ref b) => {
            let mut clean_b = vec![];
            for expr in b {
                let mut expr2 = expr.clone();
                expr2.body = remove_blocks_expr(expr.body.deref());
                clean_b.push(expr2);
            }
            ExprKind::Match(remove_blocks_expr(a), clean_b)
        },

        ExprKind::Closure(ref a, ref b, ref c, ref d, ref e) => ExprKind::Closure(a.clone(), b.clone(), c.clone(), remove_blocks_expr(d), e.clone()),

        ExprKind::Range(ref ma, ref mb, ref c) => {
            let clean_a = if let &Some(ref a) = ma {
                Some(remove_blocks_expr(a))
            } else {
                None
            };
            let clean_b = if let &Some(ref b) = mb {
                Some(remove_blocks_expr(b))
            } else {
                None
            };
            ExprKind::Range(clean_a, clean_b, c.clone())
        },

        ref x => x.clone(),
    };

    let expr2 = Expr {
        id: expr.id,
        node: node,
        span: expr.span,
        attrs: expr.attrs.clone(),
    };
    P(expr2)
}

pub fn check_block(block: &Block) -> (DependencyTree, Vec<InOutEnvironment>) {
    let mut deptree: DependencyTree = vec![];
    let mut depstrtree: Vec<InOutEnvironment> = vec![];
    for stmt in &block.stmts {
        let depstr = check_stmt(&mut deptree, &stmt);
        depstrtree.push(depstr);

        // check_expr sometimes inserts blocks into deptree
        // Need to work out dependencies for these blocks
        for i in depstrtree.len()..deptree.len() {
            let node = &deptree[i];
            let subdepstr = node.extract_paths();
            depstrtree.push(subdepstr);
        }

        // Check that indexs are correct
        let alen = deptree.len();
        let blen = depstrtree.len();
        assert!(alen == blen, format!("{} != {}", alen, blen));
    }

    (deptree, depstrtree)
}

pub fn check_stmt(deptree: &mut DependencyTree, stmt: &Stmt) -> InOutEnvironment {
    match stmt.node {
        // A local let ?
        StmtKind::Local(ref local) => {
            if let Some(ref expr) = local.init {
                // Check expression
                let mut subtree = vec![];
                let (mut inenv, mut outenv) = check_expr(&mut subtree, &expr.deref());

                // Add current variable name as part of the environment
                if let PatKind::Ident(ref mode, ref ident, ref mpat) = local.pat.deref().node {
                    // TODO: Use other two unused variables
                    outenv.push(vec![ident.node]);
                } else {
                    // Managed to get something other than Ident
                    panic!("local.pat: {:?}", local.pat);
                }

                // Add Expr or ExprBlock into dependency tree
                if subtree.len() == 0 {
                    deptree.push(DependencyNode::Expr(remove_blocks(&stmt), vec![], (inenv.clone(), outenv.clone())));
                } else {
                    // Get environment of inner block
                    for v in &subtree {
                        let &(ref vin, ref vout) = v.get_env();
                        inenv.merge(vin.clone());
                        outenv.merge(vout.clone());
                    }
                    assert!(subtree.len() > 0);
                    deptree.push(DependencyNode::ExprBlock(remove_blocks(&stmt), subtree, vec![], (inenv.clone(), outenv.clone())));
                }
                (inenv, outenv)
            } else {
                // Add current variable name as part of the environment
                if let PatKind::Ident(ref mode, ref ident, ref mpat) = local.pat.deref().node {
                    (Environment::empty(), Environment::new(vec![vec![ident.node]]))
                } else {
                    // Managed to get something other than Ident
                    panic!("local.pat: {:?}", local.pat);
                }
            }
        },

        // A line in a function
        StmtKind::Expr(ref expr) |
        StmtKind::Semi(ref expr) => {
            // Check expression
            let mut subtree = vec![];
            let (mut inenv, mut outenv) = check_expr(&mut subtree, &expr.deref());

            // Add Expr or ExprBlock into dependency tree
            if subtree.len() == 0 {
                deptree.push(DependencyNode::Expr(remove_blocks(&stmt), vec![], (inenv.clone(), outenv.clone())));
            } else {
                // Get environment of inner block
                for v in &subtree {
                    let &(ref vin, ref vout) = v.get_env();
                    inenv.merge(vin.clone());
                    outenv.merge(vout.clone());
                }
                deptree.push(DependencyNode::ExprBlock(remove_blocks(&stmt), subtree, vec![], (inenv.clone(), outenv.clone())));
            }
            (inenv, outenv)
        },

        StmtKind::Item(ref item) => {
            eprintln!("ITEM: {:?}", item);
            (Environment::empty(), Environment::empty())
        },

        // Macros environment/dependenccies added form patch
        StmtKind::Mac(_) => {
            deptree.push(DependencyNode::Mac(P(stmt.clone()), vec![], (Environment::empty(), Environment::empty())));
            (Environment::empty(), Environment::empty())
        },
    }
}

pub fn check_expr(sub_blocks: &mut DependencyTree, expr: &Expr) -> InOutEnvironment {
    let mut dependencies = vec![];
    let mut produces = vec![];
    //eprintln!("{:?}", expr.node);
    let subexprs: Vec<P<Expr>> = {
        match expr.node {
            ExprKind::Box(ref expr1) |
            ExprKind::Unary(_, ref expr1) |
            ExprKind::Cast(ref expr1, _) |
            ExprKind::Type(ref expr1, _) |
            ExprKind::Field(ref expr1, _) |
            ExprKind::TupField(ref expr1, _) |
            ExprKind::Paren(ref expr1) |
            ExprKind::Try(ref expr1) => vec![expr1.clone()],

            ExprKind::AddrOf(_, ref expr1) => {
                //let mut unneeded = vec![];
                //let (subinenv, _) = check_expr(&mut unneeded, expr1);
                //produces.extend(subinenv.0);
                vec![expr1.clone()]
            },

            ExprKind::InPlace(ref expr1, ref expr2) |
            ExprKind::Binary(_, ref expr1, ref expr2) |
            ExprKind::Assign(ref expr1, ref expr2) |
            ExprKind::AssignOp(_, ref expr1, ref expr2) |
            ExprKind::Index(ref expr1, ref expr2) |
            ExprKind::Repeat(ref expr1, ref expr2) => {
                //let mut unneeded = vec![];
                //let (subinenv, _) = check_expr(&mut unneeded, expr1);
                //produces.extend(subinenv.0);
                vec![expr1.clone(), expr2.clone()]
            },

            ExprKind::Array(ref exprl) |
            ExprKind::Tup(ref exprl)  => exprl.clone(),

            ExprKind::MethodCall(ref expr1, ref exprl) => {
                // TODO: expr1 resolves to a method name
                // Should check whether method is safe/independent?
                /*for ref expr in exprl {
                    let mut unneeded = vec![];
                    let (subinenv, _) = check_expr(&mut unneeded, expr);
                    produces.extend(subinenv.0);
                }*/
                exprl.clone()
            },
            ExprKind::Call(ref expr1, ref exprl) => {
                 // TODO: expr1 resolves to a method name
                 // Should check whether method is safe/independent?
                 /*for ref expr in exprl {
                     let mut unneeded = vec![];
                     let (subinenv, _) = check_expr(&mut unneeded, expr);
                     produces.extend(subinenv.0);
                 }*/
                 exprl.clone()
            },

            ExprKind::Break(_, ref mexpr1) |
            ExprKind::Ret(ref mexpr1) |
            ExprKind::Struct(_, _, ref mexpr1) | // fields
            ExprKind::Yield(ref mexpr1) => {
                if let &Some(ref expr1) = mexpr1 {
                    vec![expr1.clone()]
                } else {
                    vec![]
                }
            },

            ExprKind::If(ref expr1, ref block1, ref mexpr2) |
            ExprKind::IfLet(_, ref expr1, ref block1, ref mexpr2) => {
                let (subdeptree, sub_env) = analyse_block_with_env(block1);
                let (subinenv, suboutenv) = sub_env.clone();
                dependencies.extend(subinenv.into_iter());
                produces.extend(suboutenv.into_iter());
                sub_blocks.push(DependencyNode::Block(stmtID!(block1), subdeptree, vec![], sub_env));
                // TODO: Examine subdeptree for external dependencies and update vec![node_id]
                if let &Some(ref expr2) = mexpr2 {
                    vec![expr1.clone(), expr2.clone()]
                } else {
                    vec![expr1.clone()]
                }
            },

            ExprKind::While(ref expr1, ref block1, _) |
            ExprKind::WhileLet(_, ref expr1, ref block1, _) |
            ExprKind::ForLoop(_, ref expr1, ref block1, _) => {
                let (subdeptree, sub_env) = analyse_block_with_env(block1);
                let (subinenv, suboutenv) = sub_env.clone();
                dependencies.extend(subinenv.into_iter());
                produces.extend(suboutenv.into_iter());
                sub_blocks.push(DependencyNode::Block(stmtID!(block1), subdeptree, vec![], sub_env));
                vec![expr1.clone()]
            },

            ExprKind::Loop(ref block1, _) |
            ExprKind::Block(ref block1) |
            ExprKind::Catch(ref block1) => {
                let (subdeptree, sub_env) = analyse_block_with_env(block1);
                let (subinenv, suboutenv) = sub_env.clone();
                dependencies.extend(subinenv.into_iter());
                produces.extend(suboutenv.into_iter());
                sub_blocks.push(DependencyNode::Block(stmtID!(block1), subdeptree, vec![], sub_env));
                vec![]
            },

            ExprKind::Match(ref expr1, ref arml) => {
                for arm in arml.deref() {
                    let mut bodysubblocks = vec![];

                    // Check the arm body
                    let (mut bodyinenv, mut bodyoutenv) = check_expr(&mut bodysubblocks, arm.body.deref());
                    let mut patternsenv = Environment::empty();
                    for pat in &arm.pats {
                        let patternenv = check_pattern(&mut vec![], &pat.deref().node);
                        patternsenv.merge(patternenv);
                    }
                    bodyinenv.remove_env(patternsenv.clone());
                    bodyoutenv.remove_env(patternsenv.clone());

                    // Remove pattensenv from sub_blocks
                    for mut bodyblock in &mut bodysubblocks {
                        let &mut (ref mut inenv, ref mut outenv) = bodyblock.get_env_mut();
                        inenv.remove_env(patternsenv.clone());
                        outenv.remove_env(patternsenv.clone());
                    }

                    // If there is a guard, check it
                    if let Some(ref _guard) = arm.guard {
                        panic!("guards not supported!");
                    }

                    dependencies.extend(bodyinenv.clone().into_iter());
                    produces.extend(bodyinenv.into_iter()); // Naively assume that we release all dependencies
                    produces.extend(bodyoutenv.into_iter()); // TODO: Does bodyoutenv make sense (it is probably empty)

                    // Push sub_blocks in correct order
                    sub_blocks.append(&mut bodysubblocks);
                }
                vec![expr1.clone()]
            },

            ExprKind::Closure(_, _, _, ref expr1, _) => {
                vec![expr1.clone()]
            },

            ExprKind::Range(ref mexpr1, ref mexpr2, _) => {
                let mut exprs = vec![];
                if let &Some(ref expr1) = mexpr1 {
                    exprs.push(expr1.clone())
                }
                if let &Some(ref expr2) = mexpr2 {
                    exprs.push(expr2.clone())
                }
                exprs
            },

            ExprKind::Path(_, ref path) => {
                if let Some(pathname) = read_path(path) {
                    dependencies.push(pathname);
                }

                // TODO: Check whether path is using move or borrow
                vec![]
            },

            // Independent base expressions
            ExprKind::Lit(_) |
            ExprKind::Mac(_) => vec![],

            // Unused expressions, panic if used
            _ => panic!(format!("Unmatched expression: {:?}", expr.node)),
        }
    };

    // Create list of stuff that is touched
    for subexpr in &subexprs {
        let (subinenv, suboutenv) = check_expr(sub_blocks, subexpr);
        dependencies.extend(subinenv.clone().into_iter());
        produces.extend(subinenv.into_iter()); // Naively assume that we release all dependencies
        produces.extend(suboutenv.into_iter());
    }

    // Return our dependency list to include those statements
    (Environment::new(dependencies), Environment::new(produces))
}

pub fn check_pattern(sub_blocks: &mut DependencyTree, patkind: &PatKind) -> Environment {
    let mut env = Environment::empty();
    match patkind {
        &PatKind::Ident(ref _binding, ref spanident, ref mpat) => {
            let ident = spanident.node;
            env.push(vec![ident]);
            if let &Some(ref pat) = mpat {
                env.merge(check_pattern(sub_blocks, &pat.node));
            }
        },

        &PatKind::Struct(ref path, ref fieldpats, _) => {
            if let Some(pathname) = read_path(path) {
                env.push(pathname)
            }
            for fieldpat in fieldpats {
                env.push(vec![fieldpat.node.ident]);
            }
        },

        &PatKind::TupleStruct(ref path, ref pats, _) => {
            if let Some(pathname) = read_path(path) {
                env.push(pathname)
            }
            for pat in pats {
                env.merge(check_pattern(sub_blocks, &pat.node));
            }
        },

        &PatKind::Path(_, ref path) => if let Some(pathname) = read_path(path) {
            env.push(pathname)
        },

        &PatKind::Tuple(ref pats, _) => {
            for pat in pats {
                env.merge(check_pattern(sub_blocks, &pat.node));
            }
        },

        &PatKind::Box(ref pat) |
        &PatKind::Ref(ref pat, _) => env.merge(check_pattern(sub_blocks, &pat.node)),

        &PatKind::Lit(ref expr) => {
            let (inenv, outenv) = check_expr(sub_blocks, expr);
            env.merge(inenv);
            env.merge(outenv);
        },

        &PatKind::Range(ref expr1, ref expr2, _) => {
            let (inenv, outenv) = check_expr(sub_blocks, expr1);
            env.merge(inenv);
            env.merge(outenv);
            let (inenv, outenv) = check_expr(sub_blocks, expr2);
            env.merge(inenv);
            env.merge(outenv);
        },

        &PatKind::Slice(ref pats1, ref mpat, ref pats2) => {
            for pat in pats1 {
                env.merge(check_pattern(sub_blocks, &pat.node));
            }
            if let &Some(ref pat) = mpat {
                env.merge(check_pattern(sub_blocks, &pat.node));
            }
            for pat in pats2 {
                env.merge(check_pattern(sub_blocks, &pat.node));
            }
        },

        _ => {},
    }

    env
}
