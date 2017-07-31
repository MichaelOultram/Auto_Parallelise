use std::sync::{Arc, mpsc};

use process::Process as Process;

use router;
use router::Router as Router;

pub struct Packet {
    pub to_id : usize,
    pub from_id : usize,
}

pub struct Machine {
    pub id : usize,
    pub global_queue: Vec<Process>,
    pub local_queue : Vec<Process>,
    pub comms_send : mpsc::Sender<Packet>,
    pub comms_receive : mpsc::Receiver<Packet>,
}


impl Machine {

    pub fn new(machine_id : usize, router : &mut Router) -> Self {
        let (comms_send, comms_receive) = router::add_machine(router, machine_id);
        Machine {
            id: machine_id,
            global_queue: vec![],
            local_queue: vec![],
            comms_send: comms_send,
            comms_receive: comms_receive,
        }
    }

    pub fn start(&self) {

    }

    // Perform a context switch
    pub fn switch(&self) {
        loop {
            if self.id == 0 {
                // Receive packets
                match self.comms_receive.recv() {
                    Ok(packet) => println!("Received from {}", packet.from_id),
                    Err(e) => println!("{:?}", e),
                }
            } else {
                // Sends a test packet
                let packet = Packet {
                    to_id: 0,
                    from_id: self.id,
                };
                self.comms_send.send(packet);
            }
        }
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
