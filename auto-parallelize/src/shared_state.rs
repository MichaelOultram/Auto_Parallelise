#[derive(Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
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
