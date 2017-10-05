#![feature(i128_type)]
#![feature(use_external_macros)]

#![feature(plugin)]
#![plugin(compiler_plugins)]

use std::time::Instant;

fn main() {
    let mut now = Instant::now();

    let fibs = fibinacci_array(rn!(X) as usize);
    println!("{:?}", fibs);

    let elapsed = now.elapsed();
    let sec = (elapsed.as_secs() as f64) + (elapsed.subsec_nanos() as f64 / 1000_000_000.0);
    println!("Seconds: {}", sec);
}

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

#[para]
fn test_function() -> u32 {
    test_function2()
}

fn test_function2() -> u32 {
    2
}
