use std::fs::File;
use std::io::{BufReader, ErrorKind};
use std::io::prelude::*;
use std::{fmt, env};

// (C) Copyright 2020 xkr47@outerspace.dyndns.org
//
// Exercise program in Rust to detect file type of raw audio 16/24 bit PCM files
// Uses algorithm purely invented by xkr47

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() {
        eprintln!("Usage: {} <file.pcm> [...]", env::args().next().unwrap());
        return;
    }
    for file in args {
        let res = investigate(&file);
        if let Ok(res) = res {
            if let Ok(res) = res.guess_type() {
                println!("{}: {} {} {}",
                         file,
                         if res.signed { "signed" } else { "unsigned" },
                         if res.bits24 { "24bit" } else { "16bit" },
                         if res.big_endian { "big-endian" } else { "little-endian" },
                );
            } else {
                println!("{}: unclear {:?}", file, res);
            }
        } else {
            println!("{}: error {}", file, res.unwrap_err());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    extern crate spectral;
    use spectral::prelude::*;

    #[test]
    fn s16le() {
        assert_that(&detect("test-s16.pcm")).is_ok().is_equal_to(PcmType { signed: true, bits24: false, big_endian: false });
    }
    #[test]
    fn s16be() {
        assert_that(&detect("test-s16be.pcm")).is_ok().is_equal_to(PcmType { signed: true, bits24: false, big_endian: true });
    }
    #[test]
    fn u16le() {
        assert_that(&detect("test-u16.pcm")).is_ok().is_equal_to(PcmType { signed: false, bits24: false, big_endian: false });
    }
    #[test]
    fn u16be() {
        assert_that(&detect("test-u16be.pcm")).is_ok().is_equal_to(PcmType { signed: false, bits24: false, big_endian: true });
    }
    #[test]
    fn s24le() {
        assert_that(&detect("test-s24.pcm")).is_ok().is_equal_to(PcmType { signed: true, bits24: true, big_endian: false });
    }
    #[test]
    fn s24be() {
        assert_that(&detect("test-s24be.pcm")).is_ok().is_equal_to(PcmType { signed: true, bits24: true, big_endian: true });
    }
    #[test]
    fn u24le() {
        assert_that(&detect("test-u24.pcm")).is_ok().is_equal_to(PcmType { signed: false, bits24: true, big_endian: false });
    }
    #[test]
    fn u24be() {
        assert_that(&detect("test-u24be.pcm")).is_ok().is_equal_to(PcmType { signed: false, bits24: true, big_endian: true });
    }
}

#[derive(Clone,Copy,Debug,PartialEq,Eq)]
struct PcmType {
    signed: bool,
    bits24: bool,
    big_endian: bool,
}

#[derive(Debug)]
struct PcmResults {
    s16le: f64,
    s16be: f64,
    u16le: f64,
    u16be: f64,
    s24le: f64,
    s24be: f64,
    u24le: f64,
    u24be: f64,
}

impl PcmResults {
    // how much more sure must we be of the most likely outcome compared to the second most likely
    const THRESHOLD: f64 = 4.0;

    fn guess_type(&self) -> Result<PcmType, String> {
        let mut res = vec!(
            (PcmType { signed: true, bits24: false, big_endian: false }, self.s16le),
            (PcmType { signed: true, bits24: false, big_endian: true }, self.s16be),
            (PcmType { signed: false, bits24: false, big_endian: false }, self.u16le),
            (PcmType { signed: false, bits24: false, big_endian: true }, self.u16be),
            (PcmType { signed: true, bits24: true, big_endian: false }, self.s24le),
            (PcmType { signed: true, bits24: true, big_endian: true }, self.s24be),
            (PcmType { signed: false, bits24: true, big_endian: false }, self.u24le),
            (PcmType { signed: false, bits24: true, big_endian: true }, self.u24be),
        );
        res.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        if res[0].1 / res[1].1 < PcmResults::THRESHOLD {
            Err(format!("Below threshold for {:?} vs {:?}", res[0], res[1]))
        } else {
            Ok(res[0].0)
        }
    }
}

struct Avg {
    sum: i64,
    diffsum: u64,
    count: u32,
    last: i8,
    debug: bool,
}

impl Avg {
    fn new() -> Avg {
        Avg { sum: 0, diffsum: 0, count: 0, last: 0, debug: false }
    }
    fn _newd() -> Avg {
        Avg { sum: 0, diffsum: 0, count: 0, last: 0, debug: true }
    }

    fn add(&mut self, value: i8) {
        if self.count > 0 {
            self.sum += value as i64;
            self.diffsum += (value as i64 - self.last as i64).abs() as u64;
        }
        self.count = self.count+1;
        self.last = value;
        if self.debug && self.count < 10000 {
            println!("{}. {} {} {}", self.count, value, self.diffsum, self.diffavg());
        }
    }

    fn _avg(&self) -> f64 {
        self.sum as f64 / self.count as f64
    }

    fn diffavg(&self) -> f64 {
        self.diffsum as f64 / self.count as f64
    }
}

#[derive(Clone,Copy,Debug)]
struct Stereo<T: Copy + std::fmt::Debug> {
    l: T,
    r: T
}

struct Avg2 {
    l: Avg,
    r: Avg
}

impl Avg2 {
    fn new() -> Avg2 { Avg2 { l: Avg::new(), r: Avg::new() } }
    fn _newd() -> Avg2 { Avg2 { l: Avg::_newd(), r: Avg::_newd() } }

    fn add(&mut self, l: i8, r: i8) {
        self.l.add(l);
        self.r.add(r);
    }

    fn _avg(&self) -> Stereo<f64> {
        Stereo { l: self.l._avg(), r: self.r._avg() }
    }

    fn diffavg(&self) -> Stereo<f64> {
        Stereo { l: self.l.diffavg(), r: self.r.diffavg() }
    }
}

impl fmt::Display for Avg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} / {}", self._avg(), self.diffavg())
    }
}

impl fmt::Debug for Avg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //write!(f, "{} / {} sum {} diffsum {} count {}", self.avg(), self.diffavg(), self.sum, self.diffsum, self.count)
        write!(f, "{}", self.diffavg())
    }
}

fn investigate(filename: &str) -> std::io::Result<PcmResults> {
    let file = File::open(filename)?;
    let meta = file.metadata()?;
    if !meta.is_file() {
        return Err(std::io::Error::from(ErrorKind::InvalidInput));
    }
    /*
        match meta.len() / 2 % 6 {
            2 | 4 => return Ok(Signed16),
            3 => return Ok(Signed24),
            1 | 5 => panic!("Bad file {} length {}", filename, meta.len()),
            _ => (),
        }
    */
    let mut buf_reader = BufReader::new(file);

    let mut a16: [[Avg; 2]; 2] = [[Avg::new(), Avg::new()], [Avg::new(), Avg::new()]];
    let mut a24: [[Avg2; 3]; 2] = [ // [bit7-inv][byte]
        [Avg2::new(), Avg2::new(), Avg2::new()], [Avg2::new(), Avg2::new(), Avg2::new()],
    ];
    let mut buf = [0_u8; 12];
    let mut bufs = [[0_i8; 12]; 2];

    while let Ok(()) = buf_reader.read_exact(&mut buf) {
        for i in 0..buf.len() {
            let v = buf[i] as i8;
            bufs[0][i] = v;
            bufs[1][i] = v.wrapping_add(-128);
        }
        for toggled in 0..=1 { // 0 = original, 1 = bit 7 toggled
            for sample in 0..=2 {
                for byte in 0..=1 {
                    a16[toggled][byte].add(bufs[toggled][sample * 4 + byte]);
                }
            }
            for sample in 0..=1 {
                for byte in 0..=2 {
                    if toggled == 0 || byte != 1 {
                        a24[toggled][byte].add(bufs[toggled][sample * 6 + byte], bufs[toggled][sample * 6 + 3 + byte]);
                    }
                }
            }
        }
    };
    let v16 = [[a16[0][0].diffavg(), a16[0][1].diffavg()], [a16[1][0].diffavg(), a16[1][1].diffavg()]];
    let v24i = [
        [
            a24[0][0].diffavg(),
            a24[0][1].diffavg(),
            a24[0][2].diffavg()
        ], [
            a24[1][0].diffavg(),
            Stereo { l: 0.0, r: 0.0 }, /* 7-bit toggled middle byte not needed */
            a24[1][2].diffavg()
        ]
    ];
    // the inner if expressions below are to handle 16 bit files that have been converted to 24 bit by filling lsbs with zeroes
    let v24= [
        [
            Stereo { l: if v24i[0][0].l <= 0.0 { v24i[0][1].l } else { v24i[0][0].l }, r: if v24i[0][0].r <= 0.0 { v24i[0][1].r } else { v24i[0][0].r } },
            v24i[0][1],
            Stereo { l: if v24i[0][2].l <= 0.0 { v24i[0][1].l } else { v24i[0][2].l }, r: if v24i[0][2].r <= 0.0 { v24i[0][1].r } else { v24i[0][2].r } },
        ], [
            Stereo { l: if v24i[1][0].l <= 0.0 { v24i[0][1].l } else { v24i[1][0].l }, r: if v24i[1][0].r <= 0.0 { v24i[0][1].r } else { v24i[1][0].r } },
            v24i[1][1], // not needed
            Stereo { l: if v24i[1][2].l <= 0.0 { v24i[0][1].l } else { v24i[1][2].l }, r: if v24i[1][2].r <= 0.0 { v24i[0][1].r } else { v24i[1][2].r } },
        ]
    ];
    //println!("v16 signed {:?} unsigned {:?}", v16[0], v16[1]);
    //println!("v24 signed {:?} unsigned {:?}", v24[0], v24[1]);
    let results = PcmResults {
        s16le: if v16[0][1] > 0.0 { v16[0][0] / v16[0][1] * v16[1][1] } else { 0. },
        s16be: if v16[0][0] > 0.0 { v16[0][1] / v16[0][0] * v16[1][0] } else { 0. },
        u16le: if v16[1][1] > 0.0 { v16[0][0] / v16[1][1] * v16[0][1] } else { 0. },
        u16be: if v16[1][0] > 0.0 { v16[0][1] / v16[1][0] * v16[0][0] } else { 0. },
        s24le: (if v24[0][2].l > 0.0 { v24[0][1].l / v24[0][2].l * v24[1][2].l } else { 0. }).min(if v24[0][2].r > 0.0 { v24[0][1].r / v24[0][2].r * v24[1][2].r } else { 0. }),
        s24be: (if v24[0][0].l > 0.0 { v24[0][1].l / v24[0][0].l * v24[1][0].l } else { 0. }).min(if v24[0][0].r > 0.0 { v24[0][1].r / v24[0][0].r * v24[1][0].r } else { 0. }),
        u24le: (if v24[1][2].l > 0.0 { v24[0][1].l / v24[1][2].l * v24[0][2].l } else { 0. }).min(if v24[1][2].r > 0.0 { v24[0][1].r / v24[1][2].r * v24[0][2].r } else { 0. }),
        u24be: (if v24[1][0].l > 0.0 { v24[0][1].l / v24[1][0].l * v24[0][0].l } else { 0. }).min(if v24[1][0].r > 0.0 { v24[0][1].r / v24[1][0].r * v24[0][0].r } else { 0. }),
    };
    //println!("Res {:?}", results);
    Ok(results)
}
