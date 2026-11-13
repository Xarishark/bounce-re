//! Small proportional pixel font standing in for the Nokia S40 "small" font.
//! Glyphs come from embedded-graphics' 5x8 ISO-8859-1 font, trimmed to
//! their inked width so text looks proportional like the phone's font.

use embedded_graphics::{
    mono_font::{iso_8859_1::FONT_5X8, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Baseline, Text},
};
use std::collections::HashMap;

struct Glyph {
    w: i32,
    bits: Vec<bool>, // w * CELL_H
}

pub struct Font {
    glyphs: HashMap<char, Glyph>,
    /// Box height used for anchors (BOTTOM) and line spacing.
    pub height: i32,
    pub ascent: i32,
}

const CELL_W: usize = 5;
const CELL_H: usize = 8;
/// Vertical offset of glyphs inside the font box (Nokia font has 1px leading).
const Y_OFF: i32 = 1;

struct Buf {
    px: [bool; CELL_W * CELL_H],
}
impl OriginDimensions for Buf {
    fn size(&self) -> Size {
        Size::new(CELL_W as u32, CELL_H as u32)
    }
}
impl DrawTarget for Buf {
    type Color = BinaryColor;
    type Error = core::convert::Infallible;
    fn draw_iter<I: IntoIterator<Item = Pixel<BinaryColor>>>(&mut self, pixels: I) -> Result<(), Self::Error> {
        for Pixel(p, c) in pixels {
            if c.is_on() && p.x >= 0 && p.y >= 0 && (p.x as usize) < CELL_W && (p.y as usize) < CELL_H {
                self.px[p.y as usize * CELL_W + p.x as usize] = true;
            }
        }
        Ok(())
    }
}

impl Font {
    pub fn new() -> Font {
        let style = MonoTextStyle::new(&FONT_5X8, BinaryColor::On);
        let mut glyphs = HashMap::new();
        for code in (32u32..127).chain(160..256) {
            let ch = char::from_u32(code).unwrap();
            let mut buf = Buf { px: [false; CELL_W * CELL_H] };
            let mut tmp = [0u8; 4];
            let s = ch.encode_utf8(&mut tmp);
            let _ = Text::with_baseline(s, Point::new(0, 0), style, Baseline::Top).draw(&mut buf);
            let mut lo = CELL_W;
            let mut hi = 0;
            for x in 0..CELL_W {
                if (0..CELL_H).any(|y| buf.px[y * CELL_W + x]) {
                    lo = lo.min(x);
                    hi = hi.max(x);
                }
            }
            let g = if lo > hi {
                Glyph { w: 2, bits: vec![false; 2 * CELL_H] }
            } else {
                let w = hi - lo + 1;
                let mut bits = vec![false; w * CELL_H];
                for y in 0..CELL_H {
                    for x in 0..w {
                        bits[y * w + x] = buf.px[y * CELL_W + lo + x];
                    }
                }
                Glyph { w: w as i32, bits }
            };
            glyphs.insert(ch, g);
        }
        Font { glyphs, height: 10, ascent: 8 }
    }

    fn glyph(&self, c: char) -> &Glyph {
        self.glyphs.get(&c).or_else(|| self.glyphs.get(&'?')).unwrap()
    }

    pub fn char_width(&self, c: char) -> i32 {
        self.glyph(c).w + 1
    }

    pub fn width(&self, s: &str) -> i32 {
        let w: i32 = s.chars().map(|c| self.char_width(c)).sum();
        (w - 1).max(0)
    }

    pub fn draw(&self, s: &str, x: i32, y: i32, mut put: impl FnMut(i32, i32)) {
        let mut cx = x;
        for c in s.chars() {
            let g = self.glyph(c);
            for gy in 0..CELL_H as i32 {
                for gx in 0..g.w {
                    if g.bits[(gy * g.w + gx) as usize] {
                        put(cx + gx, y + Y_OFF + gy);
                    }
                }
            }
            cx += g.w + 1;
        }
    }

    /// Greedy word wrap to `max_w` pixels.
    pub fn wrap(&self, text: &str, max_w: i32) -> Vec<String> {
        let mut lines = Vec::new();
        for para in text.split('\n') {
            let mut line = String::new();
            for word in para.split(' ') {
                let cand = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
                if self.width(&cand) <= max_w || line.is_empty() {
                    if self.width(&cand) > max_w {
                        // hard-break an over-long word
                        let mut part = String::new();
                        for ch in cand.chars() {
                            let t = format!("{part}{ch}");
                            if self.width(&t) > max_w && !part.is_empty() {
                                lines.push(std::mem::take(&mut part));
                                part.push(ch);
                            } else {
                                part = t;
                            }
                        }
                        line = part;
                    } else {
                        line = cand;
                    }
                } else {
                    lines.push(std::mem::take(&mut line));
                    line = word.to_string();
                }
            }
            lines.push(line);
        }
        lines
    }
}
