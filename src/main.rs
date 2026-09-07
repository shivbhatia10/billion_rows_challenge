// 1BRC measurements generator. No crates, no JDK.
//   rustc -O gen.rs -o gen
//   ./gen 1000000000 measurements.txt weather_stations.csv
// Station list (805 KB, no JDK needed):
//   curl -O https://raw.githubusercontent.com/gunnarmorling/1brc/main/data/weather_stations.csv
//
// Note: column 2 of that file is really a latitude, but the official generator
// treats it as a mean temperature too, so we do the same. Output for 1B rows is
// ~15.5 GB (a bit fatter than the official ~13 GB, since these city names are
// longer than the 413 names the original Java generator uses).
//
// Verified here: 10,000 unique stations, multi-byte UTF-8 names, every line
// matches ^[^;]+;-?[0-9]{1,2}\.[0-9]$, ~1.3 s per 10M rows per core.

use std::fs::File;
use std::io::{BufWriter, Read, Write};
use std::sync::mpsc::sync_channel;

const N_STATIONS: usize = 10_000;
const CHUNK_ROWS: usize = 500_000;

// xorshift64* — small, fast, plenty good for fake weather.
struct Rng(u64);
impl Rng {
    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }
    fn next_f64(&mut self) -> f64 {
        // 53 random bits -> [0,1)
        (self.next_u64() >> 11) as f64 * (1.0 / (1u64 << 53) as f64)
    }
    // Box-Muller: two uniforms in, one standard normal out.
    fn next_normal(&mut self) -> f64 {
        let u1 = self.next_f64().max(1e-300);
        let u2 = self.next_f64();
        (-2.0 * u1.ln()).sqrt() * (std::f64::consts::TAU * u2).cos()
    }
}

fn load_stations(path: &str) -> Vec<(Vec<u8>, f64)> {
    let mut raw = String::new();
    File::open(path)
        .unwrap_or_else(|e| panic!("cannot open {}: {}", path, e))
        .read_to_string(&mut raw)
        .expect("station file is not valid UTF-8");

    let mut seen = std::collections::HashSet::new();
    let mut all: Vec<(Vec<u8>, f64)> = Vec::new();
    for line in raw.lines() {
        if line.starts_with('#') || line.is_empty() {
            continue;
        }
        let Some((name, mean)) = line.split_once(';') else {
            continue;
        };
        let Ok(mean) = mean.trim().parse::<f64>() else {
            continue;
        };
        if name.is_empty() || name.len() > 100 || !seen.insert(name.to_string()) {
            continue;
        }
        all.push((name.as_bytes().to_vec(), mean));
    }
    assert!(
        all.len() >= N_STATIONS,
        "only {} usable stations",
        all.len()
    );

    // Fisher-Yates, fixed seed so the same list comes out every run.
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    for i in (1..all.len()).rev() {
        let j = (rng.next_u64() % (i as u64 + 1)) as usize;
        all.swap(i, j);
    }
    all.truncate(N_STATIONS);
    all
}

// Append e.g. -12.3 given tenths = -123.
fn push_temp(buf: &mut Vec<u8>, tenths: i32) {
    let (neg, t) = (tenths < 0, tenths.unsigned_abs());
    if neg {
        buf.push(b'-');
    }
    let whole = t / 10;
    if whole >= 100 {
        buf.push(b'0' + (whole / 100) as u8);
    }
    if whole >= 10 {
        buf.push(b'0' + (whole / 10 % 10) as u8);
    }
    buf.push(b'0' + (whole % 10) as u8);
    buf.push(b'.');
    buf.push(b'0' + (t % 10) as u8);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let rows: u64 = args
        .get(1)
        .map(|s| s.replace('_', "").parse().expect("row count"))
        .unwrap_or(1_000_000_000);
    let out = args
        .get(2)
        .map(|s| s.as_str())
        .unwrap_or("measurements.txt");
    let csv = args
        .get(3)
        .map(|s| s.as_str())
        .unwrap_or("weather_stations.csv");

    let stations = load_stations(csv);
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunks = (rows + CHUNK_ROWS as u64 - 1) / CHUNK_ROWS as u64;

    // Bounded queue: producers block when the writer falls behind, so RAM stays flat.
    let (tx, rx) = sync_channel::<Vec<u8>>(threads * 2);
    let stations = &stations;

    std::thread::scope(|s| {
        for t in 0..threads {
            let tx = tx.clone();
            s.spawn(move || {
                let mut rng = Rng(0xDEAD_BEEF_0000_0001 ^ ((t as u64 + 1) << 32));
                let mut c = t as u64;
                while c < chunks {
                    let n = CHUNK_ROWS.min((rows - c * CHUNK_ROWS as u64) as usize);
                    let mut buf: Vec<u8> = Vec::with_capacity(n * 16);
                    for _ in 0..n {
                        let (name, mean) = &stations[(rng.next_u64() as usize) % N_STATIONS];
                        let v = mean + rng.next_normal() * 10.0;
                        let tenths = (v * 10.0).round().clamp(-999.0, 999.0) as i32;
                        buf.extend_from_slice(name);
                        buf.push(b';');
                        push_temp(&mut buf, tenths);
                        buf.push(b'\n');
                    }
                    if tx.send(buf).is_err() {
                        return;
                    }
                    c += threads as u64;
                }
            });
        }
        drop(tx);

        let mut w = BufWriter::with_capacity(1 << 22, File::create(out).expect("create output"));
        for buf in rx {
            w.write_all(&buf).expect("write");
        }
        w.flush().expect("flush");
    });

    eprintln!("wrote {rows} rows to {out}");
}
