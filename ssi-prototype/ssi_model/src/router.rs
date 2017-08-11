use std::thread;
use std::sync::mpsc;
use self::mpsc::{Sender, Receiver};
use rand;
use rand::Rng;
use machine::NetData as NetData;

pub struct Router {
    senders: Vec<Sender<Packet>>,
    receivers: Vec<Receiver<Packet>>,
    num_processes: u32,
    vector_clock: Vec<u32>,
}

#[derive(Clone)]
pub struct Packet {
    pub vector_clock : Vec<u32>,
    pub data : PacketData,
}

#[derive(Clone)]
pub enum PacketData {
    NetData(usize, NetData), // to_id, NetData
    SimData(usize, SimData), // from_id, NetData
}

#[derive(Clone)]
pub enum SimData {
    ProcessStart(String),
    ProcessEnd(String),
    ProcessSpawn(String),
}

impl Router {
    pub fn new(num_processes: u32) -> Self {
        Router {
            senders: vec![],
            receivers: vec![],
            vector_clock: vec![],
            num_processes: num_processes,
        }
    }

    pub fn add_machine(&mut self, id : usize) -> (Sender<Packet>, Receiver<Packet>) {
        // Create channels
        let (router_send, machine_receive) = mpsc::channel();
        let (machine_send, router_recieve) = mpsc::channel();

        // Add a space in the vector clock
        self.vector_clock.push(0);

        // Allow a 2 way connection between router and machine
        self.senders.insert(id, router_send);
        self.receivers.insert(id, router_recieve);
        (machine_send, machine_receive)
    }

    pub fn start_router(mut self) -> (Box<Fn() -> ()>, Receiver<Packet>) {
        // Allow external communication with router thread
        let (external_send, router_receive) = mpsc::channel();
        let (simulation_relay, external_receive) = mpsc::channel();

        self.receivers.push(router_receive);
        let num_machines = self.senders.len();
        thread::spawn(move || {
            let mut rng = rand::thread_rng();
            let mut processes_complete = 0;
            'main: loop {
                rng.shuffle(&mut self.receivers); // Makes network traffic order more unpredictable
                for receiver in self.receivers.iter() {
                    match receiver.try_recv() {
                        Ok(packet) => {
                            simulation_relay.send(packet.clone());
                            combine_vector_clocks(&mut self.vector_clock, &packet.vector_clock);
                            //println!("[router] {:?}", self.vector_clock);
                            match packet.data {
                                PacketData::NetData(to_id, data) => match data {
                                    // External terminate packet
                                    NetData::Terminate => {
                                        self.terminate();
                                        break 'main;
                                    }

                                    // Forward packet to machine
                                    _ => {
                                        // Find the sender channel and forward the packet
                                        let ref send_to = *self.senders.get(to_id).unwrap();
                                        let forward_packet = Packet{
                                            vector_clock: packet.vector_clock.clone(),
                                            data: PacketData::NetData(to_id, data),
                                        };
                                        match send_to.send(forward_packet) {
                                            Ok(_) => {},
                                            Err(e) => panic!(e),
                                        };
                                    }
                                },

                                PacketData::SimData(from_id, data) => match data {
                                    SimData::ProcessStart(process_name) => println!("[machine-{}] {} Started", from_id, process_name),

                                    SimData::ProcessEnd(process_name) => {
                                        processes_complete += 1;
                                        println!("[machine-{}] {} Ended", from_id, process_name);
                                        // Check to see if simulation is over
                                        if processes_complete >= self.num_processes {
                                            self.terminate();
                                            break 'main;
                                        }
                                    }

                                    SimData::ProcessSpawn(process_name) => println!("[machine-{}] {} Spawned", from_id, process_name),
                                },
                            }
                        },
                        Err(_) => {},// Ignore if no packet received
                    }
                }
            }
        });
        let terminate_simulation = Box::new(move || {
            external_send.send(Packet {
                vector_clock: vec![0;num_machines as usize], // Unable to use actual vector clock
                data: PacketData::NetData(0, NetData::Terminate),
            });
        });

        (terminate_simulation, external_receive)
    }

    fn terminate(&self) {
        println!("[router] killing all machines");
        // Kill all machines
        for i in 0..self.senders.len() {
            self.senders[i].send(Packet{
                vector_clock: self.vector_clock.clone(),
                data: PacketData::NetData(i, NetData::Terminate),
            });
        }
    }
}

pub fn combine_vector_clocks(original: &mut Vec<u32>, new: &Vec<u32>) {
    let length = original.len();
    assert_eq!(length, new.len());

    for i in 0..length {
        if new[i] > original[i] {
            original[i] = new[i];
        }
    }
}
