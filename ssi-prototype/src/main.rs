#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;

mod process;
use process::Process as Process;
use process::Status as Status;

fn main() {
    let mut ready_queue = Vec::new();
    let mut run_queue = Vec::new();
    //let mut wait_queue = Vec::new();

    for x in 0..10 {
        let p = Process::new(format!("{}-{}","Hello", x));
        ready_queue.push(p);
    }

    for x in 0..3 {
        let mut p = ready_queue.pop().unwrap();
        p.status = Status::Runnable;
        run_queue.push(p);
    }

    for p in run_queue {
        println!("{}", p.to_json());
    }

}
