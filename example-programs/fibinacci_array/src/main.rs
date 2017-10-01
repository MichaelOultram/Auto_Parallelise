#![feature(i128_type)]
use std::time::Instant;

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

fn main() {
    let now = Instant::now();

    let fibs = fibinacci_array(1000);
    println!("{:?}", fibs);

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}
