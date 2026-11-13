//! The high-level LCDUI screens Bounce uses (an implicit `List` and plain
//! text `Form`s with commands), drawn into the 128x128 screen the way a
//! Series 40 phone lays them out: title, body, soft-key bar.

use crate::midp::gfx::{Surface, HCENTER, LEFT, RIGHT, SCREEN_H, SCREEN_W, TOP};
use crate::midp::lang;

// Nokia key codes
pub const K_UP: i32 = -1;
pub const K_DOWN: i32 = -2;
pub const K_LEFT: i32 = -3;
pub const K_RIGHT: i32 = -4;
pub const K_FIRE: i32 = -5;
pub const K_SOFT_L: i32 = -6;
pub const K_SOFT_R: i32 = -7;

/// `Canvas.getGameAction`
pub fn game_action(key: i32) -> i32 {
    match key {
        K_UP | 50 => 1,   // UP
        K_LEFT | 52 => 2, // LEFT
        K_RIGHT | 54 => 5, // RIGHT
        K_DOWN | 56 => 6, // DOWN
        K_FIRE | 53 => 8, // FIRE
        _ => 0,
    }
}

/// The game's `Command` objects.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cmd {
    /// `List.SELECT_COMMAND`
    Select,
    /// mBackCmd: Back (BACK)
    Back,
    /// mOkayCmd: OK (OK)
    Okay,
    /// mContinueCmd: Continue (OK)
    Continue,
}

impl Cmd {
    pub fn label(self) -> String {
        lang::s(match self {
            Cmd::Select | Cmd::Okay => 19,
            Cmd::Back => 6,
            Cmd::Continue => 8,
        })
    }
    fn is_back(self) -> bool {
        self == Cmd::Back
    }
}

/// Which screen this is, standing in for `var2 == this.mNewGameMenu`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tag {
    MainMenu,
    NewGameMenu,
    TextPage,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    List,
    Form,
}

pub struct Screen {
    pub tag: Tag,
    pub kind: Kind,
    pub title: String,
    /// Form string items
    pub items: Vec<String>,
    /// List elements
    pub opts: Vec<String>,
    pub sel: usize,
    pub cmds: Vec<Cmd>,
    scroll: i32,
}

// Bounce's own colours: sky background, blue UI bar, red ball
const BG: i32 = 0xB0E0F0;
const BAR: i32 = 0x0853AA;
const BAR_EDGE: i32 = 0x1060B0;
const TEXT: i32 = 0x08305A;
const SEL: i32 = 0xC8201C;
const WHITE: i32 = 0xFFFFFE;

const TITLE_H: i32 = 13;
const SOFT_H: i32 = 12;
const ROW_H: i32 = 12;
const LINE_H: i32 = 10;
const BODY_Y: i32 = TITLE_H + 2;
const BODY_H: i32 = SCREEN_H - TITLE_H - SOFT_H - 3;

impl Screen {
    /// `new List(title, IMPLICIT)` (+ `append` for each element)
    pub fn list(tag: Tag, title: String, opts: Vec<String>) -> Screen {
        Screen { tag, kind: Kind::List, title, items: Vec::new(), opts, sel: 0, cmds: Vec::new(), scroll: 0 }
    }
    /// `new Form(title)`
    pub fn form(title: String) -> Screen {
        Screen { tag: Tag::TextPage, kind: Kind::Form, title, items: Vec::new(), opts: Vec::new(), sel: 0, cmds: Vec::new(), scroll: 0 }
    }
    pub fn add_command(&mut self, c: Cmd) {
        if !self.cmds.contains(&c) {
            self.cmds.push(c);
        }
    }
    pub fn append(&mut self, s: impl Into<String>) {
        self.items.push(s.into());
    }
    /// `setSelectedIndex(i, true)`
    pub fn set_selected_index(&mut self, i: i32) {
        if i >= 0 && (i as usize) < self.opts.len() {
            self.sel = i as usize;
        }
    }
    pub fn selected_index(&self) -> i32 {
        self.sel as i32
    }
    /// `getString(i)`
    pub fn get_string(&self, i: i32) -> String {
        self.opts.get(i.max(0) as usize).cloned().unwrap_or_default()
    }

    fn left_cmd(&self) -> Option<Cmd> {
        if self.kind == Kind::List {
            return Some(Cmd::Select);
        }
        self.cmds.iter().copied().find(|c| !c.is_back())
    }
    fn right_cmd(&self) -> Option<Cmd> {
        self.cmds.iter().copied().find(|c| c.is_back())
    }

    /// Handle a key press; returns a command for `commandAction`.
    pub fn key(&mut self, key: i32, font: &crate::midp::font::Font) -> Option<Cmd> {
        match self.kind {
            Kind::List => {
                let n = self.opts.len();
                match key {
                    K_UP | 50 if n > 0 => self.sel = (self.sel + n - 1) % n,
                    K_DOWN | 56 if n > 0 => self.sel = (self.sel + 1) % n,
                    K_FIRE | K_SOFT_L | 53 => return self.left_cmd(),
                    K_SOFT_R => return self.right_cmd(),
                    _ => {}
                }
                None
            }
            Kind::Form => {
                match key {
                    K_UP | K_DOWN | 50 | 56 => {
                        let total = self.lines(font).len() as i32 * LINE_H;
                        let max = (total - BODY_H).max(0);
                        let d = if matches!(key, K_UP | 50) { -LINE_H * 3 } else { LINE_H * 3 };
                        self.scroll = (self.scroll + d).clamp(0, max);
                    }
                    K_FIRE | K_SOFT_L | 53 => {
                        // a Form without an OK-type command: Fire does what the right key does
                        return self.left_cmd().or(self.right_cmd());
                    }
                    K_SOFT_R => return self.right_cmd().or(self.left_cmd()),
                    _ => {}
                }
                None
            }
        }
    }

    /// Form items laid out as lines (each StringItem starts a new paragraph
    /// only where its text has a line break, like the phone does).
    fn lines(&self, font: &crate::midp::font::Font) -> Vec<String> {
        let mut out = Vec::new();
        let w = SCREEN_W - 8;
        for it in &self.items {
            // the blank lines the game appends ("\n\n") become one gap
            if it.trim().is_empty() {
                out.push(String::new());
                continue;
            }
            out.extend(font.wrap(it.trim_end_matches('\n'), w));
        }
        out
    }

    pub fn draw(&mut self, g: &mut Surface) {
        g.set_clip(0, 0, SCREEN_W, SCREEN_H);
        let font = g.font.clone();
        g.set_color(BG);
        g.fill_rect(0, 0, SCREEN_W, SCREEN_H);
        // title bar
        g.set_color(BAR);
        g.fill_rect(0, 0, SCREEN_W, TITLE_H);
        g.set_color(BAR_EDGE);
        g.fill_rect(0, TITLE_H - 1, SCREEN_W, 1);
        g.set_color(WHITE);
        g.draw_string(&self.title, SCREEN_W / 2, 2, TOP | HCENTER);

        g.set_clip(0, BODY_Y, SCREEN_W, BODY_H);
        match self.kind {
            Kind::List => {
                let n = self.opts.len() as i32;
                let visible = BODY_H / ROW_H;
                let sel = self.sel as i32;
                if sel * ROW_H < self.scroll {
                    self.scroll = sel * ROW_H;
                }
                if (sel + 1) * ROW_H > self.scroll + visible * ROW_H {
                    self.scroll = (sel + 1) * ROW_H - visible * ROW_H;
                }
                for i in 0..n {
                    let y = BODY_Y + i * ROW_H - self.scroll;
                    if i == sel {
                        g.set_color(SEL);
                        g.fill_rect(2, y, SCREEN_W - 4, ROW_H);
                        g.set_color(WHITE);
                    } else {
                        g.set_color(TEXT);
                    }
                    g.draw_string(&self.opts[i as usize], 6, y + 2, TOP | LEFT);
                }
                if n > visible {
                    let track = BODY_H - 2;
                    let th = (track * visible / n).max(4);
                    let ty = BODY_Y + 1 + (track - th) * sel / (n - 1).max(1);
                    g.set_color(BAR);
                    g.fill_rect(SCREEN_W - 2, ty, 2, th);
                }
            }
            Kind::Form => {
                let lines = self.lines(&font);
                g.set_color(TEXT);
                for (i, text) in lines.iter().enumerate() {
                    let y = BODY_Y + 1 + i as i32 * LINE_H - self.scroll;
                    g.draw_string(text, 4, y, TOP | LEFT);
                }
                let total = lines.len() as i32 * LINE_H;
                if total > BODY_H {
                    let track = BODY_H - 2;
                    let th = (track * BODY_H / total).max(4);
                    let ty = BODY_Y + 1 + (track - th) * self.scroll / (total - BODY_H).max(1);
                    g.set_color(BAR);
                    g.fill_rect(SCREEN_W - 2, ty, 2, th);
                }
            }
        }
        g.set_clip(0, 0, SCREEN_W, SCREEN_H);

        // soft-key bar
        let y = SCREEN_H - SOFT_H;
        g.set_color(BAR);
        g.fill_rect(0, y, SCREEN_W, SOFT_H);
        g.set_color(BAR_EDGE);
        g.fill_rect(0, y, SCREEN_W, 1);
        g.set_color(WHITE);
        if let Some(c) = self.left_cmd() {
            g.draw_string(&c.label(), 3, y + 2, TOP | LEFT);
        }
        if let Some(c) = self.right_cmd() {
            g.draw_string(&c.label(), SCREEN_W - 3, y + 2, TOP | RIGHT);
        }
    }
}
