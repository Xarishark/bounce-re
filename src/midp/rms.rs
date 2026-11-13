//! `javax.microedition.rms.RecordStore` backed by one file per store in the
//! `save` folder (see `paths`): each record as a big-endian u32 length and
//! its bytes, in record-id order (ids start at 1, nothing is ever deleted).

use std::fs;
use std::path::PathBuf;

fn path(name: &str) -> PathBuf {
    crate::paths::save_dir().join(format!("{name}.rms"))
}

pub struct RecordStore {
    name: String,
    records: Vec<Vec<u8>>,
}

impl RecordStore {
    /// `openRecordStore(name, true)`
    pub fn open(name: &str) -> RecordStore {
        let mut records = Vec::new();
        if let Ok(d) = fs::read(path(name)) {
            let mut i = 0;
            while i + 4 <= d.len() {
                let n = u32::from_be_bytes([d[i], d[i + 1], d[i + 2], d[i + 3]]) as usize;
                i += 4;
                if i + n > d.len() {
                    break;
                }
                records.push(d[i..i + n].to_vec());
                i += n;
            }
        }
        RecordStore { name: name.into(), records }
    }

    fn flush(&self) {
        let mut out = Vec::new();
        for r in &self.records {
            out.extend_from_slice(&(r.len() as u32).to_be_bytes());
            out.extend_from_slice(r);
        }
        let p = path(&self.name);
        let tmp = p.with_extension("tmp");
        let ok = fs::create_dir_all(crate::paths::save_dir()).and_then(|_| fs::write(&tmp, &out)).and_then(|_| fs::rename(&tmp, &p));
        if let Err(e) = ok {
            crate::log!("couldn't save {}: {e}", p.display());
        }
    }

    pub fn get_num_records(&self) -> i32 {
        self.records.len() as i32
    }

    pub fn add_record(&mut self, data: &[u8]) -> i32 {
        self.records.push(data.to_vec());
        self.flush();
        self.records.len() as i32
    }

    /// `getRecord(id)`; None stands for InvalidRecordIDException.
    pub fn get_record(&self, id: i32) -> Option<Vec<u8>> {
        self.records.get((id - 1).max(0) as usize).filter(|_| id >= 1).cloned()
    }

    /// `setRecord(id, data)`; false stands for InvalidRecordIDException.
    pub fn set_record(&mut self, id: i32, data: &[u8]) -> bool {
        if id < 1 || id as usize > self.records.len() {
            return false;
        }
        self.records[id as usize - 1] = data.to_vec();
        self.flush();
        true
    }
}

/// `java.io.DataInputStream` over a byte array; reading past the end is the
/// EOFException (None).
pub struct DataIn<'a> {
    d: &'a [u8],
    pub pos: usize,
}

impl<'a> DataIn<'a> {
    pub fn new(d: &'a [u8]) -> Self {
        DataIn { d, pos: 0 }
    }
    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let s = self.d.get(self.pos..self.pos + n)?;
        self.pos += n;
        Some(s)
    }
    /// `read()`: an unsigned byte, or -1 at the end
    pub fn read(&mut self) -> i32 {
        self.take(1).map(|b| b[0] as i32).unwrap_or(-1)
    }
    pub fn read_byte(&mut self) -> Option<i32> {
        self.take(1).map(|b| b[0] as i8 as i32)
    }
    pub fn read_short(&mut self) -> Option<i32> {
        self.take(2).map(|b| i16::from_be_bytes([b[0], b[1]]) as i32)
    }
    pub fn read_int(&mut self) -> Option<i32> {
        self.take(4).map(|b| i32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    }
}

/// `java.io.DataOutputStream` into a byte array.
#[derive(Default)]
pub struct DataOut(pub Vec<u8>);

impl DataOut {
    pub fn write_byte(&mut self, v: i32) {
        self.0.push(v as u8);
    }
    pub fn write_short(&mut self, v: i32) {
        self.0.extend_from_slice(&(v as i16).to_be_bytes());
    }
    pub fn write_int(&mut self, v: i32) {
        self.0.extend_from_slice(&v.to_be_bytes());
    }
}
