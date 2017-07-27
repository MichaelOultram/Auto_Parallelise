use serde_json;
use rand;
use rand::distributions::{IndependentSample, Range};

#[derive(Debug, PartialEq, Serialize)]
pub enum InstructionType {
    CPU,
    IO, // Which machine's IO?
}

#[derive(Serialize)]
pub struct Instruction {
    pub itype: InstructionType,
    pub cycles: u32,
}
pub type Program = Vec<Instruction>;

pub fn generate_program<R : rand::Rng>(rng : &mut R, min : u32, max : u32) -> Program {
    let mut instructions : Program = vec![];
    let range = Range::new(min, max);

    for _ in 0..range.ind_sample(rng) {

    }

    instructions
}

/// The status of a context - used for scheduling
/// See syscall::process::waitpid and the sync module for examples of usage
#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize)]
pub enum Status {
    Runnable,
    Blocked,
    Exited(usize)
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

    pub fn generate_processes(amount : u32, dict : &Vec<String>) -> Vec<Self> {
        let mut processes : Vec<Process> = vec![];

        // Random number generator
        let dict_range = Range::new(0, dict.len());
        let mut rng = rand::thread_rng();

        for _ in 0..amount {
            // Create a process using a random dictionary word
            let i = dict_range.ind_sample(&mut rng);
            let mut process = Process::new((*dict)[i].clone());

            // Generate the program for the process and add to the list
            process.program = generate_program(&mut rng, 5, 10);
            process.status = Status::Runnable;
            processes.push(process);
        }

        processes
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
