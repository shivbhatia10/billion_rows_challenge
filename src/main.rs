use std::collections::HashMap;
use std::fmt::Write;
use std::hash::{BuildHasherDefault, Hasher};
use std::{
    fs::File,
    io::{self, BufRead, BufReader},
};

const FILE_NAME: &'static str = "measurements.txt";
const K: u64 = 0x517c_c1b7_2722_0a95;

#[derive(Default)]
struct CustomHasher(u64);

impl Hasher for CustomHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut h = self.0;
        let mut c = bytes;
        while c.len() >= 8 {
            let v = u64::from_le_bytes(c[..8].try_into().unwrap());
            h = (h.rotate_left(5) ^ v).wrapping_mul(K);
            c = &c[8..];
        }
        for &b in c {
            h = (h.rotate_left(5) ^ b as u64).wrapping_mul(K);
        }
        self.0 = h;
    }
    fn write_u8(&mut self, i: u8) {
        self.0 = (self.0.rotate_left(5) ^ i as u64).wrapping_mul(K);
    }
    fn write_usize(&mut self, i: usize) {
        self.0 = (self.0.rotate_left(5) ^ i as u64).wrapping_mul(K);
    }
    fn finish(&self) -> u64 {
        self.0
    }
}

type CustomHashMap<K, V> = HashMap<K, V, BuildHasherDefault<CustomHasher>>;

pub fn main() -> Result<(), io::Error> {
    let file = File::open(FILE_NAME)?;
    let mut buf_reader = BufReader::with_capacity(1 << 20, file);
    let mut line: Vec<u8> = Vec::new();

    // (min, total, max, count)
    let mut accs: CustomHashMap<Vec<u8>, (i16, i64, i16, u32)> = CustomHashMap::default();

    for _ in 0..1_000_000_000 {
        line.clear();
        let n = buf_reader.read_until(b'\n', &mut line)?;
        let (split_point, value) = parse(&line, n);
        let key = &line[..split_point];


        // if n == 0 {
        //     break;
        // }
        // if line[n - 1] == b'\n' {
        //     n -= 1;
        // }
        // let sc = if line[n - 4] == b';' {
        //     n - 4
        // } else if line[n - 5] == b';' {
        //     n - 5
        // } else {
        //     n - 6
        // };
        // let (key, value) = (&line[..sc], &line[sc + 1..]);
        //
        // let (neg, value) = if value[0] == b'-' {
        //     (true, &value[1..])
        // } else {
        //     (false, value)
        // };
        // let value = if value.len() == 3 {
        //     (value[0] - b'0') as i16 * 10 + (value[2] - b'0') as i16
        // } else {
        //     (value[0] - b'0') as i16 * 100
        //         + (value[1] - b'0') as i16 * 10
        //         + (value[3] - b'0') as i16
        // };
        // let value = if neg { -value } else { value };

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

fn parse(line: &[u8], n: usize) -> (usize, i16) {
    let tenths = (line[n - 1] - b'0') as i16;
    let ones = (line[n - 3] - b'0') as i16;
    let c4 = line[n - 4];
    let has_tens = ((c4 >= b'0') & (c4 <= b'9')) as i16;
    let tens = (c4 as i16 - b'0' as i16) * has_tens;
    let p = n - 4 - has_tens as usize;
    let neg = (line[p] == b'-') as i16;
    let mag = tens * 100 + ones * 10 + tenths;
    (p - neg as usize, (mag ^ -neg) + neg)
}
