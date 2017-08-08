use serde_json;
use std::cmp;
use std::collections::VecDeque;
use rand;
use rand::StdRng;
use statrs::distribution::{Binomial, Distribution};

const MIN_CYCLES : u32 = 100;
const MAX_CYCLES : u32 = 1000;
const MIN_INSTRUCTIONS : usize = 100;
const MAX_INSTRUCTIONS : usize = 1000;
const MAX_CHILD_PROCESSESS : u64 = 25;
const CHILD_BRANCH_RATE : f64 = 0.25;

#[derive(Clone)]
pub struct Generator {
    pub num_processes : i32,
    pub min_cycles : i32,
    pub max_cycles : i32,
    pub min_instructions : i32,
    pub max_instructions : i32,
    pub max_child_processes : i32,
    pub child_branch_rate : f32,
}

impl Generator {
    pub fn default() -> Self {
        Generator {
            num_processes: 500,
            min_cycles : 100,
            max_cycles : 1000,
            min_instructions : 100,
            max_instructions : 1000,
            max_child_processes : 25,
            child_branch_rate : 0.25,
        }
    }

    pub fn generate_process_tree(&self, rng : &mut rand::StdRng, dict : &Vec<String>) -> Process {
        let word = rand::sample(rng, dict, 1)[0];
        let mut process = Process::new(word.clone());
        let mut children : Vec<Process> = vec![];

        let mut resources = self.num_processes;
        while resources > 0 {
            resources -= 1;
            let n = Binomial::new(self.child_branch_rate as f64, self.max_child_processes as u64).unwrap();
            let mut self_clone = self.clone();
            self_clone.num_processes = cmp::min(n.sample::<StdRng>(rng) as u32, resources as u32) as i32;

            children.push(self_clone.generate_process_tree(rng, dict));
            resources -= self_clone.num_processes;
        }

        process.program = generate_program(rng, self.min_instructions as usize, self.max_instructions as usize, children);

        process
    }
}

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

    pub fn depth(&self) -> u32 {
        let mut depth = 1;
        for inst in &self.program {
            match inst {
                &Instruction::CPU(_) | &Instruction::IO(_) => {}
                &Instruction::Spawn(ref process) => {
                    let p_depth = process.depth();
                    if 1 + p_depth > depth {
                        depth = 1 + p_depth;
                    }
                }
            }
        }
        depth
    }

}
