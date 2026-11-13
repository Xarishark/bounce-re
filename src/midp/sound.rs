//! `com.nokia.mid.sound.Sound` for `FORMAT_TONE` data: Nokia Smart Messaging
//! ringtones (`.ott`), parsed into notes and played as a square wave on the
//! phone's single tone channel (a new sound cuts off the one playing).

pub const RATE: i32 = 44100;

/// Tempo table of the Smart Messaging spec (beats per minute).
const BPM: [u32; 32] = [
    25, 28, 31, 35, 40, 45, 50, 56, 63, 70, 80, 90, 100, 112, 125, 140, 160, 180, 200, 225, 250, 285, 320, 355, 400, 450, 500,
    565, 635, 715, 800, 900,
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tone {
    /// Hz; 0 is a rest
    pub freq: f32,
    /// seconds the tone sounds
    pub on: f32,
    /// seconds of silence after it (style "natural" / "staccato")
    pub off: f32,
    /// 0..=15
    pub volume: u8,
}

struct Bits<'a> {
    d: &'a [u8],
    pos: usize,
}

impl Bits<'_> {
    fn get(&mut self, n: usize) -> Option<u32> {
        let mut v = 0;
        for _ in 0..n {
            let byte = *self.d.get(self.pos / 8)?;
            let bit = (byte >> (7 - self.pos % 8)) & 1;
            v = v << 1 | bit as u32;
            self.pos += 1;
        }
        Some(v)
    }
    fn align(&mut self) {
        self.pos = self.pos.div_ceil(8) * 8;
    }
}

/// Parse an OTT ringtone (basic song) into tones.
pub fn parse_ott(d: &[u8]) -> Option<Vec<Tone>> {
    let mut b = Bits { d, pos: 0 };
    let _parts = b.get(8)?;
    let mut cmd = b.get(7)?;
    if cmd == 0b0100101 {
        // <ringing-tone-programming> is followed by a filler to the octet
        b.align();
        cmd = b.get(7)?;
    }
    if cmd != 0b0011101 {
        return None; // not <sound>
    }
    let song_type = b.get(3)?;
    if song_type != 1 {
        return None; // only <basic-song-type> is used by phones' games
    }
    let title_len = b.get(4)? as usize;
    for _ in 0..title_len {
        b.get(8)?;
    }
    let patterns = b.get(8)?;
    let mut out = Vec::new();
    let (mut scale, mut style, mut bpm, mut volume) = (2u32, 0u32, 63u32, 7u8);
    for _ in 0..patterns {
        let _hdr = b.get(3)?;
        let _id = b.get(2)?;
        let loops = b.get(4)?;
        let n = b.get(8)?;
        let mut pat = Vec::new();
        for _ in 0..n {
            match b.get(3)? {
                0b001 => {
                    let note = b.get(4)?;
                    let dur = b.get(3)?;
                    let spec = b.get(2)?;
                    // whole note = 4 beats
                    let mut len = 4.0 * 60.0 / bpm as f32 / (1u32 << dur.min(5)) as f32;
                    len *= match spec {
                        1 => 1.5,
                        2 => 1.75,
                        3 => 2.0 / 3.0,
                        _ => 1.0,
                    };
                    let freq = if (1..=12).contains(&note) {
                        // scale 0 has A at 440 Hz (1 = 880, the default; 2 = 1760; 3 = 3520);
                        // note 1 is the C below that A
                        let a = 440.0 * (1u32 << scale) as f32;
                        a * 2f32.powf((note as f32 - 10.0) / 12.0)
                    } else {
                        0.0
                    };
                    let off = match style {
                        0 => len / 16.0, // natural
                        2 => len / 2.0,  // staccato
                        _ => 0.0,        // continuous
                    };
                    pat.push(Tone { freq, on: len - off, off, volume });
                }
                0b010 => scale = b.get(2)?,
                0b011 => style = b.get(2)?,
                0b100 => bpm = BPM[b.get(5)? as usize],
                0b101 => volume = b.get(4)? as u8,
                _ => return if out.is_empty() && pat.is_empty() { None } else { Some([out, pat].concat()) },
            }
        }
        // loop value 15 means forever; play such patterns once
        let times = if loops == 15 { 1 } else { loops + 1 };
        for _ in 0..times {
            out.extend_from_slice(&pat);
        }
    }
    Some(out)
}

/// A loaded `Sound`.
#[derive(Clone)]
pub struct Sound {
    tones: std::rc::Rc<Vec<Tone>>,
}

impl Sound {
    /// `new Sound(data, Sound.FORMAT_TONE)`; an unparsable file plays nothing.
    pub fn new(data: &[u8]) -> Sound {
        let tones = parse_ott(data).unwrap_or_else(|| {
            if !data.is_empty() {
                crate::log!("unsupported tone data ({} bytes)", data.len());
            }
            Vec::new()
        });
        Sound { tones: std::rc::Rc::new(tones) }
    }
}

/// The phone's tone generator.
pub struct Tones {
    playing: Option<(std::rc::Rc<Vec<Tone>>, usize, f32)>,
    phase: f32,
    pub volume: f32,
    pub muted: bool,
}

impl Tones {
    pub fn new() -> Tones {
        Tones { playing: None, phase: 0.0, volume: 1.0, muted: false }
    }

    /// `Sound.play(1)`
    pub fn play(&mut self, s: &Sound) {
        self.playing = Some((s.tones.clone(), 0, 0.0));
    }

    /// Advance by `frames` samples, writing interleaved stereo into `out`.
    pub fn render(&mut self, frames: usize, out: &mut Vec<f32>) {
        out.clear();
        out.resize(frames * 2, 0.0);
        let dt = 1.0 / RATE as f32;
        let gain = if self.muted { 0.0 } else { 0.12 * self.volume };
        for i in 0..frames {
            let Some((tones, idx, t)) = self.playing.as_mut() else { break };
            let Some(tone) = tones.get(*idx).copied() else {
                self.playing = None;
                break;
            };
            if *t < tone.on && tone.freq > 0.0 {
                self.phase = (self.phase + tone.freq * dt).fract();
                let v = if self.phase < 0.5 { 1.0 } else { -1.0 };
                let s = v * gain * tone.volume as f32 / 15.0;
                out[i * 2] = s;
                out[i * 2 + 1] = s;
            }
            *t += dt;
            if *t >= tone.on + tone.off {
                *idx += 1;
                *t = 0.0;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Writes bit fields MSB first, like the OTT format.
    fn bits(fields: &[(u32, usize)]) -> Vec<u8> {
        let mut out = Vec::new();
        let mut n = 0;
        for &(v, w) in fields {
            for i in (0..w).rev() {
                if n % 8 == 0 {
                    out.push(0);
                }
                *out.last_mut().unwrap() |= (((v >> i) & 1) as u8) << (7 - n % 8);
                n += 1;
            }
        }
        out
    }

    #[test]
    fn parses_a_made_up_ringtone() {
        // two command parts; programming + filler; sound; basic song; no title; one pattern
        // of three instructions: tempo 140 bpm (index 15), scale 1, a quarter-note A
        let d = bits(&[
            (2, 8),
            (0b0100101, 7),
            (0, 1),
            (0b0011101, 7),
            (1, 3),
            (0, 4),
            (1, 8),
            (0, 3),
            (0, 2),
            (0, 4),
            (3, 8),
            (0b100, 3),
            (15, 5),
            (0b010, 3),
            (1, 2),
            (0b001, 3),
            (10, 4),
            (2, 3),
            (0, 2),
        ]);
        let t = parse_ott(&d).expect("parses");
        assert_eq!(t.len(), 1);
        assert!((t[0].freq - 880.0).abs() < 0.5, "{t:?}");
        // quarter note at 140 bpm, style natural (1/16 of it silent)
        assert!((t[0].on + t[0].off - 60.0 / 140.0).abs() < 1e-4, "{t:?}");
    }
}

#[cfg(test)]
mod jar_tests {
    /// Prints the three effects from the player's JAR, when it's in game-files/.
    #[test]
    fn decode_game_sounds() {
        let jar = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("game-files/Bounce.jar");
        let Ok(f) = std::fs::File::open(&jar) else { return };
        let mut z = zip::ZipArchive::new(f).unwrap();
        for name in ["sounds/pop.ott", "sounds/pickup.ott", "sounds/up.ott"] {
            let mut d = Vec::new();
            std::io::Read::read_to_end(&mut z.by_name(name).unwrap(), &mut d).unwrap();
            let t = super::parse_ott(&d).unwrap_or_else(|| panic!("{name} doesn't parse"));
            assert!(!t.is_empty() && t.iter().any(|x| x.freq > 0.0), "{name}: {t:?}");
            let total: f32 = t.iter().map(|x| x.on + x.off).sum();
            println!("{name}: {} notes, {total:.2}s: {:?}", t.len(), t.iter().map(|x| x.freq.round() as i32).collect::<Vec<_>>());
        }
    }
}
