use syntax::ast::{Block, Expr, ExprKind, Stmt, StmtKind, Path, PatKind, Ident};
use syntax::ptr::P;
use std::ops::Deref;
use serde::ser::{Serialize, Serializer, SerializeStruct, SerializeSeq};
use std::{vec, iter};
use syntax::print::pprust;

pub type PathName = Vec<Ident>;
pub type StmtID = (u32, u32);
macro_rules! stmtID {
    ($i:ident) => {{($i.span.lo().0, $i.span.hi().0)}}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Environment(Vec<PathName>);
impl Environment {
    pub fn new(dep_str: Vec<PathName>) -> Self {
        Environment{0: dep_str}
    }
    pub fn empty() -> Self {Environment::new(vec![])}
    pub fn into_depstr(&self) -> EncodedEnvironment {
        let mut depstr = vec![];
        for e in &self.0 {
            let pathstr: Vec<String> = e.into_iter().map(|i| pprust::ident_to_string(*i)).collect();
            depstr.push(pathstr);
        }
        depstr.sort_unstable();
        depstr.dedup();
        depstr
    }
    pub fn append(&mut self, patch: &EncodedEnvironment) {
        let mut env = self.into_depstr();
        env.append(&mut patch.clone());
        env.sort_unstable();
        env.dedup();
        self.0.clear();
        for p in env {
            let path: PathName = p.into_iter().map(|i| Ident::from_str(&i)).collect();
            self.0.push(path);
        }
    }
    pub fn merge(&mut self, patch: Environment) {
        for p in patch.0 {
            self.0.push(p);
        }
        self.append(&vec![]);
    }
    pub fn into_iter(self) -> vec::IntoIter<Vec<Ident>> {self.0.into_iter()}
    pub fn push(&mut self, elem: PathName) {self.0.push(elem)}
    pub fn contains(&self, elem: &PathName) -> bool {self.0.contains(elem)}
}
impl iter::FromIterator<PathName> for Environment {
    fn from_iter<I>(i: I) -> Self
    where I: IntoIterator<Item = PathName> {
        let mut env = vec![];
        for e in i {
            env.push(e)
        }
        Environment::new(env)
    }
}
impl Serialize for Environment {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        let mut seq = serializer.serialize_seq(Some(1))?;
        seq.serialize_element(&self.into_depstr())?;
        seq.end()
    }
}

// Used for actual statements
#[derive(Debug, PartialEq)]
pub enum DependencyNode {
    Expr(P<Stmt>, Vec<usize>, Environment), // Statement and Dependency indicies
    Block(StmtID, DependencyTree, Vec<usize>, Environment),
    ExprBlock(P<Stmt>, DependencyTree, Vec<usize>, Environment),
    Mac(P<Stmt>, Vec<usize>, Environment)
}
pub type DependencyTree = Vec<DependencyNode>;

// Used to store as JSON
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EncodedDependencyNode {
    Expr(StmtID, Vec<usize>, EncodedEnvironment), // Statement ID and Dependency indicies
    Block(StmtID, EncodedDependencyTree, Vec<usize>, EncodedEnvironment),
    ExprBlock(StmtID, EncodedDependencyTree, Vec<usize>, EncodedEnvironment),
    Mac(StmtID, Vec<usize>, EncodedEnvironment)
}
pub type EncodedDependencyTree = Vec<EncodedDependencyNode>;
pub type EncodedEnvironment = Vec<Vec<String>>;

impl Serialize for DependencyNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match self {
            &DependencyNode::Expr(ref stmt, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Expr", 4)?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("stmt", &format!("{:?}", stmt))?;
                state.serialize_field("deps", deps)?;
                state.serialize_field("env", env)?;
                state.end()
            },
            &DependencyNode::Block(ref stmt, ref tree, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Block", 2)?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("env", env)?;
                //state.serialize_field("stmt", &format!("{:?}", stmt))?;
                //state.serialize_field("deps", deps)?;
                //state.serialize_field("subtree", tree)?;
                state.end()
            },
            &DependencyNode::ExprBlock(ref stmt, ref tree, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("ExprBlock", 5)?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("stmt", &format!("{:?}", stmt))?;
                state.serialize_field("deps", deps)?;
                state.serialize_field("subtree", tree)?;
                state.serialize_field("env", env)?;
                state.end()
            },
            &DependencyNode::Mac(ref stmt, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Mac", 4)?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("stmt", &format!("{:?}", stmt))?;
                state.serialize_field("deps", deps)?;
                state.serialize_field("env", env)?;
                state.end()
            },
        }
    }
}

impl DependencyNode {
    pub fn get_stmtid(&self) -> StmtID {
        match self {
            &DependencyNode::Expr(ref stmt, _, _) |
            &DependencyNode::Mac(ref stmt, _, _) |
            &DependencyNode::ExprBlock(ref stmt, _, _, _) => {
                let stmt = stmt.deref();
                stmtID!(stmt)
            },
            &DependencyNode::Block(ref stmtid, _, _, _) => stmtid.clone(),
        }
    }

    pub fn get_stmt(&self) -> Option<&P<Stmt>> {
        match self {
            &DependencyNode::Expr(ref stmt, _, _)  |
            &DependencyNode::ExprBlock(ref stmt, _, _, _) |
            &DependencyNode::Mac(ref stmt, _, _) => Some(stmt),
            &DependencyNode::Block(_, _, _, _) => None,
        }
    }

    pub fn get_deps(&self) -> Vec<usize> {
        match self {
            &DependencyNode::Expr( _, ref deps, _)  |
            &DependencyNode::Block(_, _, ref deps, _) |
            &DependencyNode::ExprBlock(_, _, ref deps, _) |
            &DependencyNode::Mac(_, ref deps, _) => deps.clone(),
        }
    }

    pub fn get_env(&self) -> &Environment {
        match self {
            &DependencyNode::Expr( _, _, ref env)  |
            &DependencyNode::Block(_, _, _, ref env) |
            &DependencyNode::ExprBlock(_, _, _, ref env) |
            &DependencyNode::Mac(_, _, ref env) => env,
        }
    }

    pub fn get_env_mut(&mut self) -> &mut Environment {
        match self {
            &mut DependencyNode::Expr( _, _, ref mut env)  |
            &mut DependencyNode::Block(_, _, _, ref mut env) |
            &mut DependencyNode::ExprBlock(_, _, _, ref mut env) |
            &mut DependencyNode::Mac(_, _, ref mut env) => env,
        }
    }

    pub fn get_deps_stmtids(&self, deptree: &DependencyTree) -> Vec<StmtID> {
        let deps = self.get_deps();
        let mut deps_stmtids = vec![];
        for dep in deps {
            let dep_node = &deptree[dep.clone()];
            deps_stmtids.push(dep_node.get_stmtid());
        }
        deps_stmtids
    }

    pub fn extract_paths(&self) -> Environment {
        match self {
            &DependencyNode::Block(_, ref nodes, _, _) => {
                let mut dep_strs = vec![];
                for subnode in nodes {
                    let subdep_strs = subnode.extract_paths();
                    dep_strs.extend(subdep_strs.0);
                }
                Environment::new(dep_strs)
            },
            &DependencyNode::Expr(ref stmt, _, _) => check_stmt(&mut vec![], stmt.deref()),
            _ => Environment::new(vec![]),
        }
    }

    pub fn encode(&self) -> EncodedDependencyNode {
        match self {
            &DependencyNode::Expr(_, ref deps, ref env) => {
                EncodedDependencyNode::Expr(self.get_stmtid(), deps.clone(), env.into_depstr())
            },
            &DependencyNode::Block(_, ref subdeptree, ref deps, ref env) => {
                let encoded_subdeptree = encode_deptree(subdeptree);
                EncodedDependencyNode::Block(self.get_stmtid(), encoded_subdeptree, deps.clone(), env.into_depstr())
            },
            &DependencyNode::ExprBlock(_, ref subdeptree, ref deps, ref env) => {
                let encoded_subdeptree = encode_deptree(subdeptree);
                EncodedDependencyNode::ExprBlock(self.get_stmtid(), encoded_subdeptree, deps.clone(), env.into_depstr())
            },
            &DependencyNode::Mac(_, ref deps, ref env) => EncodedDependencyNode::Mac(self.get_stmtid(), deps.clone(), env.into_depstr())
        }
    }
}

pub fn encode_deptree(deptree: &DependencyTree) -> EncodedDependencyTree {
    let mut encoded_deptree = vec![];
    for node in deptree {
        encoded_deptree.push(node.encode());
    }
    encoded_deptree
}


pub fn merge_dependencies(base: &mut DependencyTree, patch: &EncodedDependencyTree) {
    if base.len() != patch.len() {
        panic!("Lengths not equal: {:?} != {:?}", base, patch);
    }
    for i in 0..base.len() {
        merge_dependencies_helper(&mut base[i], &patch[i]);
    }
}

fn merge_dependencies_helper(base: &mut DependencyNode, patch: &EncodedDependencyNode) {
    // Get list of dependencies from the patch
    let (patch_deps, patch_depstr) = match patch {
        &EncodedDependencyNode::ExprBlock(_, _, ref deps, ref depstr) |
        &EncodedDependencyNode::Block(_, _, ref deps, ref depstr) |
        &EncodedDependencyNode::Expr(_, ref deps, ref depstr)  |
        &EncodedDependencyNode::Mac(_, ref deps, ref depstr) => (deps, depstr),
    };

    // Add the dependencies to the current node
    match base {
        &mut DependencyNode::ExprBlock(_, ref mut base_subtree, ref mut deps, ref mut env) => {
            deps.extend(patch_deps);
            deps.sort_unstable();
            deps.dedup();
            env.append(patch_depstr);
            // Recurse down the tree
            if let &EncodedDependencyNode::ExprBlock(_, ref patch_subtree, _, _) = patch {
                merge_dependencies(base_subtree, patch_subtree);
            } else {
                panic!("patch was not a exprblock: {:?}", patch);
            }
        },
        &mut DependencyNode::Block(_, ref mut base_subtree, ref mut deps, ref mut env) => {
            deps.extend(patch_deps);
            deps.sort_unstable();
            deps.dedup();
            env.append(patch_depstr);
            // Recurse down the tree
            if let &EncodedDependencyNode::Block(_, ref patch_subtree, _, _) = patch {
                merge_dependencies(base_subtree, patch_subtree);
            } else {
                panic!("patch was not a block: {:?}", patch);
            }
        },
        &mut DependencyNode::Expr(ref stmt, ref mut deps, ref mut env) => {
            deps.extend(patch_deps);
            deps.sort_unstable();
            deps.dedup();
            env.append(patch_depstr);
            // Check that span ids line up
            if let &EncodedDependencyNode::Expr((patch_lo, patch_hi), _, _) = patch {
                let base_lo = stmt.span.lo().0;
                let base_hi = stmt.span.hi().0;
                if base_lo != patch_lo || base_hi != patch_hi {
                    panic!("span ids do not line up: base:{} != patch:{} or base:{} != patch:{}", base_lo, patch_lo, base_hi, patch_hi);
                }
            } else {
                panic!("patch was not a expr: {:?}", patch);
            }
        },
        &mut DependencyNode::Mac(_, ref mut deps, ref mut env) => {
            deps.extend(patch_deps);
            deps.sort_unstable();
            deps.dedup();
            env.append(patch_depstr);
        }
    }
}

fn empty_block(block: &Block) -> P<Block> {
    P(Block {
        stmts: vec![],
        id: block.id,
        rules: block.rules,
        span: block.span,
        recovered: block.recovered,
    })
}

fn read_path(path: &Path) -> PathName {
    let mut output = vec![];
    for segment in &path.segments {
        output.push(segment.identifier);
    }
    output
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

        // Check to see if the expression contains a block
        let node = match expr.node {
            ExprKind::If(ref a, ref block, ref c) =>
            ExprKind::If(a.clone(), empty_block(block), c.clone()),

            ExprKind::IfLet(ref a, ref b, ref block, ref c) =>
            ExprKind::IfLet(a.clone(), b.clone(), empty_block(block), c.clone()),

            ExprKind::While(ref a, ref block, ref c) =>
            ExprKind::While(a.clone(), empty_block(block), c.clone()),

            ExprKind::WhileLet(ref a, ref b, ref block, ref c) =>
            ExprKind::WhileLet(a.clone(), b.clone(), empty_block(block), c.clone()),

            ExprKind::ForLoop(ref a, ref b, ref block, ref c) =>
            ExprKind::ForLoop(a.clone(), b.clone(), empty_block(block), c.clone()),

            ExprKind::Loop(ref block, ref c) =>
            ExprKind::Loop(empty_block(block), c.clone()),

            ExprKind::Block(ref block) =>
            ExprKind::Block(empty_block(block)),

            ExprKind::Catch(ref block) =>
            ExprKind::Catch(empty_block(block)),

            ref x => x.clone(),
        };

        let expr2 = Expr {
            id: expr.id,
            node: node,
            span: expr.span,
            attrs: expr.attrs.clone(),
        };

        output.node = match stmt.node {
            StmtKind::Expr(_) => StmtKind::Expr(P(expr2)),
            StmtKind::Semi(_) => StmtKind::Semi(P(expr2)),
            _ => panic!(),
        };
    }
    P(output)
}

pub fn analyse_block(block: &Block) -> DependencyTree {
    let (mut deptree, depstrtree) = check_block(block);

    // Examine dep_strs to find the statement indicies
    //println!("depstrtree: {:?}", depstrtree);
    for id in 1..deptree.len() { // 0 cannot have anything depending before it.
        let mut deps: Vec<usize> = vec![];
        let mut depstrs: Environment = depstrtree[id].clone();

        // find the first instance of each variable from this point (-1)
        // backwards to the start of depstrtree
        for backid in (0..id).rev() {
            let backnode: &Environment = &depstrtree[backid];
            depstrs = depstrs.into_iter().filter(|elem: &PathName| {
                if backnode.contains(elem) {
                    // Add backid into deps, and remove elem from depstrs
                    deps.push(backid);
                    return false
                }
                true
            }).collect();
        }

        // Add new deps to node
        match deptree[id] {
            DependencyNode::ExprBlock(_, _,ref mut l, ref mut env) |
            DependencyNode::Block(_, _,ref mut l, ref mut env) |
            DependencyNode::Expr(_,ref mut l, ref mut env) => {
                l.append(&mut deps);
                l.sort_unstable();
                l.dedup();
                env.merge(depstrs);
            },
            DependencyNode::Mac(_, _, _) => assert!(deps.len() == 0), // Has no dependencies
        }

    }

    deptree
}

fn check_block(block: &Block) -> (DependencyTree, Vec<Environment>) {
    let mut deptree: DependencyTree = vec![];
    let mut depstrtree: Vec<Environment> = vec![];
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

fn check_stmt(deptree: &mut DependencyTree, stmt: &Stmt) -> Environment {
    match stmt.node {
        // A local let ?
        StmtKind::Local(ref local) => {
            if let Some(ref expr) = local.init {
                deptree.push(DependencyNode::Expr(remove_blocks(&stmt), vec![], Environment::empty()));
                let node_id = deptree.len() - 1;
                let mut dep_strs = check_expr(deptree, &expr.deref(), node_id);
                if let PatKind::Ident(ref mode, ref ident, ref mpat) = local.pat.deref().node {
                    // TODO: Use other two unused variables
                    dep_strs.push(vec![ident.node]);
                } else {
                    // Managed to get something other than Ident
                    panic!("local.pat: {:?}", local.pat);
                }
                deptree[node_id].get_env_mut().merge(dep_strs.clone());
                dep_strs
            } else {
                Environment::empty()
            }
        },

        // A line in a function
        StmtKind::Expr(ref expr) |
        StmtKind::Semi(ref expr) => {
            deptree.push(DependencyNode::Expr(remove_blocks(&stmt), vec![], Environment::empty()));
            let node_id = deptree.len() - 1;
            let dep_strs = check_expr(deptree, &expr.deref(), node_id);
            deptree[node_id].get_env_mut().merge(dep_strs.clone());
            dep_strs
        },


        StmtKind::Item(ref item) => {
            println!("ITEM: {:?}", item);
            Environment::empty()
        },

        // Macros should be expanded by this point
        StmtKind::Mac(ref mac) => {
            deptree.push(DependencyNode::Mac(P(stmt.clone()), vec![], Environment::empty()));
            Environment::empty()
        },
    }
}

fn check_expr(deptree: &mut DependencyTree, expr: &Expr, node_id: usize) -> Environment {
    let mut dependencies = vec![];
    let subexprs: Vec<P<Expr>> = {
        match expr.node {
            ExprKind::Box(ref expr1) |
            ExprKind::Unary(_, ref expr1) |
            ExprKind::Cast(ref expr1, _) |
            ExprKind::Type(ref expr1, _) |
            ExprKind::Field(ref expr1, _) |
            ExprKind::TupField(ref expr1, _) |
            ExprKind::AddrOf(_, ref expr1) |
            ExprKind::Paren(ref expr1) |
            ExprKind::Try(ref expr1) => vec![expr1.clone()],

            ExprKind::InPlace(ref expr1, ref expr2) |
            ExprKind::Binary(_, ref expr1, ref expr2) |
            ExprKind::Assign(ref expr1, ref expr2) |
            ExprKind::AssignOp(_, ref expr1, ref expr2) |
            ExprKind::Index(ref expr1, ref expr2) |
            ExprKind::Repeat(ref expr1, ref expr2) => vec![expr1.clone(), expr2.clone()],

            ExprKind::Array(ref exprl) |
            ExprKind::Tup(ref exprl)   |
            ExprKind::MethodCall(_, ref exprl) => exprl.clone(),

            ExprKind::Call(ref expr1, ref exprl) => {
                 // TODO: expr1 resolves to a method name
                 // Should check whether method is safe/independent?
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
                let subdeptree = analyse_block(block1);
                deptree.push(DependencyNode::Block(stmtID!(block1), subdeptree, vec![node_id], Environment::empty()));
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
                let subdeptree = analyse_block(block1);
                deptree.push(DependencyNode::Block(stmtID!(block1), subdeptree, vec![node_id], Environment::empty()));
                // TODO: Examine subdeptree for external dependencies and update vec![node_id]
                vec![expr1.clone()]
            },

            ExprKind::Loop(ref block1, _) |
            ExprKind::Block(ref block1) |
            ExprKind::Catch(ref block1) => {
                let subdeptree = analyse_block(block1);
                deptree.push(DependencyNode::Block(stmtID!(block1), subdeptree, vec![node_id], Environment::empty()));
                // TODO: Examine subdeptree for external dependencies and update vec![node_id]
                vec![]
            },

            ExprKind::Match(ref expr1, ref arml) => {
                let mut exprs = vec![expr1.clone()];
                for arm in arml.deref() {
                    exprs.push(arm.body.clone());
                    if let Some(ref guard) = arm.guard {
                        exprs.push(guard.clone());
                    }
                }
                exprs
            },

            ExprKind::Closure(_, _, ref fndecl, ref expr1, _) => {
                // TODO: Use fndecl
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
                dependencies.push(read_path(path));
                vec![]
            },


            // Unused expressions, panic if used
            _ => vec![],
        }
    };

    // Create list of stuff that is touched
    for subexpr in &subexprs {
        dependencies.extend(check_expr(deptree, subexpr, node_id).0);
    }

    // Return our dependency list to include those statements
    Environment::new(dependencies)
}
