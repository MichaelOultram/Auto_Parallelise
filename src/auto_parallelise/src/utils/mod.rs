use std::fs::File;
use std::io::prelude::*;
use std::path::Path;

pub mod macros;
//pub mod noqueue_threadpool;

pub fn write_file(path: &Path, contents: &String) {
    let mut file = match File::create(&path) {
        Err(why) => panic!("Failed to open {}: {}", path.display(), why),
        Ok(file) => file,
    };

    // Write obj_json into the file
    if let Err(why) = file.write_all(contents.as_bytes()) {
        panic!("Failed to write {}: {}", path.display(), why);
    }
}

pub fn read_file(filename: &str) -> Option<String> {
    // Attempt to open file
    let path = Path::new(filename);
    let maybe_file = File::open(&path);

    // If the file cannot be open, this is a new run
    if let Err(_) = maybe_file {
        return None;
    }
    let mut file = maybe_file.unwrap();

    // Read the file contents into a string
    let mut s = String::new();
    if let Err(why) = file.read_to_string(&mut s) {
        //panic!("Failed to open {}: {}", path.display(), why)
        return None;
    }
    return Some(s);
}
