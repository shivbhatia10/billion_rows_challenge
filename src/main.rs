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
    let mut buffer = String::new();

    // (min, total, max, count)
    let mut accs: HashMap<String, (i16, i64, i16, u32)> = HashMap::new();

    for _ in 0..1_000_000_000 {
        buffer.clear();
        buf_reader.read_line(&mut buffer)?;
        let Some((key, value)) = buffer.split_once(";") else {
            continue;
        };
        let value = parse_val(value)?;

        match accs.get_mut(key) {
            Some(e) => {
                e.0 = e.0.min(value);
                e.1 += value as i64;
                e.2 = e.2.max(value);
                e.3 += 1;
            }
            None => {
                accs.insert(key.to_string(), (value, value as i64, value, 1));
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
            "{name}={:.1}/{mean:.1}/{:.1}",
            *min as f64 / 10.0,
            *max as f64 / 10.0
        )
        .unwrap();
    }
    out.push_str("}\n");
    print!("{out}");

    Ok(())
}

fn parse_val(val: &str) -> Result<i16, io::Error> {
    let s = val.trim_end();
    let (neg, s) = match s.strip_prefix('-') {
        Some(rest) => (true, rest),
        None => (false, s),
    };
    let (whole, frac) = s
        .split_once('.')
        .ok_or(io::Error::other("Could not split {s}"))?;
    let v = whole
        .parse::<i16>()
        .map_err(|e| io::Error::other(format!("Could not parse whole {e:?}")))?
        * 10
        + frac
            .parse::<i16>()
            .map_err(|e| io::Error::other(format!("Could not parse frac {e:?}")))?;
    if neg { Ok(-v) } else { Ok(v) }
}
