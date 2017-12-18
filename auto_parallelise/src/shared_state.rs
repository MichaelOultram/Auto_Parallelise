use syntax::ast::{Stmt, Expr};
use syntax::ptr::P;

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum CompilerStage {
    Analysis, // First Stage
    Modification, // Second Stage
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Function {
    // TODO: Need to fully qualify name
    pub ident_name: String,
    pub ident_ctxt: String,
    pub input_types: Vec<String>,
    pub output_type: Option<String>,
}

pub enum DependencyNode {
    Expr(P<Stmt>, Vec<u32>), // Statement and Dependency indicies
    Block(Option<P<Stmt>>, Vec<u32>, DependencyTree),
}
pub type DependencyTree = Vec<DependencyNode>;
