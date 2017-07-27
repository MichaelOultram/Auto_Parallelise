#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate rand;

use std::thread;
use std::sync::{Mutex, Arc};

use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

mod process;
use process::Process as Process;


mod machine;
use machine::Machine as Machine;

type JoinHandle = thread::JoinHandle<()>;
type AMutex<T> = Arc<Mutex<T>>;
#[inline]
pub fn new_amutex<T>(t : T) -> Arc<Mutex<T>> {
    Arc::new(Mutex::new(t))
}

fn dictionary() -> Vec<String> {
    let mut dict = vec![];

    // Put each line of dictionary.txt into the vector
    let f = File::open("dictionary.txt").unwrap();
    let file = BufReader::new(&f);
    for line in file.lines(){
        let l = line.unwrap();
        dict.push(l);
    }

    dict
}

fn main() {
    // Generate Processess
    let all_processes : Vec<Process>;
    {
        // Load the dictionary from file
        let dict = dictionary();

        // Process queue
        all_processes = Process::generate_processes(100, &dict);
    }

    // Generate Machines
    let mut all_machines = vec![];
    for machine_id in 0..4 {
        let machine = Arc::new(Machine::new(machine_id));
        let machine2 = machine.clone();
        thread::spawn(move || {
            machine2.switch()
        });
        all_machines.push(machine);
    }

    for m in &all_machines {
        println!("{}", m.to_string());
    }

    // Print all_processess
    for p in &all_processes {
        println!("{}", p.to_string());
    }

    for m in &all_machines {
        println!("{}", m.to_string());
    }

}
