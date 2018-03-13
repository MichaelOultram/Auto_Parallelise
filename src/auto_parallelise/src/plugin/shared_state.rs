use serde_json;

use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

use utils;
use parallel_stages::dependency_analysis::StmtID;

#[derive(Clone, Serialize, Deserialize)]
pub struct AutoParallelise {
    pub compiler_stage: CompilerStage,
    pub linter_level: u32, // Used to determine when linter has finished
    pub functions: Vec<Function>,
    pub config: Config,
}

#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Debug)]
pub enum CompilerStage {
    Analysis, // First Stage
    Modification, // Second Stage
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Function {
    // Function Identifier
    pub ident_name: String, // TODO: Need to fully qualify name
    pub ident_ctxt: String,

    pub output_type: Option<String>, // TODO: Remove?

    // Used to determine if entire function call can be parallelised.
    pub is_unsafe: bool,
    pub called_functions: Vec<String>,
    pub input_types: Vec<String>,

    pub encoded_deptree: EncodedDependencyTree,
}


// Depencency Tree
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

#[derive(Clone, Serialize, Deserialize)]
pub struct Config {
    pub enabled: bool,
}
impl Config {
    pub fn default() -> Self {
        Config {
            enabled: true,
        }
    }

    pub fn save(&self, path: &Path) {
        // Try to convert the object to json
        let obj_json = match serde_json::to_string_pretty(&self) {
            Ok(obj) => obj,
            Err(why) => panic!("Unable to convert Config to JSON: {}", why),
        };

        utils::write_file(path, &obj_json);
    }
}
