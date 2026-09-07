use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead, BufReader},
};

const FILE_NAME: &'static str = "measurements.txt";

pub fn main() -> Result<(), io::Error> {
    let file = File::open(FILE_NAME)?;
    let mut buf_reader = BufReader::with_capacity(1 << 20, file);
    let mut buffer = String::new();

    let accs: HashMap<String, (f32, f32, f32, u32)> = HashMap::new();

    for i in 0..1_000_000_000 {
        buf_reader.read_line(&mut buffer)?;
        let Some((key, value)) = buffer.split_once(";") else {
            continue;
        };
        let value: f32 = value.trim().parse().map_err(|err| io::Error::other(err))?;

        let (min, sum, max, count) = accs.get(&key).unwrap_or((0.0, 0.0, 0.0, 0));

        buffer.clear();
        if i % 1_000_000 == 0 {
            println!("{i}");
        }
    }

    Ok(())
}
