extern crate rand;
#[macro_use] extern crate lazy_static;

use rand::{Rng, thread_rng};

type Block = Vec<StmtKind>;
fn block_to_string(block: &Block) -> String {
    let mut output = "{\n".to_owned();
    for stmt in block {
        output.push_str(&stmt.to_string());
        output.push_str("\n");
    }
    output.push_str("}");
    output
}

lazy_static! {
    static ref ALPHABET: Vec<&'static str> = "a b c d e f g h i j k l m n o p q r s t u v w x y z".split_whitespace().collect();

    static ref STMT_KIND_LIST: Vec<&'static str> = vec!["NewVar", "Assignment", "ForRange", "ForList", "IfLtElse", "Push", "Pop", "Print"];

    static ref EXPR_KIND_LIST: Vec<&'static str> = vec!["Value", "StdinValue", "Var", "Add", "Mul", "Sub"];
}

static EXPR_HEIGHT: u32 = 3;

#[derive(Clone)]
enum Variable {
    Int(String),
    List(String),
}

impl Variable {
    fn to_string(&self) -> String {
        match self {
            &Variable::Int(ref var) |
            &Variable::List(ref var) => var.clone(),
        }
    }
}

enum ExprKind{
    Value(i32),
    StdinValue(usize),
    Var(Variable),
    Add(Box<ExprKind>, Box<ExprKind>),
    Mul(Box<ExprKind>, Box<ExprKind>),
    Sub(Box<ExprKind>, Box<ExprKind>),
}

impl ExprKind {
    fn generate<'a>(height: u32, env: &Vec<Variable>) -> Box<Self> {
        let mut rng = thread_rng();
        if height == 0 {
            return Box::new(ExprKind::Value(rng.gen_range(-10, 10)))
        }
        loop {
            let exprid = rng.choose(&EXPR_KIND_LIST).unwrap();
            match exprid {
                &"Value" => return Box::new(ExprKind::Value(rng.gen_range(-10, 10))),
                &"StdinValue" => return Box::new(ExprKind::StdinValue(rng.gen_range(0, 20))),
                &"Add" => {
                    let a = ExprKind::generate(height-1, env);
                    let b = ExprKind::generate(height-1, env);
                    return Box::new(ExprKind::Add(a, b));
                }
                &"Mul" => {
                    let a = ExprKind::generate(height-1, env);
                    let b = ExprKind::generate(height-1, env);
                    return Box::new(ExprKind::Mul(a, b));
                }
                &"Sub" => {
                    let a = ExprKind::generate(height-1, env);
                    let b = ExprKind::generate(height-1, env);
                    return Box::new(ExprKind::Sub(a, b));
                }
                &"Var" => {
                    if let Some(var) = rng.choose(&env) {
                        return Box::new(ExprKind::Var(var.clone()));
                    }
                }
                &&_ => panic!("Not found in EXPR_KIND_LIST: {}", exprid),
            }
        }
    }

    fn to_string(&self) -> String {
        match self {
            &ExprKind::Value(val) => format!("{}", val),
            &ExprKind::StdinValue(id) => format!("stdin[{}]", id),
            &ExprKind::Var(ref var) => match var {
                &Variable::Int(ref varname) => varname.clone(),
                &Variable::List(ref varname) => format!("{}.pop().unwrap_or(0)", varname),
            },
            &ExprKind::Add(ref a, ref b) => format!("({}) + ({})", a.to_string(), b.to_string()),
            &ExprKind::Mul(ref a, ref b) => format!("({}) * ({})", a.to_string(), b.to_string()),
            &ExprKind::Sub(ref a, ref b) => format!("({}) - ({})", a.to_string(), b.to_string()),
            _ => unimplemented!(),
        }
    }
}



enum StmtKind {
    NewVar(Variable),
    Assignment(Variable, Box<ExprKind>),
    ForRange(Variable, Box<ExprKind>, Box<ExprKind>, Block),
    ForList(Variable, Variable, Block),
    IfLtElse(Box<ExprKind>, Box<ExprKind>, Block, Block),

    Push(Variable, Box<ExprKind>),
    Pop(Variable),

    Print(Box<ExprKind>),
}

impl StmtKind {
    pub fn to_string(&self) -> String {
        match self {
            &StmtKind::NewVar(ref var) => match var {
                &Variable::Int(ref varname) => format!("let mut {}: i32 = 0;", varname),
                &Variable::List(ref varname) => format!("let mut {}: Vec<i32> = vec![0];", varname),
            },

            &StmtKind::Assignment(ref var, ref value) =>
            format!("{} = {};", var.to_string(), value.to_string()),

            &StmtKind::ForRange(ref var, ref from, ref to, ref inner_block) =>
            format!("for {} in 0.max({})..100.min({}) {{\n{}\n}}", var.to_string(), from.to_string(), to.to_string(), block_to_string(inner_block)),

            &StmtKind::ForList(ref var, ref list, ref inner_block) =>
            format!("for {} in {}.clone() {{\n{}\n}}", var.to_string(), list.to_string(), block_to_string(inner_block)),

            &StmtKind::IfLtElse(ref a, ref b, ref true_block, ref false_block) =>
            //format!("if ({}) < ({}) {} else {}", a.to_string(), b.to_string(),
            format!("if ({}) < ({}) {}", a.to_string(), b.to_string(), block_to_string(true_block)),
            //block_to_string(false_block)),

            &StmtKind::Push(ref var, ref value) =>
            format!("{}.push({});", var.to_string(), value.to_string()),

            &StmtKind::Pop(ref var) =>
            format!("{}.pop();", var.to_string()),

            &StmtKind::Print(ref val) => {
                let val_str = val.to_string();
                format!("println!(\"{} = {{:?}}\", {});", val_str, val_str)
            },
        }
    }
}

fn generate_block(num_statements: usize, external_env: &Vec<Variable>) -> Block {
    let mut env: Vec<Variable> = external_env.clone();
    let mut program: Block = vec![];
    let mut rng = thread_rng();
    while program.len() < num_statements {
        match rng.choose(&STMT_KIND_LIST).unwrap() {
            &"NewVar" => {
                let varname: String = rng.choose(&ALPHABET).unwrap().to_string();
                let var = if rng.gen() {
                    Variable::Int(varname.clone())
                } else {
                    Variable::List(varname.clone())
                };
                env.retain(|v| v.to_string() != varname);
                env.push(var.clone());
                program.push(StmtKind::NewVar(var));
            },
            &"Assignment" => {
                let var = rng.choose(&env);
                if let Some(&Variable::Int(_)) = var {
                    program.push(StmtKind::Assignment(var.unwrap().clone(), ExprKind::generate(EXPR_HEIGHT, &env)));
                }
            },
            &"ForRange" => {
                let varname: String = rng.choose(&ALPHABET).unwrap().to_string();
                let iter_var = Variable::Int(varname);
                let from = ExprKind::generate(EXPR_HEIGHT, &env);
                let to = ExprKind::generate(EXPR_HEIGHT, &env);
                let inner_block = generate_block(num_statements / 2, &env);
                program.push(StmtKind::ForRange(iter_var, from, to, inner_block));
            },
            &"ForList" => {
                let varname: String = rng.choose(&ALPHABET).unwrap().to_string();
                let iter_var = Variable::Int(varname);
                let list_var = rng.choose(&env);
                if let Some(&Variable::List(_)) = list_var {
                    let inner_block = generate_block(num_statements / 2, &env);
                    program.push(StmtKind::ForList(iter_var, list_var.unwrap().clone(), inner_block));
                }
            },
            &"IfLtElse" => {
                let a = ExprKind::generate(EXPR_HEIGHT, &env);
                let b = ExprKind::generate(EXPR_HEIGHT, &env);
                let block_a = generate_block(num_statements / 2, &env);
                let block_b = generate_block(num_statements / 2, &env);
                program.push(StmtKind::IfLtElse(a, b, block_a, block_b));
            },
            &"Push" => {
                let list_var = rng.choose(&env);
                if let Some(&Variable::List(_)) = list_var {
                    let val = ExprKind::generate(EXPR_HEIGHT, &env);
                    program.push(StmtKind::Push(list_var.unwrap().clone(), val));
                }
            },
            &"Pop" => {
                let list_var = rng.choose(&env);
                if let Some(&Variable::List(_)) = list_var {
                    program.push(StmtKind::Pop(list_var.unwrap().clone()));
                }
            },
            &"Print" => {
                let val = ExprKind::generate(EXPR_HEIGHT, &env);
                program.push(StmtKind::Print(val));
            },
            &&_ => panic!("Not found in STMT_KIND_LIST"),
        }
    }
    for var in env {
        program.push(StmtKind::Print(Box::new(ExprKind::Var(var))));
    }
    program
}

fn main() {
    let num_statements = ::std::env::args().last().unwrap().parse().unwrap();
    let inner_block = block_to_string(&generate_block(num_statements, &vec![]));
    let source_code = format!("
#![feature(plugin)]
#![plugin(auto_parallelise)]

#[autoparallelise]
fn main() {{
let stdin: Vec<i32> = ::std::env::args().skip(1).map(|i| i.parse::<i32>().unwrap()).collect();
{}
}}", inner_block);

    println!("{}", source_code);
}
