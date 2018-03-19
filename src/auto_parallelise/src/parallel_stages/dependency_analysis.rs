use syntax::ast::{Block, Stmt, Ident};
use syntax::ptr::P;
use std::ops::Deref;
use serde::ser::{Serialize, Serializer, SerializeStruct, SerializeSeq};
use std::{vec, iter};
use syntax::print::pprust;
use syntax_pos::symbol::Symbol;
use syntax_pos::hygiene::Mark;

use parallel_stages::deconstructor;
use plugin::shared_state::{EncodedDependencyNode, EncodedDependencyTree, EncodedEnvironment};

pub type PathName = Vec<Ident>;
pub type StmtID = (u32, u32);

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
    pub fn clear(&mut self) {self.0.clear()}
    pub fn get(&self, i: usize) -> Option<&PathName> {self.0.get(i)}
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
                    //eprintln!("contains: {:?} == {:?}", target_elem, elem);
                    return true;
                }
            }
        }
        //eprintln!("contains: {:?} != {:?}", target_elem, self.0);
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

impl Serialize for DependencyNode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where S: Serializer
    {
        match self {
            &DependencyNode::Expr(ref stmt, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Expr", 5)?;
                state.serialize_field("type", "Expr")?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("stmt", &pprust::stmt_to_string(stmt))?;
                state.serialize_field("deps", deps)?;
                state.serialize_field("env", env)?;
                state.end()
            },
            &DependencyNode::Block(ref stmt, ref tree, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Block", 4)?;
                state.serialize_field("type", "Block")?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("subtree", tree)?;
                state.serialize_field("env", env)?;
                //state.serialize_field("stmt", &format!("{:?}", stmt))?;
                //state.serialize_field("deps", deps)?;
                state.end()
            },
            &DependencyNode::ExprBlock(ref stmt, ref tree, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("ExprBlock", 6)?;
                state.serialize_field("type", "ExprBlock")?;
                state.serialize_field("stmtid", &format!("{:?}", self.get_stmtid()))?;
                state.serialize_field("stmt", &pprust::stmt_to_string(stmt))?;
                state.serialize_field("deps", deps)?;
                state.serialize_field("subtree", tree)?;
                state.serialize_field("env", env)?;
                state.end()
            },
            &DependencyNode::Mac(ref stmt, ref deps, ref env) => {
                let mut state = serializer.serialize_struct("Mac", 5)?;
                state.serialize_field("type", "Mac")?;
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
            &DependencyNode::Expr(ref stmt, _, _) => deconstructor::check_stmt(&mut vec![], stmt.deref()),
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


pub fn replace_dependencies(base: &mut DependencyTree, patch: &EncodedDependencyTree) {
    if base.len() != patch.len() {
        panic!("Lengths not equal: {:?} != {:?}", base, patch);
    }
    for i in 0..base.len() {
        replace_dependencies_helper(&mut base[i], &patch[i]);
    }
}

fn replace_dependencies_helper(base: &mut DependencyNode, patch: &EncodedDependencyNode) {
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
            // Replacement instead of merge
            deps.clear();
            env.0.clear();
            env.1.clear();
            deps.extend(patch_deps);
            deps.sort_unstable();
            deps.dedup();
            env.0.append(patch_inenv);
            env.1.append(patch_outenv);
            // Recurse down the tree
            if let &EncodedDependencyNode::ExprBlock(_, ref patch_subtree, _, _) = patch {
                replace_dependencies(base_subtree, patch_subtree);
            } else {
                panic!("patch was not a exprblock: {:?}", patch);
            }
        },
        &mut DependencyNode::Block(_, ref mut base_subtree, ref mut deps, ref mut env) => {
            // Replacement insdeps.clear();tead of merge
            deps.clear();
            env.0.clear();
            env.1.clear();
            deps.extend(patch_deps);
            deps.sort_unstable();
            deps.dedup();
            env.0.append(patch_inenv);
            env.1.append(patch_outenv);
            // Recurse down the tree
            if let &EncodedDependencyNode::Block(_, ref patch_subtree, _, _) = patch {
                replace_dependencies(base_subtree, patch_subtree);
            } else {
                panic!("patch was not a block: {:?}", patch);
            }
        },
        &mut DependencyNode::Expr(ref stmt, ref mut deps, ref mut env) => {
            // Replacement instead of merge
            deps.clear();
            env.0.clear();
            env.1.clear();
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
            // Replacement instead of merge
            deps.clear();
            env.0.clear();
            env.1.clear();
            deps.extend(patch_deps);
            deps.sort_unstable();
            deps.dedup();
            env.0.append(patch_inenv);
            env.1.append(patch_outenv);
        }
    }
}

pub fn analyse_block(block: &Block) -> DependencyTree {
    let (deptree, _) = analyse_block_with_env(block);
    deptree
}

pub fn analyse_block_with_env(block: &Block) -> (DependencyTree, InOutEnvironment) {
    let (mut deptree, depstrtree) = deconstructor::check_block(block);
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
