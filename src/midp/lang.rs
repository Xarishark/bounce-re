//! `com.nokia.mid.appl.boun.Local`: localized string table (`/lang.xx` etc).
//! Format: table of big-endian u16 offsets followed by Java `writeUTF` strings.

use std::cell::RefCell;

thread_local! {
    static TABLE: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

fn read_utf(d: &[u8], off: usize) -> Option<String> {
    let len = u16::from_be_bytes([*d.get(off)?, *d.get(off + 1)?]) as usize;
    let bytes = d.get(off + 2..off + 2 + len)?;
    // Java modified UTF-8: decode leniently.
    let mut out = String::new();
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b & 0x80 == 0 {
            out.push(b as char);
            i += 1;
        } else if b & 0xe0 == 0xc0 && i + 1 < bytes.len() {
            let c = ((b as u32 & 0x1f) << 6) | (bytes[i + 1] as u32 & 0x3f);
            out.push(char::from_u32(c).unwrap_or('?'));
            i += 2;
        } else if b & 0xf0 == 0xe0 && i + 2 < bytes.len() {
            let c = ((b as u32 & 0x0f) << 12) | ((bytes[i + 1] as u32 & 0x3f) << 6) | (bytes[i + 2] as u32 & 0x3f);
            out.push(char::from_u32(c).unwrap_or('?'));
            i += 3;
        } else {
            out.push('?');
            i += 1;
        }
    }
    Some(out)
}

/// Loads `/lang.<locale>`, falling back to `/lang.xx` like the original.
pub fn init(locale: &str) {
    let data = crate::midp::assets::get(&format!("/lang.{locale}")).or_else(|| crate::midp::assets::get("/lang.xx"));
    let mut v = Vec::new();
    if let Some(d) = data {
        let n = u16::from_be_bytes([d[0], d[1]]) as usize / 2;
        for i in 0..n {
            let off = u16::from_be_bytes([d[i * 2], d[i * 2 + 1]]) as usize;
            v.push(read_utf(d, off).unwrap_or_else(|| "Err".into()));
        }
    }
    TABLE.with(|t| *t.borrow_mut() = v);
}

/// `Local.getText(int)`
pub fn s(i: i32) -> String {
    TABLE.with(|t| {
        let t = t.borrow();
        if t.is_empty() {
            return "NoLang".into();
        }
        t.get(i as usize).cloned().unwrap_or_else(|| "Err".into())
    })
}

/// `Local.getText(int, String[])` with one argument: the first `%U` is replaced.
pub fn s1(i: i32, arg: &str) -> String {
    s(i).replacen("%U", arg, 1)
}
