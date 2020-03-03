use std::fs::File;
use std::io::BufReader;
use std::io::prelude::*;
use crate::PcmType::{Signed16, Signed24};
use std::fmt;

#[derive(Debug)]
enum PcmType {
    Signed16,
    Unsigned16,
    Signed24,
    Unsigned24,
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

fn main() {
    /*
    let file = "test.pcm";
    let pcm_type = detect(file).expect(&format!("Failed to detect type of file {}", file));
    println!("Type of {} seems to be {:?}", file, pcm_type);
    */
    for file in [ "test-s16.pcm", "test-s16be.pcm", "test-u16.pcm", "test-u16be.pcm", "test-s24.pcm", "test-s24be.pcm", "test-u24.pcm", "test-u24be.pcm" ].iter() {
        println!("---- {}", file);
        detect(file).unwrap();
    }

}

struct Avg {
    sum: i64,
    diffsum: u64,
    count: u32,
    last: i8,
}

impl Avg {
    fn new() -> Avg {
        Avg { sum: 0, diffsum: 0, count: 0, last: 0 }
    }

    fn add(&mut self, value: i8) {
        self.sum += value as i64;
        self.count = self.count+1;
        self.diffsum += (value as i64 - self.last as i64).abs() as u64;
        self.last = value;
    }

    fn avg(&self) -> f64 {
        self.sum as f64 / self.count as f64
    }

    fn diffavg(&self) -> f64 {
        self.diffsum as f64 / self.count as f64
    }
}

impl fmt::Display for Avg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} / {}", self.avg(), self.diffavg())
    }
}

impl fmt::Debug for Avg {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        //write!(f, "{} / {} sum {} diffsum {} count {}", self.avg(), self.diffavg(), self.sum, self.diffsum, self.count)
        write!(f, "{}", self.diffavg())
    }
}

fn detect(filename: &str) -> std::io::Result<PcmType> {
    let file = File::open(filename)?;
    /*
    let meta = file.metadata()?;
    match meta.len() / 2 % 6 {
        2 | 4 => return Ok(Signed16),
        3 => return Ok(Signed24),
        1 | 5 => panic!("Bad file {} length {}", filename, meta.len()),
        _ => (),
    }
*/
    let mut buf_reader = BufReader::new(file);

    let mut a16: [[Avg; 2]; 2] = [[Avg::new(), Avg::new()], [Avg::new(), Avg::new()]];
    let mut a24: [[Avg; 3]; 2] = [[Avg::new(), Avg::new(), Avg::new()], [Avg::new(), Avg::new(), Avg::new()]];
    let mut buf = [0_u8; 12];
    let mut bufs = [[0_i8; 12]; 2];

    while let Ok(()) = buf_reader.read_exact(&mut buf) {
        for i in 0..buf.len() {
            bufs[0][i] = buf[i] as i8;
            bufs[1][i] = bufs[0][i].wrapping_add(-128);
        }
        for toggled in 0..=1 { // 0 = original, 1 = bit 7 toggled
            for sample in 0..=2 {
                for byte in 0..=1 {
                    a16[toggled][byte].add(bufs[toggled][sample * 4 + byte]);
                }
            }
            for sample in 0..=1 {
                for byte in 0..=2 {
                    //if toggled == 0 || byte != 1 { // TODO uncomment when done and ensure no change
                        a24[toggled][byte].add(bufs[toggled][sample * 6 + byte]);
                    //}
                }
            }
        }
    };
    let v16 = [[a16[0][0].diffavg(), a16[0][1].diffavg()], [a16[1][0].diffavg(), a16[1][1].diffavg()]];
    let v24i = [[a24[0][0].diffavg(), a24[0][1].diffavg(), a24[0][2].diffavg()], [a24[1][0].diffavg(), 0.0 /* not needed */, a24[1][2].diffavg()]];
    // the inner if expressions below are to handle 16 bit files that have been converted to 24 bit by filling lsbs with zeroes
    let v24 = [
        [
            if v24i[0][0] <= 0.0 { v24i[0][1] } else { v24i[0][0] },
            v24i[0][1],
            if v24i[0][2] <= 0.0 { v24i[0][1] } else { v24i[0][2] },
        ], [
            v24i[1][0],
            0.0, // not needed
            v24i[1][2]
        ]
    ];
    println!("v16 signed {:?} unsigned {:?}", v16[0], v16[1]);
    println!("v24 signed {:?} unsigned {:?}", v24[0], v24[1]);
    println!("Res {:?}", PcmResults {
        s16le: if v16[0][1] > 0.0 { v16[0][0] / v16[0][1] * v16[1][1] } else { 1000. },
        s16be: if v16[0][0] > 0.0 { v16[0][1] / v16[0][0] * v16[1][0] } else { 1000. },
        u16le: if v16[1][1] > 0.0 { v16[0][0] / v16[1][1] * v16[0][1] } else { 1000. },
        u16be: if v16[1][0] > 0.0 { v16[0][1] / v16[1][0] * v16[0][0] } else { 1000. },
        s24le: if v24[0][2] > 0.0 { v24[0][1] / v24[0][2] * v24[1][2] } else { 1000. },
        s24be: if v24[0][0] > 0.0 { v24[0][1] / v24[0][0] * v24[1][0] } else { 1000. },
        u24le: if v24[1][2] > 0.0 { v24[0][1] / v24[1][2] * v24[0][2] } else { 1000. },
        u24be: if v24[1][0] > 0.0 { v24[0][1] / v24[1][0] * v24[0][0] } else { 1000. },
    });
    Ok(Signed24)
}
