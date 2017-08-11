use serde_json;
use std::cmp;
use std::collections::VecDeque;
use rand;
use rand::StdRng;
use statrs::distribution::{Binomial, Distribution};

#[derive(Clone)]
pub struct Generator {
    pub num_processes : i32,
    pub min_cycles : i32,
    pub max_cycles : i32,
    pub min_instructions : i32,
    pub max_instructions : i32,
    pub max_child_processes : i32,
    pub child_branch_rate_initial : f32,
    pub child_branch_rate_ramp : f32,
}

impl Generator {
    pub fn default() -> Self {
        Generator {
            num_processes: 300,
            min_cycles : 100,
            max_cycles : 1000,
            min_instructions : 100,
            max_instructions : 1000,
            max_child_processes : 85,
            child_branch_rate_initial : 0.7,
            child_branch_rate_ramp : -0.175,
        }
    }

    pub fn generate_process_tree(&self, rng : &mut rand::StdRng, dict : &Vec<String>) -> Process {
        // TODO: Make sure each process has a unique name
        let word = rand::sample(rng, dict, 1)[0];
        let mut process = Process::new(word.clone());
        let mut children : Vec<Process> = vec![];

        let mut resources = self.num_processes - 1;
        while resources > 0 {
            let n = Binomial::new(self.child_branch_rate_initial as f64, self.max_child_processes as u64).unwrap();

            // Create a clone to generate the child tree
            let mut self_clone = self.clone();

            // Assign some resources to the child process
            self_clone.num_processes = 0;
            while self_clone.num_processes == 0 {
                self_clone.num_processes = cmp::min(n.sample::<StdRng>(rng) as u32, resources as u32) as i32;
            }
            assert!(self_clone.num_processes > 0); // Make sure that child has at least 1 resource
            assert!(self_clone.num_processes <= resources);

            // Alter the branch rate (but keep it in bounds)
            self_clone.child_branch_rate_initial += self_clone.child_branch_rate_ramp;
            if self_clone.child_branch_rate_initial < 0.001 {
                self_clone.child_branch_rate_initial = 0.001;
            } else if self_clone.child_branch_rate_initial > 1.0 {
                self_clone.child_branch_rate_initial = 1.0;
            }

            children.push(self_clone.generate_process_tree(rng, dict));
            resources -= self_clone.num_processes;
        }

        process.program = self.generate_program(rng, children);

        process
    }

    pub fn generate_program<R : rand::Rng>(&self, rng : &mut R, child_processes : Vec<Process>) -> Program {
        // Convert child processess into Spawn instructions
        // Add these instructions to our program
        let num_children = child_processes.len();
        let mut program : Vec<Instruction> = child_processes.into_iter().map(|p| Instruction::Spawn(p)).collect();
        assert_eq!(num_children, program.len());

        // Pad the process with extra CPU and IO instructions
        for _ in 0..rng.gen_range(self.min_instructions, self.max_instructions) {
            let cycles = rng.gen_range(self.min_cycles as u32, self.max_cycles as u32);

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
}

#[derive(Clone, Serialize)]
pub enum Instruction {
    CPU(u32),
    IO(u32), // TODO: Which machine's IO?
    Spawn(Process),
}


pub type Program = VecDeque<Instruction>;

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

    pub fn num_processes(&self) -> u32 {
        let mut num_processes = 1;
        for inst in &self.program {
            match inst {
                &Instruction::CPU(_) | &Instruction::IO(_) => {}
                &Instruction::Spawn(ref process) => {
                    num_processes += process.num_processes();
                }
            }
        }
        num_processes
    }

}
