use syntax::ast::{Block, Expr, ExprKind, Stmt, StmtKind, Path, PatKind, Ident, PathSegment};
use syntax::ptr::P;
use std::ops::Deref;
use serde::ser::{Serialize, Serializer, SerializeStruct, SerializeSeq};
use std::{vec, iter};
use syntax::print::pprust;
use syntax_pos::Span;
use syntax_pos::symbol::Symbol;
use syntax_pos::hygiene::{SyntaxContext, Mark};
use serialize::{Decodable, Decoder, Encodable, Encoder};

pub type PathName = Vec<Ident>;
pub type StmtID = (u32, u32);
macro_rules! stmtID {
    ($i:ident) => {{($i.span.lo().0, $i.span.hi().0)}}
}

#[derive(Clone, Debug, PartialEq)]
pub struct Environment(Vec<PathName>);
pub type InOutEnvironment = (Environment, Environment);
impl Environment {
    pub fn new(dep_str: Vec<PathName>) -> Self {
        let mut env = Environment{0: dep_str};
        env.append(&vec![]);
        env
    }
    pub fn empty() -> Self {Environment{0:vec![]}}
    pub fn into_depstr(&self) -> EncodedEnvironment {
        let mut depstr = vec![];
        for e in &self.0 {
            let pathstr: Vec<(String, Vec<u32>)> = e.into_iter().map(|i| {
                let mut string = "".to_owned();
                string.push_str(&*i.name.as_str());
                //let ctxt_str = format!("{:?}", i.ctxt);
                let mark_vec = i.ctxt.marks();
                let mut mark_u32_vec = vec![];
                for mark in mark_vec {
                    mark_u32_vec.push(mark.modern().as_u32());
                }
                (string, mark_u32_vec)
            }).collect();
            depstr.push(pathstr);
        }
        depstr.sort_unstable();
        depstr.dedup();
        //eprintln!("into_depstr: {:?} -> {:?}", self.0, depstr);
        depstr
    }
    pub fn append(&mut self, patch: &EncodedEnvironment) {
        let mut env = self.into_depstr();
        env.append(&mut patch.clone());
        env.sort_unstable();
        env.dedup();
        self.0.clear();
        for p in env {
            let path: PathName = p.clone().into_iter().map(|(string, marks_u32)| {
                let mut ident = Ident::with_empty_ctxt(Symbol::gensym(&string[0..]));
                for mark_u32 in marks_u32 {
                    let mark = Mark::from_u32(mark_u32);
                    ident.ctxt = ident.ctxt.apply_mark(mark);
                }
                ident
            }).collect();
            //eprintln!("append: {:?} -> {:?}", p, path);
            self.0.push(path);
        }
    }    pub fn merge(&mut self, patch: Environment) {
        for p in patch.0 {
            self.0.push(p);
        }
        self.append(&vec![]);
    }
    pub fn into_iter(self) -> vec::IntoIter<Vec<Ident>> {self.0.into_iter()}
    pub fn push(&mut self, elem: PathName) {self.0.push(elem)}
    pub fn contains(&self, target_elem: &PathName) -> bool {
        for elem in &self.0 {
            if elem.len() == target_elem.len() {
                let mut is_equal = true;
                for i in 0..elem.len() {
                    let elem_section = elem[i].name.as_str();
                    let target_elem_section = target_elem[i].name.as_str();
                    if elem_section != target_elem_section {
                        is_equal = false;
                        break;
                    }
                }
                if is_equal {
                    eprintln!("contains: {:?} == {:?}", target_elem, elem);
                    return true;
                }
            }
        }
        eprintln!("contains: {:?} != {:?}", target_elem, self.0);
        false
    }
    pub fn len(&self) -> usize {self.0.len()}
    /*pub fn remove(&mut self, elems: Vec<PathName>) {
        self.0.retain(|elem| !elems.contains(&elem));
    }*/
    pub fn remove_env(&mut self, elems: Environment) {
        self.0.retain(|elem| !elems.contains(&elem));
    }
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
    Expr(P<Stmt>, Vec<usize>, InOutEnvironment), // Statement and Dependency indicies
    Block(StmtID, DependencyTree, Vec<usize>, InOutEnvironment),
    ExprBlock(P<Stmt>, DependencyTree, Vec<usize>, InOutEnvironment),
    Mac(P<Stmt>, Vec<usize>, InOutEnvironment)
}
pub type DependencyTree = Vec<DependencyNode>;

// Used to store as JSON
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum EncodedDependencyNode {
    Expr(StmtID, Vec<usize>, EncodedInOutEnvironment), // Statement ID and Dependency indicies
    Block(StmtID, EncodedDependencyTree, Vec<usize>, EncodedInOutEnvironment),
    ExprBlock(StmtID, EncodedDependencyTree, Vec<usize>, EncodedInOutEnvironment),
    Mac(StmtID, Vec<usize>, EncodedInOutEnvironment)
}
pub type EncodedDependencyTree = Vec<EncodedDependencyNode>;
pub type EncodedEnvironment = Vec<Vec<(String, Vec<u32>)>>;
pub type EncodedInOutEnvironment = (EncodedEnvironment, EncodedEnvironment);

impl Serialize for DependencyNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match self {
            &DependencyNode::Expr(ref stmt, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Expr", 4)?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("stmt", &pprust::stmt_to_string(stmt))?;
                state.serialize_field("deps", deps)?;
                state.serialize_field("env", env)?;
                state.end()
            },
            &DependencyNode::Block(ref stmt, ref tree, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Block", 3)?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("subtree", tree)?;
                state.serialize_field("env", env)?;
                //state.serialize_field("stmt", &format!("{:?}", stmt))?;
                //state.serialize_field("deps", deps)?;
                state.end()
            },
            &DependencyNode::ExprBlock(ref stmt, ref tree, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("ExprBlock", 5)?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("stmt", &pprust::stmt_to_string(stmt))?;
                state.serialize_field("deps", deps)?;
                state.serialize_field("subtree", tree)?;
                state.serialize_field("env", env)?;
                state.end()
            },
            &DependencyNode::Mac(ref stmt, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Mac", 4)?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("stmt", &pprust::stmt_to_string(stmt))?;
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

    pub fn get_env(&self) -> &InOutEnvironment {
        match self {
            &DependencyNode::Expr( _, _, ref env)  |
            &DependencyNode::Block(_, _, _, ref env) |
            &DependencyNode::ExprBlock(_, _, _, ref env) |
            &DependencyNode::Mac(_, _, ref env) => env,
        }
    }

    pub fn get_env_mut(&mut self) -> &mut InOutEnvironment {
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

    pub fn extract_paths(&self) -> InOutEnvironment {
        match self {
            &DependencyNode::Block(_, ref nodes, _, _) => {
                let mut inenv = vec![];
                let mut outenv = vec![];
                for subnode in nodes {
                    let (subin, subout) = subnode.extract_paths();
                    inenv.extend(subin.0);
                    outenv.extend(subout.0);
                }
                (Environment::new(inenv), Environment::new(outenv))
            },
            &DependencyNode::Expr(ref stmt, _, _) => check_stmt(&mut vec![], stmt.deref()),
            _ => (Environment::new(vec![]), Environment::new(vec![]))
        }
    }

    pub fn encode(&self) -> EncodedDependencyNode {
        let &(ref inenv, ref outenv) = self.get_env();
        let encoded_inoutenv = (inenv.into_depstr(), outenv.into_depstr());
        match self {
            &DependencyNode::Expr(_, ref deps, ref env) => {
                EncodedDependencyNode::Expr(self.get_stmtid(), deps.clone(), encoded_inoutenv)
            },
            &DependencyNode::Block(_, ref subdeptree, ref deps, ref env) => {
                let encoded_subdeptree = encode_deptree(subdeptree);
                EncodedDependencyNode::Block(self.get_stmtid(), encoded_subdeptree, deps.clone(), encoded_inoutenv)
            },
            &DependencyNode::ExprBlock(_, ref subdeptree, ref deps, ref env) => {
                let encoded_subdeptree = encode_deptree(subdeptree);
                EncodedDependencyNode::ExprBlock(self.get_stmtid(), encoded_subdeptree, deps.clone(), encoded_inoutenv)
            },
            &DependencyNode::Mac(_, ref deps, ref env) => EncodedDependencyNode::Mac(self.get_stmtid(), deps.clone(), encoded_inoutenv)
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
    let (patch_deps, &(ref patch_inenv, ref patch_outenv)) = match patch {
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
            env.0.append(patch_inenv);
            env.1.append(patch_outenv);
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
            env.0.append(patch_inenv);
            env.1.append(patch_outenv);
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
            env.0.append(patch_inenv);
            env.1.append(patch_outenv);
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
            env.0.append(patch_inenv);
            env.1.append(patch_outenv);
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


    let mut segments = vec![];
    for segment_ident in &output {
        let segment = PathSegment::from_ident(segment_ident.clone(), Span::default());
        segments.push(segment);
    }
    let var_name = Path {
        span: Span::default(),
        segments: segments,
    };
    //eprintln!("read_path: {} -> {:?} -> {}", pprust::path_to_string(path), output, pprust::path_to_string(&var_name));
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

pub fn analyse_block(block: &Block) -> DependencyTree {
    let (deptree, _) = analyse_block_with_env(block);
    deptree
}

fn analyse_block_with_env(block: &Block) -> (DependencyTree, InOutEnvironment) {
    let (mut deptree, depstrtree) = check_block(block);
    let mut inenv = Environment::empty();
    let mut outenv = Environment::empty();

    // Examine dep_strs to find the statement indicies
    //eprintln!("depstrtree: {:?}", depstrtree);
    for id in 0..deptree.len() {
        let mut deps: Vec<usize> = vec![];
        let (mut depin, mut depout) = depstrtree[id].clone();

        // This node consumes some of the outenv, and adds new parts of the outenv
        outenv.remove_env(depin.clone());
        outenv.merge(depout.clone());

        // find the first instance of each variable from this point (-1)
        // backwards to the start of depstrtree
        for backid in (0..id).rev() {
            let &(ref backin, ref backout) = &depstrtree[backid];
            depin = depin.into_iter().filter(|elem: &PathName| {
                if backout.contains(elem) {
                    // Add backid into deps, and remove elem from depstrs
                    deps.push(backid);
                    return false
                } else if backin.contains(elem) {
                    panic!("{:?} consumes {:?} without releasing. Unable to satisfy {:?}", deptree[backid], elem, deptree[id]);
                }
                true
            }).collect();
        }

        // Any unresolved dependencies require an external dependency
        inenv.merge(depin.clone());


        // Add new deps to node
        match deptree[id] {
            DependencyNode::ExprBlock(_, _,ref mut l, ref mut env) |
            DependencyNode::Block(_, _,ref mut l, ref mut env) |
            DependencyNode::Expr(_,ref mut l, ref mut env) => {
                l.append(&mut deps);
                l.sort_unstable();
                l.dedup();
                // env.merge(depstrs);
            },
            DependencyNode::Mac(_, _, _) => assert!(deps.len() == 0), // Has no dependencies
        }

    }

    (deptree, (inenv, outenv))
}

fn check_block(block: &Block) -> (DependencyTree, Vec<InOutEnvironment>) {
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

fn check_stmt(deptree: &mut DependencyTree, stmt: &Stmt) -> InOutEnvironment {
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

fn check_expr(sub_blocks: &mut DependencyTree, expr: &Expr) -> InOutEnvironment {
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
                dependencies.extend(subinenv.0);
                produces.extend(suboutenv.0);
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
                dependencies.extend(subinenv.0);
                produces.extend(suboutenv.0);
                sub_blocks.push(DependencyNode::Block(stmtID!(block1), subdeptree, vec![], sub_env));
                vec![expr1.clone()]
            },

            ExprKind::Loop(ref block1, _) |
            ExprKind::Block(ref block1) |
            ExprKind::Catch(ref block1) => {
                let (subdeptree, sub_env) = analyse_block_with_env(block1);
                let (subinenv, suboutenv) = sub_env.clone();
                dependencies.extend(subinenv.0);
                produces.extend(suboutenv.0);
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

                    dependencies.extend(bodyinenv.0.clone());
                    produces.extend(bodyinenv.0); // Naively assume that we release all dependencies
                    produces.extend(bodyoutenv.0); // TODO: Does bodyoutenv make sense (it is probably empty)

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
                dependencies.push(read_path(path));
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
        dependencies.extend(subinenv.0.clone());
        produces.extend(subinenv.0); // Naively assume that we release all dependencies
        produces.extend(suboutenv.0);
    }

    // Return our dependency list to include those statements
    (Environment::new(dependencies), Environment::new(produces))
}

fn check_pattern(sub_blocks: &mut DependencyTree, patkind: &PatKind) -> Environment {
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
            env.push(read_path(path));
            for fieldpat in fieldpats {
                env.push(vec![fieldpat.node.ident]);
            }
        },

        &PatKind::TupleStruct(ref path, ref pats, _) => {
            env.push(read_path(path));
            for pat in pats {
                env.merge(check_pattern(sub_blocks, &pat.node));
            }
        },

        &PatKind::Path(_, ref path) => env.push(read_path(path)),

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
