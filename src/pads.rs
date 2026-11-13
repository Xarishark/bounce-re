//! Every connected controller, all heard at once (it's a one-player game,
//! so it doesn't matter which pad a button comes from: a Steam Deck's own
//! controls, a paired pad, Steam Input's virtual pad all just work).
//!
//! Hot-plugging: [`Pads::sync`] opens whatever is connected right now
//! (pads present at start, or connected while another screen was reading
//! events), and [`Pads::handle`] opens and closes pads as they come and go.

use crate::midp::lcdui::{K_DOWN, K_FIRE, K_LEFT, K_RIGHT, K_SOFT_L, K_SOFT_R, K_UP};
use sdl3::event::Event;
use sdl3::gamepad::{Axis, Button, Gamepad};
use sdl3::GamepadSubsystem;
use std::collections::HashMap;

/// Input source ids: one per button / stick axis per pad, so the same key
/// held on two pads is counted twice and released only when both let go.
pub const SRC_PAD: u64 = 2 << 40;
pub const SRC_AXIS: u64 = 3 << 40;

/// What a controller event means for the game.
#[derive(Debug, PartialEq, Eq)]
pub enum PadAction {
    Press(u64, i32),
    Release(u64),
    /// a pad went away: let go of everything it was holding
    ReleaseAllFrom(u32),
}

/// Does input source `src` belong to pad `id`?
pub fn src_is_from(src: u64, id: u32) -> bool {
    src >> 40 >= 2 && (src & 0xff_ffff_ffff) >> 8 == id as u64
}

pub fn map_button(b: Button) -> Option<i32> {
    Some(match b {
        Button::DPadUp => K_UP,
        Button::DPadDown => K_DOWN,
        Button::DPadLeft => K_LEFT,
        Button::DPadRight => K_RIGHT,
        Button::South | Button::West => K_FIRE,
        Button::East | Button::Start => K_SOFT_R,
        Button::North | Button::Back => K_SOFT_L,
        Button::LeftShoulder => K_LEFT,
        Button::RightShoulder => K_RIGHT,
        _ => return None,
    })
}

#[derive(Default)]
pub struct Pads {
    open: HashMap<u32, Gamepad>,
    /// stick/trigger direction per (pad, axis): -1, 0 or 1
    axis: HashMap<(u32, u8), i32>,
}

impl Pads {
    pub fn new() -> Pads {
        Pads::default()
    }

    #[allow(dead_code)] // used by the tests
    pub fn count(&self) -> usize {
        self.open.len()
    }

    fn open_pad(&mut self, sub: &GamepadSubsystem, id: sdl3::joystick::JoystickId) {
        let key: u32 = id.into();
        if self.open.contains_key(&key) {
            return;
        }
        match sub.open(id) {
            Ok(p) => {
                crate::log!("controller connected: {}", p.name().unwrap_or_default());
                self.open.insert(key, p);
            }
            Err(e) => crate::log!("couldn't open controller {key}: {e}"),
        }
    }

    /// Open every connected controller and forget ones that are gone.
    /// Returns the pads that went away (their held keys should be released).
    pub fn sync(&mut self, sub: &GamepadSubsystem) -> Vec<PadAction> {
        let now = sub.gamepads().unwrap_or_default();
        for id in &now {
            self.open_pad(sub, *id);
        }
        let ids: Vec<u32> = now.iter().map(|&i| i.into()).collect();
        let gone: Vec<u32> = self.open.keys().copied().filter(|k| !ids.contains(k)).collect();
        gone.into_iter()
            .map(|k| {
                self.open.remove(&k);
                self.axis.retain(|(p, _), _| *p != k);
                PadAction::ReleaseAllFrom(k)
            })
            .collect()
    }

    /// Turn one SDL event into game input. Non-controller events give nothing.
    pub fn handle(&mut self, sub: &GamepadSubsystem, ev: &Event) -> Vec<PadAction> {
        let mut out = Vec::new();
        match ev {
            Event::GamepadAdded { which, .. } => self.open_pad(sub, *which),
            Event::GamepadRemoved { which, .. } => {
                let k: u32 = (*which).into();
                if let Some(p) = self.open.remove(&k) {
                    crate::log!("controller disconnected: {}", p.name().unwrap_or_default());
                }
                self.axis.retain(|(p, _), _| *p != k);
                out.push(PadAction::ReleaseAllFrom(k));
            }
            Event::GamepadButtonDown { which, button, .. } => {
                if let Some(code) = map_button(*button) {
                    let k: u32 = (*which).into();
                    out.push(PadAction::Press(SRC_PAD | (k as u64) << 8 | *button as u64, code));
                }
            }
            Event::GamepadButtonUp { which, button, .. } => {
                if map_button(*button).is_some() {
                    let k: u32 = (*which).into();
                    out.push(PadAction::Release(SRC_PAD | (k as u64) << 8 | *button as u64));
                }
            }
            Event::GamepadAxisMotion { which, axis, value, .. } => {
                let k: u32 = (*which).into();
                let (id, neg, pos): (u8, i32, i32) = match axis {
                    Axis::LeftX => (0, K_LEFT, K_RIGHT),
                    Axis::LeftY => (1, K_UP, K_DOWN),
                    Axis::TriggerRight => (2, 0, K_FIRE),
                    Axis::TriggerLeft => (3, 0, K_SOFT_L),
                    _ => return out,
                };
                // hysteresis: engage at 50%, release at 30%
                let v = *value as i32;
                let prev = *self.axis.get(&(k, id)).unwrap_or(&0);
                let now = if v > 16000 || (prev > 0 && v > 9800) {
                    1
                } else if v < -16000 || (prev < 0 && v < -9800) {
                    -1
                } else {
                    0
                };
                if now != prev {
                    let src = SRC_AXIS | (k as u64) << 8 | id as u64;
                    out.push(PadAction::Release(src));
                    let code = match now {
                        1 => pos,
                        -1 => neg,
                        _ => 0,
                    };
                    if code != 0 {
                        out.push(PadAction::Press(src, code));
                    }
                    self.axis.insert((k, id), now);
                }
            }
            _ => {}
        }
        out
    }

    /// Rumble every connected pad.
    pub fn rumble(&mut self, lo: u16, hi: u16, ms: u32) {
        for p in self.open.values_mut() {
            let _ = p.set_rumble(lo, hi, ms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sdl3::sys::joystick as sj;

    fn attach_virtual_pad(name: &std::ffi::CStr) -> sj::SDL_JoystickID {
        #[allow(deprecated)]
        let desc = sj::SDL_VirtualJoystickDesc {
            version: std::mem::size_of::<sj::SDL_VirtualJoystickDesc>() as u32,
            r#type: sj::SDL_JOYSTICK_TYPE_GAMEPAD.0 as u16,
            naxes: 6,
            nbuttons: 15,
            name: name.as_ptr(),
            ..Default::default()
        };
        let id = unsafe { sj::SDL_AttachVirtualJoystick(&desc) };
        assert!(id.0 != 0, "couldn't attach a virtual pad");
        id
    }

    fn pump(events: &mut sdl3::EventPump, sub: &GamepadSubsystem, pads: &mut Pads) -> Vec<PadAction> {
        unsafe { sj::SDL_UpdateJoysticks() };
        let mut out = Vec::new();
        for ev in events.poll_iter() {
            out.extend(pads.handle(sub, &ev));
        }
        out
    }

    /// Hot-plug, two pads at once, sticks and unplugging, with SDL's
    /// virtual controllers (no hardware needed).
    #[test]
    fn hotplug_and_all_pads_at_once() {
        let sdl = sdl3::init().unwrap();
        let sub = sdl.gamepad().unwrap();
        let mut events = sdl.event_pump().unwrap();
        let mut pads = Pads::new();

        // a pad that was already connected before anyone listened for events
        let a = attach_virtual_pad(c"Pad A");
        for _ in events.poll_iter() {}
        assert_eq!(pads.count(), 0);
        assert!(pads.sync(&sub).is_empty());
        assert_eq!(pads.count(), 1, "sync should open pads connected earlier");

        // a second pad plugged in while running
        let b = attach_virtual_pad(c"Pad B");
        pump(&mut events, &sub, &mut pads);
        assert_eq!(pads.count(), 2, "hot-plugged pad should be opened");

        let ja = unsafe { sj::SDL_OpenJoystick(a) };
        let jb = unsafe { sj::SDL_OpenJoystick(b) };
        let south = sdl3::sys::gamepad::SDL_GAMEPAD_BUTTON_SOUTH.0 as i32;
        let leftx = sdl3::sys::gamepad::SDL_GAMEPAD_AXIS_LEFTX.0 as i32;

        // both pads are heard, as separate sources
        unsafe { sj::SDL_SetJoystickVirtualButton(ja, south, true) };
        let got_a = pump(&mut events, &sub, &mut pads);
        unsafe { sj::SDL_SetJoystickVirtualButton(jb, south, true) };
        let got_b = pump(&mut events, &sub, &mut pads);
        let press = |v: &[PadAction]| v.iter().find_map(|x| if let PadAction::Press(s, c) = x { Some((*s, *c)) } else { None });
        let (sa, ca) = press(&got_a).expect("pad A fire");
        let (sb, cb) = press(&got_b).expect("pad B fire");
        assert_eq!((ca, cb), (K_FIRE, K_FIRE));
        assert_ne!(sa, sb);
        assert!(src_is_from(sa, a.0) && src_is_from(sb, b.0));

        // left stick on pad B moves right
        unsafe { sj::SDL_SetJoystickVirtualAxis(jb, leftx, 30000) };
        let got = pump(&mut events, &sub, &mut pads);
        assert!(got.iter().any(|x| matches!(x, PadAction::Press(_, c) if *c == K_RIGHT)), "{got:?}");

        // unplugging pad A releases what it held and closes it
        unsafe {
            sj::SDL_CloseJoystick(ja);
            sj::SDL_DetachVirtualJoystick(a);
        }
        let got = pump(&mut events, &sub, &mut pads);
        assert!(got.contains(&PadAction::ReleaseAllFrom(a.0)), "{got:?}");
        assert_eq!(pads.count(), 1);

        // and plugging it back in works again
        let a2 = attach_virtual_pad(c"Pad A again");
        pump(&mut events, &sub, &mut pads);
        assert_eq!(pads.count(), 2);
        unsafe {
            sj::SDL_CloseJoystick(jb);
            sj::SDL_DetachVirtualJoystick(b);
            sj::SDL_DetachVirtualJoystick(a2);
        }
    }
}
