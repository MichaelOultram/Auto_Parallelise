#![feature(plugin)]
#![plugin(auto_parallelise())]

#[autoparallelise]
fn main() {
    let mut a = 4;
    let mut b = 3;
    let mut c = 5;
    a += 1;
    b += 1;
    for i in 0..a {
        println!("{}", i);
    }
    println!("{} * {}", a, c);
    println!("{} + {}", a, b);
    println!("End of program");
}
