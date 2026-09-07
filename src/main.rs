use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

const FILE_NAME: &'static str = "measurements.txt";

pub fn main() -> Result<(), io::Error> {
    let file = File::open(FILE_NAME)?;
    let mut buf_reader = BufReader::new(file);
    let mut buffer = String::new();

    for _ in 0..1_000_000_000 {
        buf_reader.read_line(&mut buffer)?;

        print!("{}", &buffer);
        buffer.clear();
    }

    Ok(())
}

