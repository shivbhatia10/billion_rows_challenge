use std::{
    fs::File,
    io::{self, Read},
};

const FILE_NAME: &'static str = "measurements.txt";

pub fn main() -> Result<(), io::Error> {
    let mut file = File::open(FILE_NAME)?;
    let mut buffer = [0; 10];

    let n = file.read(&mut buffer)?;

    println!("buffer read: {:?}", &buffer[..n]);

    Ok(())
}
