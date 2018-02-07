#![feature(plugin)]
#![plugin(auto_parallelise)]

#[autoparallelise]
fn main() {
    let mut a = 4;
    let mut b = 3;
    //let mut c = 5;
    a += 1;
    b += 1;
    //println!("{} * {}", a, c);
    //println!("{} + {}", a, b);
}

/*
fn main() {
    std::thread::spawn(move || { let mut a = 4; a += 1; });
    let mut b = 3;
    b += 1;
}
*/
