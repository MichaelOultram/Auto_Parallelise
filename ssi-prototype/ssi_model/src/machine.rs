use std::thread;
use std::thread::JoinHandle;
use std::sync::mpsc;
use std::collections::VecDeque;

use rand;
use rand::Rng;

use process;
use process::Process as Process;
use process::Instruction as Instruction;

use router::*;

#[derive(Clone)]
pub struct MachineConfig {
    pub num_machines: i32,
    pub local_queue_length: i32,
    pub num_cycles_per_context: i32,
    pub max_hops: i32,
}

impl MachineConfig {
    pub fn new() -> Self {
        MachineConfig {
            num_machines: 10,
            local_queue_length: 3,
            num_cycles_per_context: 100,
            max_hops: 10,
        }
    }

    pub fn start_machines(&self, router: &mut Router, init_process: Process) -> Vec<JoinHandle<()>> {
        // Create the machines
        let mut machines = VecDeque::new();
        for machine_id in 0..self.num_machines as usize {
            let machine_config = self.clone();
            let machine = Machine::new(machine_config, machine_id, router);
            machines.push_back(machine);
        }

        // Give the init process to the first machine
        machines[0].global_queue.push_back(init_process);

        // Start all machine threads
        machines.into_iter().map(|mut m| thread::spawn(move || m.switch())).collect()
    }
}

#[derive(Clone)]
pub enum NetData {
    Request(usize, u32), // consumer, hops
    Reply(Process),
    Terminate, // Machine should terminate
}

pub struct Machine {
    pub id : usize,
    pub global_queue: VecDeque<Process>, // Processes that may be moved to another machine
    pub blocked_machines_queue : VecDeque<usize>, // Machine id's that are waiting on this machine for a process
    pub local_queue : VecDeque<Process>, // Processes that are running on this machine

    pub comms_send : mpsc::Sender<Packet>,
    pub comms_receive : mpsc::Receiver<Packet>,
    pub vector_clock : Vec<u32>,

    pub num_requests_sent: u32,
    pub running: bool,
    pub config: MachineConfig,
}


impl Machine {
    pub fn new(config: MachineConfig, machine_id : usize, router : &mut Router) -> Self {
        let (comms_send, comms_receive) = router.add_machine(machine_id);
        Machine {
            id: machine_id,
            global_queue: VecDeque::new(),
            blocked_machines_queue: VecDeque::new(),
            local_queue: VecDeque::new(),
            comms_send: comms_send,
            comms_receive: comms_receive,
            vector_clock: vec![0;config.num_machines as usize],
            num_requests_sent: 0,
            running: false,
            config: config,
        }
    }

    #[inline]
    fn random_machine(&self) -> usize {
        let mut rng = rand::thread_rng();
        loop {
            let i = rng.gen_range(0, self.config.num_machines as usize);
            if i != self.id {
                return i
            }
        }

    }

    fn receive(&mut self) {
        // Keep receiving until there are no more packets waiting
        loop {
            match self.comms_receive.try_recv() {
                Ok(packet) => {
                    combine_vector_clocks(&mut self.vector_clock, &packet.vector_clock);
                    //rself.print(format!("VC-{:?}", self.vector_clock));
                    match packet.data {
                        PacketData::NetData(to_id, data) => match data {
                            NetData::Request(request_machine_id, hops) => {
                                if self.global_queue.is_empty() {
                                    if hops >= self.config.max_hops as u32 {
                                        // Block the request_machine_id
                                        self.blocked_machines_queue.push_back(request_machine_id);
                                        self.print(format!("Blocked {}", request_machine_id));
                                    } else {
                                        // Forward the request to another machine
                                        let random_machine = self.random_machine();
                                        self.send_netdata(random_machine, NetData::Request(request_machine_id, hops + 1));
                                    }
                                } else {
                                    // Honor the request
                                    let process = self.global_queue.pop_front().unwrap();
                                    self.send_netdata(request_machine_id, NetData::Reply(process));
                                }
                            },
                            NetData::Reply(mut process) => {
                                process.status = process::Status::Runnable;
                                self.local_queue.push_back(process);
                                self.num_requests_sent -= 1;
                            },
                            NetData::Terminate => self.running = false,
                        },
                        PacketData::SimData(_,_) => unreachable!(), // Router should absorb all NetData packets
                    }
                },
                Err(_) => return,
            }
        }
    }

    fn send(&mut self, data: PacketData) {
        self.vector_clock[self.id] += 1;
        let packet = Packet {
            vector_clock: self.vector_clock.clone(),
            data: data,
        };
        match self.comms_send.send(packet) {
            Err(e) => self.running = false,// Router must have terminated
            Ok(_) => {},
        }
    }

    fn send_netdata(&mut self, to_id: usize, data: NetData) {
        self.send(PacketData::NetData(to_id, data));
    }

    fn send_simdata(&mut self, data: SimData) {
        let from_id = self.id;
        self.send(PacketData::SimData(from_id, data));
    }

    fn fetch_new_processes(&mut self){
        // Give all blocked machines a process (if there is one)
        while !self.global_queue.is_empty() && !self.blocked_machines_queue.is_empty() {
            let request_machine_id = self.blocked_machines_queue.pop_front().unwrap();
            let process = self.global_queue.pop_front().unwrap();
            self.print(format!("Gave {} to machine-{}", process.to_string(), request_machine_id));
            self.send_netdata(request_machine_id, NetData::Reply(process));
        }

        // Make our local_queue length + num_requests_sent >= TARGET_LOCAL_QUEUE_LENGTH
        while (self.local_queue.len() as u32) + self.num_requests_sent < self.config.local_queue_length as u32 {
            // Check to see if there is any processes waiting in the global queue
            if !self.global_queue.is_empty() {
                // Move a process from the global queue into the local queue
                let mut process = self.global_queue.pop_front().unwrap();
                process.status = process::Status::Runnable;
                self.print(format!("Added {} from global_queue to local_queue", process.to_string()));
                self.send_simdata(SimData::ProcessStart(process.name.clone()));
                self.local_queue.push_back(process);
            } else {
                // Request another process from a random machine
                let random_machine = self.random_machine();
                let packet_data = NetData::Request(self.id, 1); // 1 is initial number of hops
                self.send_netdata(random_machine, packet_data);
                self.print(format!("Requesting a process from machine-{}", random_machine));
                self.num_requests_sent += 1;
            }
        }
    }


    // Perform a context switch
    pub fn switch(&mut self) {
        self.print("Started".to_string());
        self.running = true;
        while self.running {
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
        self.print(format!("Terminated"));
    }

    fn print(&self, message : String) {
        //println!("[{}] {}", self.to_string(), message);
    }

    fn execute(&mut self, process : &mut Process) {
        let mut cycles = 0;
        while cycles < self.config.num_cycles_per_context && process.status == process::Status::Runnable {
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
                    self.send_simdata(SimData::ProcessSpawn(new_process.name.clone()));
                    self.global_queue.push_back(new_process);
                }
            },
            None => {}
        }

        if process.program.is_empty() {
            process.status = process::Status::Exited;
            let from_id = self.id;
            self.send_simdata(SimData::ProcessEnd(process.name.clone()));
        }
    }

    pub fn to_string(&self) -> String {
        format!("machine-{}", self.id)
    }

}
