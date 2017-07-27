use process::Process as Process;
use process::InstructionType as InstructionType;

use {AMutex, new_amutex, JoinHandle};

pub struct Machine {
    pub id : u32,
    pub global_queue: AMutex<Vec<Process>>,
    pub local_queue : AMutex<Vec<Process>>,
    pub join_handle : AMutex<Option<JoinHandle>>,
}


impl Machine {

    pub fn new(machine_id : u32) -> Self {
        Machine {
            id: machine_id,
            global_queue: new_amutex(vec![]),
            local_queue: new_amutex(vec![]),
            join_handle: new_amutex(None),
        }
    }

    // Perform a context switch
    pub fn switch(&self) {
        
    }

    // Executes one instruction for the program
    pub fn step_program(&self, process : &mut Process) {
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
            },
            None => {}
        }

        if finished_instruction {
            process.program.pop();
        }
    }

    pub fn to_string(&self) -> String {
        format!("machine-{}", self.id)
    }

}
