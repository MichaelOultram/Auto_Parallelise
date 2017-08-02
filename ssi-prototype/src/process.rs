use serde_json;
use std::cmp;
use std::collections::VecDeque;
use rand;
use rand::StdRng;
use statrs::distribution::{Binomial, Distribution};

static MIN_CYCLES : u32 = 100;
static MAX_CYCLES : u32 = 1000;
static MIN_INSTRUCTIONS : usize = 25;
static MAX_INSTRUCTIONS : usize = 100;
static MAX_CHILD_PROCESSESS : u64 = 25;
static CHILD_BRANCH_RATE : f64 = 0.75;

#[derive(Clone, Serialize)]
pub enum Instruction {
    CPU(u32),
    IO(u32), // TODO: Which machine's IO?
    Spawn(Process),
}


pub type Program = VecDeque<Instruction>;

pub fn generate_program<R : rand::Rng>(rng : &mut R, min_extra : usize, max_extra : usize, child_processes : Vec<Process>) -> Program {
    // Convert child processess into Spawn instructions
    // Add these instructions to our program
    let num_children = child_processes.len();
    let mut program : Vec<Instruction> = child_processes.into_iter().map(|p| Instruction::Spawn(p)).collect();
    assert_eq!(num_children, program.len());

    // Pad the process with extra CPU and IO instructions
    for _ in 0..rng.gen_range(min_extra, max_extra) {
        let cycles = rng.gen_range(MIN_CYCLES, MAX_CYCLES);

        // 50:50 chance of CPU or IO instruction
        let instruction : Instruction;
        if rng.gen() {
            instruction = Instruction::CPU(cycles);
        } else {
            instruction = Instruction::IO(cycles);
        }

        program.push(instruction);
    }

    // Shuffle the instruction order in the program
    rng.shuffle(&mut program);
    program.into()
}

/// The status of a context - used for scheduling
/// See syscall::process::waitpid and the sync module for examples of usage
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub enum Status {
    Runnable,
    Blocked,
    Exited,
}

#[derive(Clone, Serialize)]
pub struct Process {
    pub name : String,
    pub status : Status,
    pub program : Program,
}

impl Process {
    pub fn new(name: String) -> Self {
        Process {
            name: name,
            status: Status::Blocked,
            program: VecDeque::new(),
        }
    }

    pub fn generate_process_tree(rng : &mut rand::StdRng, amount_of_children : u64, dict : &Vec<String>) -> Self {
        let word = rand::sample(rng, dict, 1)[0];
        let mut process = Process::new(word.clone());
        let mut children : Vec<Process> = vec![];

        let mut resources = amount_of_children;
        while resources > 0 {
            resources -= 1;
            let n = Binomial::new(CHILD_BRANCH_RATE, MAX_CHILD_PROCESSESS).unwrap();
            let num_sub_children = cmp::min(n.sample::<StdRng>(rng) as u64, resources);

            children.push(Self::generate_process_tree(rng, num_sub_children, dict));
            resources -= num_sub_children;
        }

        process.program = generate_program(rng, MIN_INSTRUCTIONS, MAX_INSTRUCTIONS, children);

        process
    }

    pub fn to_string(&self) -> String {
        format!("[{}: {:?}]", self.name, self.status)
    }

    pub fn to_json(&self) -> String {
        match serde_json::to_string_pretty(&self) {
            Ok(s) => s,
            Err(e) => panic!(e),
        }
    }

}
