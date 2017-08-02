use std::sync::mpsc;
use std::collections::VecDeque;

use rand;
use rand::Rng;

use process;
use process::Process as Process;
use process::Instruction as Instruction;

use router;
use router::Router as Router;
use router::Packet as Packet;


use NUM_MACHINES;
static TARGET_LOCAL_QUEUE_LENGTH: u32 = 10;
static NUM_CYCLES_PER_CONTEXT: u32 = 100;
static MAX_HOPS: u32 = 10;

pub enum PacketData {
    REQUEST(usize, u32), // consumer, hops
    REPLY(Process),
}

pub struct Machine {
    pub id : usize,
    pub global_queue: VecDeque<Process>, // Processes that may be moved to another machine
    pub blocked_machines_queue : VecDeque<usize>, // Machine id's that are waiting on this machine for a process
    pub local_queue : VecDeque<Process>, // Processes that are running on this machine
    pub comms_send : mpsc::Sender<Packet>,
    pub comms_receive : mpsc::Receiver<Packet>,
    pub num_requests_sent: u32,
}


impl Machine {

    pub fn new(machine_id : usize, router : &mut Router) -> Self {
        let (comms_send, comms_receive) = router::add_machine(router, machine_id);
        Machine {
            id: machine_id,
            global_queue: VecDeque::new(),
            blocked_machines_queue: VecDeque::new(),
            local_queue: VecDeque::new(),
            comms_send: comms_send,
            comms_receive: comms_receive,
            num_requests_sent: 0,
        }
    }

    #[inline]
    fn random_machine(&self) -> usize {
        let mut rng = rand::thread_rng();
        loop {
            let i = rng.gen_range(0, NUM_MACHINES);
            if i != self.id {
                return i
            }
        }

    }

    fn receive(&mut self) {
        // Keep receiving until there are no more packets waiting
        loop {
            match self.comms_receive.try_recv() {
                Ok(packet) => match packet.data {
                    PacketData::REQUEST(request_machine_id, hops) => {
                        if self.global_queue.is_empty() {
                            if hops >= MAX_HOPS {
                                // Block the request_machine_id
                                self.blocked_machines_queue.push_back(request_machine_id);
                            } else {
                                // Forward the request to another machine
                                self.send(self.random_machine(), PacketData::REQUEST(request_machine_id, hops + 1));
                            }
                        } else {
                            // Honor the request
                            let process = self.global_queue.pop_front().unwrap();
                            self.send(request_machine_id, PacketData::REPLY(process));
                        }
                    },
                    PacketData::REPLY(mut process) => {
                        process.status = process::Status::Runnable;
                        self.local_queue.push_back(process);
                        self.num_requests_sent -= 1;
                    },
                },
                Err(_) => return,
            }
        }
    }

    fn send(&self, to_id: usize, data: PacketData) {
        let packet = Packet {
            to_id: to_id,
            from_id: self.id,
            data: data,
        };
        match self.comms_send.send(packet) {
            Err(e) => panic!(e),
            Ok(_) => {},
        }
    }

    fn fetch_new_processes(&mut self){
        // Give all blocked machines a process (if there is one)
        while !self.global_queue.is_empty() && !self.blocked_machines_queue.is_empty() {
            let request_machine_id = self.blocked_machines_queue.pop_front().unwrap();
            let process = self.global_queue.pop_front().unwrap();
            self.print(format!("Gave {} to machine-{}", process.to_string(), request_machine_id));
            self.send(request_machine_id, PacketData::REPLY(process));
        }

        // Make our local_queue length + num_requests_sent >= TARGET_LOCAL_QUEUE_LENGTH
        while (self.local_queue.len() as u32) + self.num_requests_sent < TARGET_LOCAL_QUEUE_LENGTH {
            // Check to see if there is any processes waiting in the global queue
            if !self.global_queue.is_empty() {
                // Move a process from the global queue into the local queue
                let mut process = self.global_queue.pop_front().unwrap();
                process.status = process::Status::Runnable;
                self.print(format!("Added {} from global_queue to local_queue", process.to_string()));
                self.local_queue.push_back(process);
            } else {
                // Request another process from a random machine
                let random_machine = self.random_machine();
                self.send(random_machine, PacketData::REQUEST(self.id, 1));
                self.print(format!("Requesting a process from machine-{}", random_machine));
                self.num_requests_sent += 1;
            }
        }
    }


    // Perform a context switch
    pub fn switch(&mut self) {
        self.print("Started".to_string());
        loop {
            self.fetch_new_processes();
            self.receive();

            // Get the next process
            match self.local_queue.pop_front() {
                Some(mut process) => {
                    // Check the process can run
                    if process.status == process::Status::Runnable {
                        self.print(format!("Executing {}", process.to_string()));
                        self.execute(&mut process);
                    }

                    // Add the process to the back of the queue if it has not exited
                    if process.status != process::Status::Exited {
                        self.local_queue.push_back(process);
                    } else {
                        self.print(format!("{} exited", process.to_string()));
                    }
                },
                None => {}, // Nothing to execute
            }

        }
    }

    fn print(&self, message : String) {
        println!("[{}] {}", self.to_string(), message);
    }

    fn execute(&mut self, process : &mut Process) {
        let mut cycles = 0;
        while cycles < NUM_CYCLES_PER_CONTEXT && process.status == process::Status::Runnable {
            self.step_program(process);
            cycles += 1;
        }
    }

    // Executes one instruction for the program
    fn step_program(&mut self, process : &mut Process) {
        // Look at the next instruction
        match process.program.pop_front() {
            Some(inst) => match inst {
                Instruction::CPU(mut cycles) => {
                    // Decrease the cycles by 1
                    cycles -= 1;
                    // Check to see if the instruction has finished
                    if cycles > 0 {
                        process.program.push_front(Instruction::CPU(cycles));
                    }
                }
                Instruction::IO(mut cycles) => {
                    // Decrease the cycles by 1
                    cycles -= 1;
                    // Check to see if the instruction has finished
                    if cycles > 0 {
                        process.program.push_front(Instruction::IO(cycles));
                    }
                }
                Instruction::Spawn(new_process) => {
                    self.global_queue.push_back(new_process);
                }
            },
            None => {}
        }

        if process.program.is_empty() {
            process.status = process::Status::Exited;
        }
    }

    pub fn to_string(&self) -> String {
        format!("machine-{}", self.id)
    }

}
