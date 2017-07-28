use serde_json;
use std::cmp;
use rand;
use rand::distributions::{IndependentSample, Range};
use rand::StdRng;
use statrs::distribution::{Binomial, Distribution};

static MIN_CYCLES : u32 = 100;
static MAX_CYCLES : u32 = 1000;
static MIN_INSTRUCTIONS : usize = 25;
static MAX_INSTRUCTIONS : usize = 100;
static MAX_CHILD_PROCESSESS : u64 = 25;
static CHILD_BRANCH_RATE : f64 = 0.75;

#[derive(Serialize)]
pub enum Instruction {
    CPU(u32),
    IO(u32), // Which machine's IO?
    Spawn(Process),
}


pub type Program = Vec<Instruction>;

pub fn generate_program<R : rand::Rng>(rng : &mut R, min_extra : usize, max_extra : usize, child_processes : Vec<Process>) -> Program {
    // Convert child processess into Spawn instructions
    // Add these instructions to our program
    let num_children = child_processes.len();
    let mut program : Program = child_processes.into_iter().map(|p| Instruction::Spawn(p)).collect();
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
    program
}

/// The status of a context - used for scheduling
/// See syscall::process::waitpid and the sync module for examples of usage
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub enum Status {
    Runnable,
    Blocked,
    Exited,
}

#[derive(Serialize)]
pub struct Process {
    pub name : String,
    pub status : Status,
    pub running: bool,
    pub program : Program,
    pub machine : Option<u32>,
}

impl Process {
    pub fn new(name: String) -> Self {
        Process {
            name: name,
            status: Status::Blocked,
            running : false,
            program: vec![],
            machine : None,
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

/*    pub fn generate_processes(amount : u32, dict : &Vec<String>) -> Vec<Self> {
        let mut processes : Vec<Process> = vec![];

        // Random number generator
        let mut rng = rand::thread_rng();

        // Generate empty processess
        for _ in 0..amount {
            // Create a process using a random dictionary word
            let word = rand::sample(&mut rng, dict, 1)[0];
            let mut process = Process::new(word.clone());
            //process.program = generate_program(&mut rng, MIN_INSTRUCTIONS, MAX_INSTRUCTIONS, vec![]);
            processes.push(process);
        }


        // Form a tree of processess
        while processes.len() > 2 {
            // Select the parent process
            let mut parent = processes.pop().unwrap();

            // Select some children processess
            let num_processes = processes.len();
            println!("num_processes: {}", num_processes);
            let children_range = Range::new(1, 1 + cmp::min(num_processes - 1, MAX_CHILD_PROCESSESS));
            let num_children = children_range.ind_sample(&mut rng);
            let mut children = processes.split_off(num_processes - num_children);

            // Make sure all children processes have a program
            for p in &mut children {
                if p.program.is_empty() {
                    p.program = generate_program(&mut rng, MIN_INSTRUCTIONS, MAX_INSTRUCTIONS, vec![]);
                }
            }

            assert_eq!(num_children, children.len());

            // Generate the program for the process which will launch the children processess
            parent.program = generate_program(&mut rng, MIN_INSTRUCTIONS, MAX_INSTRUCTIONS, children);
            println!("{}", parent.to_json());
            processes.push(parent);
        }


        processes
    }
*/

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
