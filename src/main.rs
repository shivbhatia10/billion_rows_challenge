use std::collections::HashMap;
use std::fmt::Write;
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

const FILE_NAME: &'static str = "measurements.txt";

pub fn main() -> Result<(), io::Error> {
    let file = File::open(FILE_NAME)?;
    let mut buf_reader = BufReader::with_capacity(1 << 20, file);
    let mut line: Vec<u8> = Vec::new();

    // (min, total, max, count)
    let mut accs: HashMap<Vec<u8>, (i16, i64, i16, u32)> = HashMap::new();

    for _ in 0..1_000_000_000 {
        line.clear();
        let mut n = buf_reader.read_until(b'\n', &mut line)?;
        if n == 0 {
            break;
        }
        if line[n - 1] == b'\n' {
            n -= 1;
        }
        let sc = if line[n - 4] == b';' {
            n - 4
        } else if line[n - 5] == b';' {
            n - 5
        } else {
            n - 6
        };
        let (key, value) = (&line[..sc], &line[sc + 1..]);

        let (neg, value) = if value[0] == b'-' {
            (true, &value[1..])
        } else {
            (false, value)
        };
        let value = if value.len() == 3 {
            (value[0] - b'0') as i16 * 10 + (value[2] - b'0') as i16
        } else {
            (value[0] - b'0') as i16 * 100
                + (value[1] - b'0') as i16 * 10
                + (value[3] - b'0') as i16
        };
        let value = if neg { -value } else { value };

        match accs.get_mut(key) {
            Some(e) => {
                e.0 = e.0.min(value);
                e.1 += value as i64;
                e.2 = e.2.max(value);
                e.3 += 1;
            }
            None => {
                accs.insert(key.to_vec(), (value, value as i64, value, 1));
            }
        }
    }

    let mut out = String::with_capacity(accs.len() * 40);
    out.push('{');
    let mut entries: Vec<_> = accs.iter().collect();
    entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
    for (i, (name, (min, total, max, count))) in entries.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        let mean = *total as f64 / *count as f64 / 10.0;
        write!(
            out,
            "{}={:.1}/{mean:.1}/{:.1}",
            String::from_utf8_lossy(name),
            *min as f64 / 10.0,
            *max as f64 / 10.0
        )
        .unwrap();
    }
    out.push_str("}\n");
    print!("{out}");

    Ok(())
}

