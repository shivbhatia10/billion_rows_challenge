use std::collections::HashMap;
use std::ffi::c_void;
use std::fmt::Write;
use std::fs::File;
use std::hash::{BuildHasherDefault, Hasher};
use std::io;
use std::os::unix::io::AsRawFd;
use std::thread::ScopedJoinHandle;

const FILE_NAME: &'static str = "measurements.txt";
const K: u64 = 0x517c_c1b7_2722_0a95;

const PROT_READ: i32 = 1;
const MAP_PRIVATE: i32 = 2;

const MADV_SEQUENTIAL: i32 = 2;

unsafe extern "C" {
    unsafe fn madvise(addr: *mut c_void, len: usize, advice: i32) -> i32;
}

unsafe extern "C" {
    unsafe fn mmap(
        addr: *mut c_void,
        len: usize,
        prot: i32,
        flags: i32,
        fd: i32,
        offset: i64,
    ) -> *mut c_void;
}

fn map_file(path: &str) -> io::Result<&'static [u8]> {
    let file = File::open(path)?;
    let len = file.metadata()?.len() as usize;
    let ptr = unsafe {
        mmap(
            std::ptr::null_mut(),
            len,
            PROT_READ,
            MAP_PRIVATE,
            file.as_raw_fd(),
            0,
        )
    };
    if ptr as isize == -1 {
        return Err(io::Error::last_os_error());
    }
    unsafe {
        madvise(ptr, len, MADV_SEQUENTIAL);
    }
    Ok(unsafe { std::slice::from_raw_parts(ptr as *const u8, len) })
}

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

// (min, total, max, count)
type Acc = (i16, i64, i16, u32);

pub fn main() -> Result<(), io::Error> {
    let data = map_file(FILE_NAME)?;

    let len = data.len();

    const T: usize = 8;

    let maps: Vec<_> = std::thread::scope(|s| {
        let mut handles: Vec<ScopedJoinHandle<CustomHashMap<&'static [u8], Acc>>> = Vec::new();

        for t in 0..T {
            let handle = s.spawn(move || {
                let mut accs: CustomHashMap<&'static [u8], Acc> = CustomHashMap::default();

                let mut pos = len * t / T;
                if pos > 0 {
                    while data[pos - 1] != b'\n' {
                        pos += 1;
                    }
                }

                while pos < len * (t + 1) / T {
                    let mut sc = pos;
                    while data[sc] != b';' {
                        sc += 1;
                    }
                    let name = &data[pos..sc];

                    let vs = sc + 1;
                    let neg = (data[vs] == b'-') as usize;
                    let d0 = vs + neg;
                    let two = (data[d0 + 1] != b'.') as usize;
                    let mag = if two == 1 {
                        (data[d0] - b'0') as i32 * 100
                            + (data[d0 + 1] - b'0') as i32 * 10
                            + (data[d0 + 3] - b'0') as i32
                    } else {
                        (data[d0] - b'0') as i32 * 10 + (data[d0 + 2] - b'0') as i32
                    };
                    let n = neg as i32;
                    let v = ((mag ^ -n) + n) as i16;
                    pos = d0 + two + 4;

                    match accs.get_mut(name) {
                        Some(e) => {
                            e.0 = e.0.min(v);
                            e.1 += v as i64;
                            e.2 = e.2.max(v);
                            e.3 += 1;
                        }
                        None => {
                            accs.insert(name, (v, v as i64, v, 1));
                        }
                    }
                }
                accs
            });
            handles.push(handle);
        }

        let mut out: Vec<CustomHashMap<&'static [u8], Acc>> = Vec::new();
        for h in handles {
            out.push(h.join().unwrap());
        }
        out
    });

    let mut accs: CustomHashMap<&'static [u8], Acc> = CustomHashMap::default();
    for m in maps {
        for (k, v) in m {
            match accs.get_mut(k) {
                Some(e) => {
                    e.0 = e.0.min(v.0);
                    e.1 += v.1;
                    e.2 = e.2.max(v.2);
                    e.3 += v.3;
                }
                None => {
                    accs.insert(k, v);
                }
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
