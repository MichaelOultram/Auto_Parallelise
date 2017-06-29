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

#[derive(Debug, PartialEq, Serialize)]
pub enum State {
    STARTED,
    READY,
    RUNNING,
    WAITING,
    TERMINATED,
}

#[derive(Serialize)]
pub struct Process {
    name : String,
    state : State,
    program : Vec<Instruction>,
    machine : Option<u32>,
}

impl Process {
    pub fn new(name: String) -> Process {
        Process {
            name: name,
            state: State::STARTED,
            program: Vec::new(),
            machine : None,
        }
    }

    pub fn step(&mut self) {
        let mut finished_instruction = false;
        match self.program.get_mut(0) {
            Some(inst) => {
                if (inst.itype == InstructionType::CPU && self.state == State::RUNNING) ||
                   (inst.itype == InstructionType::IO  && self.state == State::WAITING) {
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
        format!("[{}: {:?}]", self.name, self.state)
    }

    pub fn to_json(&self) -> String {
        match serde_json::to_string_pretty(&self) {
            Ok(s) => s,
            Err(e) => panic!(e),
        }
    }

}
