use process;
use process::Process as Process;

pub struct Machine {
    pub id : u32,
    pub global_queue: Vec<Process>,
    pub local_queue : Vec<Process>,
}


impl Machine {

    pub fn new(machine_id : u32) -> Self {
        Machine {
            id: machine_id,
            global_queue: vec![],
            local_queue: vec![],
        }
    }

    // Perform a context switch
    pub fn switch(&self) {

    }

    // Executes one instruction for the program
/*    pub fn step_program(&self, process : &mut Process) {
        let mut finished_instruction = false;
        match process.program.get_mut(0) {
            Some(inst) => match inst.itype {
                InstructionType::CPU => {
                    inst.cycles -= 1;
                    finished_instruction = inst.cycles <= 0;
                }
                InstructionType::IO => {
                    inst.cycles -= 1;
                    finished_instruction = inst.cycles <= 0;
                }
                InstructionType::Spawn(_) => {
                    inst.cycles -= 1;
                    finished_instruction = inst.cycles <= 0;
                }
            },
            None => {}
        }

        if finished_instruction {
            process.program.pop();
            if process.program.is_empty() {
                process.status = process::Status::Exited;
            }
        }
    }
*/
    pub fn to_string(&self) -> String {
        format!("machine-{}", self.id)
    }

}
