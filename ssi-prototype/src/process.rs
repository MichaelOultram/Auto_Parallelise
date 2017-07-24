use serde_json;

#[derive(Debug, PartialEq, Serialize)]
pub enum InstructionType {
    CPU,
    IO,
}

#[derive(Serialize)]
pub struct Instruction {
    itype: InstructionType,
    cycles: u32,
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
    pub program : Vec<Instruction>,
    pub machine : Option<u32>,
}

impl Process {
    pub fn new(name: String) -> Process {
        Process {
            name: name,
            status: Status::Blocked,
            running : false,
            program: Vec::new(),
            machine : None,
        }
    }

    pub fn step(&mut self) {
        let mut finished_instruction = false;
        match self.program.get_mut(0) {
            Some(inst) => {
                if (inst.itype == InstructionType::CPU && self.running) ||
                   (inst.itype == InstructionType::IO  && self.status == Status::Blocked) {
                    inst.cycles -= 1;
                    finished_instruction = (inst.cycles <= 0);
                }
            },
            None => ()
        }

        if finished_instruction {
            self.program.pop();
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

}
