#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
extern crate rand;
extern crate statrs;

pub mod machine;
pub mod process;
pub mod router;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
    }
}
