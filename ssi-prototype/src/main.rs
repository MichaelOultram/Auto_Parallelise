#[macro_use]
extern crate serde_derive;

extern crate serde;
extern crate serde_json;


mod process;
use process::Process as Process;

fn main() {
    let mut p = Process::new("Hello".to_string());
    println!("{}", p.to_json());

}
