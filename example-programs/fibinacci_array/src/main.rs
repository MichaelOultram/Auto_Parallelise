#![feature(i128_type)]
#![feature(use_external_macros)]

#![feature(plugin)]
#![plugin(auto_parallelize)]

use std::time::Instant;

#[auto_parallelize]
fn main() {
    let now = Instant::now();

    let fibs = fibinacci_array(10);
    println!("{:?}", fibs);

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}

#[auto_parallelize]
fn fibinacci_array(length: usize) -> Vec<u128> {
    let mut output = vec![1,1];
    for i in 2..length {
        let next = {
            let a = output.get(i-1).unwrap();
            let b = output.get(i-2).unwrap();
            a+b
        };
        output.push(next);
    }
    output
}
