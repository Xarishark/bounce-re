//! Stand-ins for the phone APIs the game was written against (MIDP 1.0,
//! RMS and the Nokia UI extensions: FullCanvas, DirectGraphics, Sound),
//! implemented on software framebuffers. The SDL3 front end in `main.rs`
//! drives them.

pub mod assets;
pub mod font;
pub mod gfx;
pub mod lang;
pub mod lcdui;
pub mod rms;
pub mod sound;
