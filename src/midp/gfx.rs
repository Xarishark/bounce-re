//! Minimal re-implementation of the MIDP 1.0 `Image` / `Graphics` classes and
//! the Nokia UI extensions Bounce uses (`DirectUtils.createImage` with a fill
//! colour, `DirectGraphics.drawImage` with flips and rotations).
//!
//! Every image is ARGB8888. A mutable image (`Image.createImage(w, h)`) is a
//! [`Surface`]: the pixels plus the state of the one `Graphics` the game keeps
//! for it (colour, clip, font).

use crate::midp::font::Font;
use std::rc::Rc;

pub const SCREEN_W: i32 = 128;
pub const SCREEN_H: i32 = 128;

// Anchor constants
pub const HCENTER: i32 = 1;
pub const VCENTER: i32 = 2;
pub const LEFT: i32 = 4;
pub const RIGHT: i32 = 8;
pub const TOP: i32 = 16;
pub const BOTTOM: i32 = 32;
pub const BASELINE: i32 = 64;

// com.nokia.mid.ui.DirectGraphics manipulations
pub const FLIP_HORIZONTAL: i32 = 8192;
pub const FLIP_VERTICAL: i32 = 16384;

#[derive(Clone)]
pub struct Image {
    pub w: i32,
    pub h: i32,
    /// ARGB8888
    pub px: Vec<u32>,
}
pub type Img = Rc<Image>;

impl Image {
    /// A `w` x `h` image filled with `argb`.
    pub fn filled(w: i32, h: i32, argb: u32) -> Image {
        Image { w, h, px: vec![argb; (w.max(0) * h.max(0)) as usize] }
    }

    pub fn from_png(data: &[u8]) -> Option<Image> {
        let mut dec = png::Decoder::new(std::io::Cursor::new(data));
        dec.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = dec.read_info().ok()?;
        let mut buf = vec![0u8; reader.output_buffer_size()?];
        let info = reader.next_frame(&mut buf).ok()?;
        let (w, h) = (info.width as usize, info.height as usize);
        let bpp = match info.color_type {
            png::ColorType::Grayscale => 1,
            png::ColorType::GrayscaleAlpha => 2,
            png::ColorType::Rgb => 3,
            png::ColorType::Rgba => 4,
            png::ColorType::Indexed => return None,
        };
        let line = info.line_size;
        let mut px = Vec::with_capacity(w * h);
        for y in 0..h {
            let row = &buf[y * line..y * line + w * bpp];
            for x in 0..w {
                let p = &row[x * bpp..x * bpp + bpp];
                let (r, g, b, a) = match bpp {
                    1 => (p[0], p[0], p[0], 255),
                    2 => (p[0], p[0], p[0], p[1]),
                    3 => (p[0], p[1], p[2], 255),
                    _ => (p[0], p[1], p[2], p[3]),
                };
                px.push((a as u32) << 24 | (r as u32) << 16 | (g as u32) << 8 | b as u32);
            }
        }
        Some(Image { w: w as i32, h: h as i32, px })
    }
}

#[inline]
fn blend(dst: u32, src: u32) -> u32 {
    let a = src >> 24;
    if a == 255 {
        return src;
    }
    if a == 0 {
        return dst;
    }
    let da = dst >> 24;
    if da == 0 {
        return src;
    }
    let ia = 255 - a;
    let ch = |s: u32| -> u32 { (((src >> s) & 0xff) * a + ((dst >> s) & 0xff) * ia) / 255 };
    (a.max(da)) << 24 | ch(16) << 16 | ch(8) << 8 | ch(0)
}

/// A mutable image with its `Graphics` state.
pub struct Surface {
    pub img: Image,
    color: u32,
    clip: (i32, i32, i32, i32), // x0, y0, x1, y1 (exclusive)
    pub font: Rc<Font>,
}

impl Surface {
    /// `Image.createImage(w, h)`: opaque white.
    pub fn new(w: i32, h: i32, font: Rc<Font>) -> Surface {
        Self::with_fill(w, h, 0xffff_ffff, font)
    }

    /// `DirectUtils.createImage(w, h, argb)`
    pub fn with_fill(w: i32, h: i32, argb: u32, font: Rc<Font>) -> Surface {
        Surface { img: Image::filled(w, h, argb), color: 0xff00_0000, clip: (0, 0, w, h), font }
    }

    /// Freeze into an immutable image (the game only reads it from then on).
    pub fn into_img(self) -> Img {
        Rc::new(self.img)
    }

    /// `setColor(int rgb)`
    pub fn set_color(&mut self, rgb: i32) {
        self.color = 0xff00_0000 | (rgb as u32 & 0x00ff_ffff);
    }

    pub fn set_clip(&mut self, x: i32, y: i32, w: i32, h: i32) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = x.saturating_add(w).min(self.img.w);
        let y1 = y.saturating_add(h).min(self.img.h);
        self.clip = (x0, y0, x1.max(x0), y1.max(y0));
    }

    pub fn fill_rect(&mut self, x: i32, y: i32, w: i32, h: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let (cx0, cy0, cx1, cy1) = self.clip;
        let xa = x.max(cx0);
        let ya = y.max(cy0);
        let xb = x.saturating_add(w).min(cx1);
        let yb = y.saturating_add(h).min(cy1);
        let c = self.color;
        let iw = self.img.w;
        for yy in ya..yb {
            let row = (yy * iw) as usize;
            for xx in xa..xb {
                self.img.px[row + xx as usize] = c;
            }
        }
    }

    #[inline]
    fn put(&mut self, x: i32, y: i32, c: u32) {
        let (x0, y0, x1, y1) = self.clip;
        if x >= x0 && x < x1 && y >= y0 && y < y1 {
            let i = (y * self.img.w + x) as usize;
            self.img.px[i] = blend(self.img.px[i], c);
        }
    }

    pub fn draw_string(&mut self, s: &str, x: i32, y: i32, anchor: i32) {
        let font = self.font.clone();
        let w = font.width(s);
        let mut x = x;
        let mut y = y;
        if anchor & HCENTER != 0 {
            x -= w / 2;
        } else if anchor & RIGHT != 0 {
            x -= w;
        }
        if anchor & BOTTOM != 0 {
            y -= font.height;
        } else if anchor & BASELINE != 0 {
            y -= font.ascent;
        }
        let c = self.color;
        font.draw(s, x, y, |px, py| self.put(px, py, c));
    }

    /// `Graphics.drawImage(img, x, y, anchor)`
    pub fn draw_image(&mut self, img: &Image, x: i32, y: i32, anchor: i32) {
        self.draw_image_m(img, x, y, anchor, 0);
    }

    /// `DirectGraphics.drawImage(img, x, y, anchor, manipulation)`: flips
    /// (FLIP_HORIZONTAL / FLIP_VERTICAL bits) and counter-clockwise rotation in
    /// degrees (90 / 180 / 270), applied around the image; the anchor places
    /// the manipulated image.
    pub fn draw_image_m(&mut self, img: &Image, x: i32, y: i32, anchor: i32, manip: i32) {
        let rot = (manip & 0x1ff) % 360;
        let swap = rot == 90 || rot == 270;
        let (dw, dh) = if swap { (img.h, img.w) } else { (img.w, img.h) };
        let mut x = x;
        let mut y = y;
        if anchor & HCENTER != 0 {
            x -= dw / 2;
        } else if anchor & RIGHT != 0 {
            x -= dw;
        }
        if anchor & VCENTER != 0 {
            y -= dh / 2;
        } else if anchor & BOTTOM != 0 {
            y -= dh;
        }
        let fh = manip & FLIP_HORIZONTAL != 0;
        let fv = manip & FLIP_VERTICAL != 0;
        let (cx0, cy0, cx1, cy1) = self.clip;
        let xa = x.max(cx0);
        let ya = y.max(cy0);
        let xb = (x + dw).min(cx1);
        let yb = (y + dh).min(cy1);
        let (sw, sh) = (img.w, img.h);
        for py in ya..yb {
            let ty = py - y;
            for px in xa..xb {
                let tx = px - x;
                // undo the rotation (counter-clockwise on the destination)...
                let (mut ux, mut uy) = match rot {
                    90 => (sw - 1 - ty, tx),
                    180 => (sw - 1 - tx, sh - 1 - ty),
                    270 => (ty, sh - 1 - tx),
                    _ => (tx, ty),
                };
                // ...then the flips, which Nokia applies before rotating
                if fh {
                    ux = sw - 1 - ux;
                }
                if fv {
                    uy = sh - 1 - uy;
                }
                let s = img.px[(uy * sw + ux) as usize];
                if s >> 24 == 0 {
                    continue;
                }
                let i = (py * self.img.w + px) as usize;
                self.img.px[i] = blend(self.img.px[i], s);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn font() -> Rc<Font> {
        Rc::new(Font::new())
    }

    /// 2x1 source: red at (0,0), green at (1,0)
    fn src() -> Image {
        Image { w: 2, h: 1, px: vec![0xffff_0000, 0xff00_ff00] }
    }

    #[test]
    fn rotate_90_is_counter_clockwise() {
        let mut s = Surface::with_fill(1, 2, 0, font());
        s.draw_image_m(&src(), 0, 0, TOP | LEFT, 90);
        // the right end of the source ends up on top
        assert_eq!(s.img.px, vec![0xff00_ff00, 0xffff_0000]);
    }

    #[test]
    fn rotate_270_and_flips() {
        let mut s = Surface::with_fill(1, 2, 0, font());
        s.draw_image_m(&src(), 0, 0, TOP | LEFT, 270);
        assert_eq!(s.img.px, vec![0xffff_0000, 0xff00_ff00]);
        let mut s = Surface::with_fill(2, 1, 0, font());
        s.draw_image_m(&src(), 0, 0, TOP | LEFT, FLIP_HORIZONTAL);
        assert_eq!(s.img.px, vec![0xff00_ff00, 0xffff_0000]);
        let mut s = Surface::with_fill(2, 1, 0, font());
        s.draw_image_m(&src(), 0, 0, TOP | LEFT, 180);
        assert_eq!(s.img.px, vec![0xff00_ff00, 0xffff_0000]);
    }
}
