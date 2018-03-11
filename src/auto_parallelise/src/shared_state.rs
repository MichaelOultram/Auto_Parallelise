use dependency_analysis::EncodedDependencyTree;

#[derive(Clone, Serialize, Deserialize)]
pub struct AutoParallelise {
    pub compiler_stage: CompilerStage,
    pub linter_level: u32, // Used to determine when linter has finished
    pub functions: Vec<Function>,
    pub enabled: bool,
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
