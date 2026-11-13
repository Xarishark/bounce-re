//! Bounce RE: native PC port of Bounce, the 2002 Nokia J2ME game.
//! SDL3 provides the window, keyboard and gamepad input, rumble and audio.
//!
//! Start-up: find the game folder (`paths`), open the log, read
//! `settings.toml`, make sure the original game's files are prepared from
//! the player's JAR (`setup`), load them, then run the game.

#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod game;
mod log;
mod midp;
mod pads;
mod paths;
mod settings;
mod setup;

use midp::{assets, font, gfx, lang, sound};

use game::Game;
use gfx::{Surface, SCREEN_H, SCREEN_W};
use midp::lcdui::{K_DOWN, K_FIRE, K_LEFT, K_RIGHT, K_SOFT_L, K_SOFT_R, K_UP};
use pads::{PadAction, Pads};
use settings::Settings;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const HELP: &str = "\
Bounce RE

Usage: bounce [options]

  --game <jar>        your Bounce.jar (prepared on first use)
  --prepare           only check the JAR given with --game and unpack it, then exit
  --scale <n>         window size at start, n x 128 pixels
  --fullscreen        start full screen
  --mute              no sound
  --no-rumble         no gamepad rumble
  --lang <xx|tl-PH>   English (xx) or Filipino text
  --headless --script <file> --out <dir>   run a test script without a window

Keyboard: arrows/WASD move, Up/W/Space jump, Enter/Space select,
  Esc/Tab menu, 0-9 phone keypad, M mute, +/- volume, F / F11 full screen.
Gamepad: D-pad/left stick move, A/X jump/select, B/Start/Y/Back menu.

Settings: settings.toml next to the game. Saves: save/. Logs: logs/.";

struct Opts {
    scale: Option<u32>,
    fullscreen: bool,
    script: Option<PathBuf>,
    out: PathBuf,
    headless: bool,
    mute: bool,
    no_rumble: bool,
    lang: Option<String>,
    game: Option<PathBuf>,
    prepare: bool,
}

fn parse_args() -> Opts {
    let mut o = Opts {
        scale: None,
        fullscreen: false,
        script: None,
        out: PathBuf::from("."),
        headless: false,
        mute: false,
        no_rumble: false,
        lang: None,
        game: None,
        prepare: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        // accept both "--opt value" and "--opt=value"
        let (a, inline) = match arg.split_once('=') {
            Some((k, v)) if k.starts_with("--") => (k.to_string(), Some(v.to_string())),
            _ => (arg.clone(), None),
        };
        let mut val = || inline.clone().or_else(|| it.next());
        match a.as_str() {
            "--scale" => o.scale = val().and_then(|v| v.parse::<u32>().ok()).map(|v| v.clamp(1, 16)),
            "--fullscreen" => o.fullscreen = true,
            "--script" => o.script = val().map(PathBuf::from),
            "--out" => o.out = val().map(PathBuf::from).unwrap_or_default(),
            "--headless" => o.headless = true,
            "--mute" => o.mute = true,
            "--no-rumble" => o.no_rumble = true,
            "--lang" => o.lang = val(),
            "--game" => o.game = val().map(PathBuf::from),
            "--prepare" => o.prepare = true,
            "--help" | "-h" => {
                println!("{HELP}");
                std::process::exit(0);
            }
            other => eprintln!("ignoring unknown option {other}"),
        }
    }
    o
}

/// Log, tell the player (a dialog when there's a window), and exit with `code`.
fn fatal(gui: bool, code: i32, msg: &str) -> ! {
    log!("error ({code}): {msg}");
    if gui {
        let _ = sdl3::messagebox::show_simple_message_box(sdl3::messagebox::MessageBoxFlag::ERROR, "Bounce", msg, None);
    }
    std::process::exit(code);
}

fn save_png(path: &Path, fb: &[u32], scale: usize) {
    let Ok(f) = std::fs::File::create(path) else { return };
    let (w0, h0) = (SCREEN_W as usize, SCREEN_H as usize);
    let mut enc = png::Encoder::new(std::io::BufWriter::new(f), (w0 * scale) as u32, (h0 * scale) as u32);
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let Ok(mut w) = enc.write_header() else { return };
    let mut data = Vec::with_capacity(fb.len() * 3 * scale * scale);
    for y in 0..h0 * scale {
        for x in 0..w0 * scale {
            let p = fb[(y / scale) * w0 + x / scale];
            data.extend_from_slice(&[(p >> 16) as u8, (p >> 8) as u8, p as u8]);
        }
    }
    let _ = w.write_image_data(&data);
}

// ----------------------------------------------------------------- input
/// Phone keys pressed by several physical sources are ref-counted so
/// releasing one source doesn't release the key while another still holds it.
struct Input {
    held: HashMap<i32, u32>,
    /// physical source id -> key code
    sources: HashMap<u64, i32>,
    repeat_at: HashMap<i32, f64>,
}

const SRC_KEY: u64 = 1 << 40;

impl Input {
    fn new() -> Input {
        Input { held: HashMap::new(), sources: HashMap::new(), repeat_at: HashMap::new() }
    }
    fn press(&mut self, src: u64, code: i32, game: &mut Game) {
        if self.sources.contains_key(&src) {
            return;
        }
        self.sources.insert(src, code);
        let n = self.held.entry(code).or_insert(0);
        *n += 1;
        if *n == 1 {
            game.key_pressed(code);
            self.repeat_at.insert(code, 380.0);
        }
    }
    fn release(&mut self, src: u64, game: &mut Game) {
        let Some(code) = self.sources.remove(&src) else { return };
        if let Some(n) = self.held.get_mut(&code) {
            *n = n.saturating_sub(1);
            if *n == 0 {
                self.held.remove(&code);
                self.repeat_at.remove(&code);
                game.key_released(code);
            }
        }
    }
    fn release_all(&mut self, game: &mut Game) {
        let srcs: Vec<u64> = self.sources.keys().copied().collect();
        for s in srcs {
            self.release(s, game);
        }
    }
    /// Apply what a controller event means (see `pads`).
    fn pad(&mut self, actions: Vec<PadAction>, game: &mut Game) {
        for a in actions {
            match a {
                PadAction::Press(src, code) => self.press(src, code, game),
                PadAction::Release(src) => self.release(src, game),
                PadAction::ReleaseAllFrom(id) => {
                    let srcs: Vec<u64> = self.sources.keys().copied().filter(|&s| pads::src_is_from(s, id)).collect();
                    for s in srcs {
                        self.release(s, game);
                    }
                }
            }
        }
    }
    /// Auto-repeat Up/Down in menus (the canvas only wants presses).
    fn tick(&mut self, dt: f64, game: &mut Game) {
        if !game.screen_is_form() {
            return;
        }
        for code in [K_UP, K_DOWN] {
            if let Some(t) = self.repeat_at.get_mut(&code) {
                *t -= dt;
                if *t <= 0.0 {
                    *t = 110.0;
                    game.key_pressed(code);
                }
            }
        }
    }
}

fn map_key(k: sdl3::keyboard::Keycode) -> Option<i32> {
    use sdl3::keyboard::Keycode as K;
    Some(match k {
        K::Up | K::W => K_UP,
        K::Down | K::S => K_DOWN,
        K::Left | K::A => K_LEFT,
        K::Right | K::D => K_RIGHT,
        K::Return | K::Return2 | K::Space | K::KpEnter => K_FIRE,
        K::Escape => K_SOFT_R,
        K::Tab | K::F1 => K_SOFT_L,
        K::_0 | K::Kp0 => 48,
        K::_1 | K::Kp1 => 49,
        K::_2 | K::Kp2 => 50,
        K::_3 | K::Kp3 => 51,
        K::_4 | K::Kp4 => 52,
        K::_5 | K::Kp5 => 53,
        K::_6 | K::Kp6 => 54,
        K::_7 | K::Kp7 => 55,
        K::_8 | K::Kp8 => 56,
        K::_9 | K::Kp9 => 57,
        _ => return None,
    })
}

// ----------------------------------------------------------------- script
/// Headless test driver: `wait ms`, `press key`, `release key`, `tap key`,
/// `shot file.png`, `stat label`, `restart` (quit and start again), `quit`. Keys are phone key codes or names
/// (up, down, left, right, fire, softl, softr), or a single digit 0-9.
fn run_script(game: &mut Game, screen: &mut Surface, script: &Path, out: &Path) {
    let mut input = Input::new();
    let text = std::fs::read_to_string(script).unwrap_or_default();
    let _ = std::fs::create_dir_all(out);
    let step = 1000.0 / 60.0;
    let code_of = |s: &str| -> i32 {
        match s {
            "up" => K_UP,
            "down" => K_DOWN,
            "left" => K_LEFT,
            "right" => K_RIGHT,
            "fire" => K_FIRE,
            "softl" => K_SOFT_L,
            "softr" => K_SOFT_R,
            // a single digit is that phone key (key code = its character)
            d if d.len() == 1 && d.as_bytes()[0].is_ascii_digit() => d.as_bytes()[0] as i32,
            _ => s.parse().unwrap_or(0),
        }
    };
    let mut buf = Vec::new();
    let mut t_ms = 0.0f64;
    let advance = |game: &mut Game, screen: &mut Surface, ms: f64, buf: &mut Vec<f32>| {
        let mut t = 0.0;
        while t < ms && !game.quit {
            game.update(step, screen);
            game.tones.render((sound::RATE as f64 * step / 1000.0) as usize, buf);
            t += step;
        }
    };
    for line in text.lines().flat_map(|l| l.split(';')) {
        let mut p = line.split_whitespace();
        let Some(cmd) = p.next() else { continue };
        let arg = p.next().unwrap_or("");
        match cmd {
            "wait" => {
                let ms: f64 = arg.parse().unwrap_or(0.0);
                t_ms += ms;
                advance(game, screen, ms, &mut buf)
            }
            "press" => input.press(SRC_KEY | (code_of(arg) as u32 as u64), code_of(arg), game),
            "release" => input.release(SRC_KEY | (code_of(arg) as u32 as u64), game),
            "tap" => {
                let c = code_of(arg);
                game.key_pressed(c);
                advance(game, screen, 60.0, &mut buf);
                game.key_released(c);
                advance(game, screen, 60.0, &mut buf);
            }
            "shot" => {
                game.update(0.0, screen);
                save_png(&out.join(arg), &screen.img.px, 3);
                println!("shot {arg}");
            }
            "stat" => println!(
                "stat {} t={} level={} lives={} rings={}/{} score={} ball=({},{}) size={} tile=({},{}) best={}/{}",
                arg,
                t_ms,
                game.mLevelNum,
                game.numLives,
                game.numRings,
                game.mTotalNumRings,
                game.mScore,
                game.mBall.xPos,
                game.mBall.yPos,
                game.mBall.mBallSize,
                game.tileX,
                game.tileY,
                game.mBestLevel,
                game.mBestScore
            ),
            // quit the way the phone would (saves the game), then start again
            "restart" => {
                game.close();
                let (font, mut tones) = (game.font.clone(), sound::Tones::new());
                tones.muted = game.tones.muted;
                *game = Game::new(font, tones);
                println!("restart");
                continue;
            }
            "quit" => break,
            _ => {}
        }
        if game.quit {
            break;
        }
    }
}

// ----------------------------------------------------------------- main
fn main() {
    let o = parse_args();
    let gui = !o.headless && o.script.is_none() && !o.prepare;
    let root = match paths::init() {
        Ok(r) => r,
        Err(m) => fatal(gui, setup::exit::NOT_WRITABLE, &m),
    };
    log::init();
    log!("Bounce RE {} in {}", env!("CARGO_PKG_VERSION"), root.display());
    if let Ok(v) = std::env::var("RECOMPLAUNCHER_VERSION") {
        log!(
            "started by RecompLauncher {v} (mode {}, pad {})",
            std::env::var("RECOMPLAUNCHER_MODE").unwrap_or_default(),
            std::env::var("RECOMPLAUNCHER_PAD").unwrap_or_default()
        );
    }
    let data = paths::data_dir();

    // RecompLauncher's setup step: no window, progress on stdout, exit code.
    if o.prepare {
        let Some(jar) = &o.game else {
            log!("--prepare needs --game <jar>");
            std::process::exit(setup::exit::UNREADABLE);
        };
        match setup::prepare(jar, &data, |f, t| println!("progress {f:.2} {t}")) {
            Ok(n) => {
                log!("prepared {n} files from {}", jar.display());
                std::process::exit(0);
            }
            Err(e) => fatal(false, e.code, &e.message),
        }
    }

    // A JAR given on the command line, or one sitting next to the game.
    if !setup::prepared(&data) {
        let jar = o.game.clone().or_else(|| setup::find_nearby(&root));
        if let Some(jar) = jar {
            log!("preparing game data from {}", jar.display());
            if let Err(e) = setup::prepare(&jar, &data, |_, _| {}) {
                fatal(gui, e.code, &e.message);
            }
        }
    }

    let settings = settings::load();
    let font = Rc::new(font::Font::new());
    let mut screen = Surface::new(SCREEN_W, SCREEN_H, font.clone());

    if !gui {
        if !setup::prepared(&data) {
            fatal(
                false,
                setup::exit::NOT_SET_UP,
                &format!("The game data isn't set up. Run once with --game <path to {}>.", setup::JAR_NAME),
            );
        }
        let mut game = start_game(&o, &settings, &data, &font, false);
        if let Some(script) = &o.script {
            run_script(&mut game, &mut screen, script, &o.out);
        }
        game.close();
        return;
    }
    if let Err(e) = run(&o, &settings, &data, &mut screen, &font) {
        fatal(true, 1, &e);
    }
}

/// Load the prepared data and build the game.
fn start_game(o: &Opts, s: &Settings, data: &Path, font: &Rc<font::Font>, gui: bool) -> Game {
    match assets::load(data) {
        Ok(n) => log!("loaded {n} game files"),
        Err(e) => fatal(gui, setup::exit::NOT_SET_UP, &e),
    }
    lang::init(o.lang.as_deref().unwrap_or(&s.language));
    let mut tones = sound::Tones::new();
    tones.volume = s.volume as f32 / 100.0;
    tones.muted = o.mute || !s.sound;
    Game::new(font.clone(), tones)
}

/// Copy the 128x128 framebuffer to the window.
fn present(canvas: &mut sdl3::render::WindowCanvas, tex: &mut sdl3::render::Texture, g: &Surface) -> Result<(), String> {
    let bytes: Vec<u8> = g.img.px.iter().flat_map(|p| p.to_le_bytes()).collect();
    tex.update(None, &bytes, 128 * 4).map_err(|e| e.to_string())?;
    canvas.set_draw_color(sdl3::pixels::Color::RGB(0, 0, 0));
    canvas.clear();
    canvas.copy(tex, None, None).map_err(|e| e.to_string())?;
    canvas.present();
    Ok(())
}

/// First start without game data: ask for the JAR (drop it on the window or
/// pick it). Returns false if the player quit.
fn setup_screen(
    canvas: &mut sdl3::render::WindowCanvas,
    tex: &mut sdl3::render::Texture,
    events: &mut sdl3::EventPump,
    sub: &sdl3::GamepadSubsystem,
    pads: &mut Pads,
    g: &mut Surface,
    data: &Path,
) -> Result<bool, String> {
    use sdl3::event::Event;
    use sdl3::keyboard::Keycode;
    let mut status: Option<(String, bool)> = None; // (text, is_error)
    let picked: Arc<Mutex<Option<Option<PathBuf>>>> = Arc::new(Mutex::new(None));
    let mut dialog_open = false;
    pads.sync(sub);
    loop {
        let mut try_jar: Option<PathBuf> = None;
        let mut browse = false;
        for ev in events.poll_iter() {
            // any controller: Fire / A (Cross) or the left soft key picks a file, B / Start quits
            for a in pads.handle(sub, &ev) {
                match a {
                    PadAction::Press(_, K_FIRE | K_SOFT_L) => browse = true,
                    PadAction::Press(_, K_SOFT_R) => return Ok(false),
                    _ => {}
                }
            }
            match ev {
                Event::Quit { .. } => return Ok(false),
                Event::DropFile { filename, .. } => try_jar = Some(PathBuf::from(filename)),
                Event::KeyDown { keycode: Some(k), repeat: false, .. } => match k {
                    Keycode::Return | Keycode::Return2 | Keycode::KpEnter | Keycode::Space | Keycode::Tab => browse = true,
                    Keycode::Escape => return Ok(false),
                    _ => {}
                },
                _ => {}
            }
        }
        if browse && !dialog_open {
            dialog_open = true;
            let slot = picked.clone();
            let filters = [sdl3::dialog::DialogFileFilter { name: "Bounce (.jar)", pattern: "jar" }];
            let r = sdl3::dialog::show_open_file_dialog(
                &filters,
                None::<PathBuf>,
                false,
                canvas.window(),
                Box::new(move |res, _| {
                    let path = res.ok().and_then(|v| v.into_iter().next());
                    *slot.lock().unwrap() = Some(path);
                }),
            );
            if let Err(e) = r {
                dialog_open = false;
                status = Some((format!("Couldn't open a file picker ({e:?}). Drop the file on this window instead."), true));
            }
        }
        let result = picked.lock().unwrap().take();
        if let Some(result) = result {
            dialog_open = false;
            try_jar = try_jar.or(result);
        }
        if let Some(jar) = try_jar {
            log!("setup: trying {}", jar.display());
            status = Some(("Checking...".into(), false));
            draw_setup(g, &status);
            present(canvas, tex, g)?;
            match setup::prepare(&jar, data, |_, _| {}) {
                Ok(n) => {
                    log!("prepared {n} files from {}", jar.display());
                    return Ok(true);
                }
                Err(e) => {
                    log!("setup failed ({}): {}", e.code, e.message);
                    status = Some((e.message.replace('\n', " "), true));
                }
            }
        }
        draw_setup(g, &status);
        present(canvas, tex, g)?;
        std::thread::sleep(Duration::from_millis(16));
    }
}

/// The confirm button as the player knows it: RecompLauncher tells us the
/// controller family (RECOMPLAUNCHER_PAD = xbox | playstation).
fn pad_button() -> &'static str {
    match std::env::var("RECOMPLAUNCHER_PAD").as_deref() {
        Ok("playstation") => "Cross",
        _ => "A",
    }
}

fn draw_setup(g: &mut Surface, status: &Option<(String, bool)>) {
    use gfx::{HCENTER, LEFT, RIGHT, TOP};
    const BG: i32 = 0xB0E0F0;
    const BAR: i32 = 0x0853AA;
    const TEXT: i32 = 0x08305A;
    g.set_clip(0, 0, 128, 128);
    g.set_color(BG);
    g.fill_rect(0, 0, 128, 128);
    g.set_color(BAR);
    g.fill_rect(0, 0, 128, 13);
    g.set_color(0xFFFFFE);
    g.draw_string("Bounce", 64, 2, TOP | HCENTER);
    let font = g.font.clone();
    let mut y = 16;
    let para = |g: &mut Surface, text: &str, rgb: i32, y: &mut i32| {
        g.set_color(rgb);
        for line in font.wrap(text, 122) {
            g.draw_string(&line, 3, *y, TOP | LEFT);
            *y += 10;
        }
    };
    para(g, "This needs your copy of the original game:", TEXT, &mut y);
    para(g, setup::JAR_NAME, 0xC8201C, &mut y);
    y += 3;
    match status {
        Some((text, err)) => para(g, text, if *err { 0xC8201C } else { 0x1C7A20 }, &mut y),
        None => para(g, &format!("Drop it on this window, or press Fire / {} to pick it.", pad_button()), TEXT, &mut y),
    }
    g.set_color(BAR);
    g.fill_rect(0, 116, 128, 12);
    g.set_color(0xFFFFFE);
    g.draw_string("Pick file", 3, 118, TOP | LEFT);
    g.draw_string("Quit", 125, 118, TOP | RIGHT);
}

fn set_icon(window: &mut sdl3::video::Window) {
    use sdl3::pixels::PixelFormat;
    let Some(icon) = assets::get("/icons/icon.png").and_then(gfx::Image::from_png) else { return };
    if let Ok(mut s) = sdl3::surface::Surface::new(icon.w as u32, icon.h as u32, PixelFormat::ARGB8888) {
        let pitch = s.pitch() as usize;
        s.with_lock_mut(|buf: &mut [u8]| {
            for y in 0..icon.h as usize {
                for x in 0..icon.w as usize {
                    let p = icon.px[y * icon.w as usize + x].to_le_bytes();
                    buf[y * pitch + x * 4..y * pitch + x * 4 + 4].copy_from_slice(&p);
                }
            }
        });
        window.set_icon(&s);
    }
}

fn run(o: &Opts, s: &Settings, data: &Path, g: &mut Surface, font: &Rc<font::Font>) -> Result<(), String> {
    use sdl3::event::{Event, WindowEvent};
    use sdl3::pixels::PixelFormat;

    // RecompLauncher's overlay also reads the controller: background input
    // stays off unless the player turns it on.
    sdl3::hint::set("SDL_JOYSTICK_ALLOW_BACKGROUND_EVENTS", if s.background_input { "1" } else { "0" });
    let sdl = sdl3::init().map_err(|e| e.to_string())?;
    let video = sdl.video().map_err(|e| e.to_string())?;
    let pad_sub = sdl.gamepad().map_err(|e| e.to_string())?;
    let mut pads = Pads::new();

    let bigpicture = std::env::var("RECOMPLAUNCHER_MODE").is_ok_and(|m| m == "bigpicture");
    let scale = o.scale.unwrap_or(s.scale);
    let mut wb = video.window("Bounce", 128 * scale, 128 * scale);
    wb.resizable().position_centered().high_pixel_density();
    let mut window = wb.build().map_err(|e| e.to_string())?;
    if o.fullscreen || s.fullscreen || bigpicture {
        let _ = window.set_fullscreen(true);
    }
    let mut canvas = window.into_canvas();
    unsafe {
        sdl3::sys::render::SDL_SetRenderVSync(canvas.raw(), 1);
    }
    let presentation = if s.scaling == "fill" {
        sdl3::sys::render::SDL_LOGICAL_PRESENTATION_LETTERBOX
    } else {
        sdl3::sys::render::SDL_LOGICAL_PRESENTATION_INTEGER_SCALE
    };
    canvas.set_logical_size(128, 128, presentation).map_err(|e| e.to_string())?;
    let tc = canvas.texture_creator();
    let mut tex = tc.create_texture_streaming(PixelFormat::ARGB8888, 128, 128).map_err(|e| e.to_string())?;
    tex.set_scale_mode(sdl3::render::ScaleMode::Nearest);
    let mut events = sdl.event_pump().map_err(|e| e.to_string())?;

    if !setup::prepared(data) && !setup_screen(&mut canvas, &mut tex, &mut events, &pad_sub, &mut pads, g, data)? {
        log!("setup cancelled");
        return Ok(());
    }
    let mut game_obj = start_game(o, s, data, font, true);
    let game = &mut game_obj;
    set_icon(canvas.window_mut());
    let rumble = !o.no_rumble && s.rumble;

    // audio (optional)
    let audio_sub = sdl.audio().ok();
    let stream = audio_sub.as_ref().and_then(|a| {
        let spec = sdl3::audio::AudioSpec::new(Some(sound::RATE), Some(2), Some(sdl3::audio::AudioFormat::f32_sys()));
        let s = a.default_playback_device().open_device_stream(Some(&spec)).ok()?;
        let _ = s.resume();
        Some(s)
    });
    if stream.is_none() {
        log!("audio unavailable; continuing silently");
    }
    let mut abuf: Vec<f32> = Vec::new();

    let mut input = Input::new();
    // pads already connected (or connected during setup) are opened here
    let gone = pads.sync(&pad_sub);
    input.pad(gone, game);
    let mut last = Instant::now();

    while !game.quit {
        for ev in events.poll_iter() {
            let actions = pads.handle(&pad_sub, &ev);
            input.pad(actions, game);
            match ev {
                Event::Quit { .. } => game.close(),
                Event::Window { win_event: WindowEvent::FocusLost, .. } => input.release_all(game),
                // like a phone call: the game pauses to the menu
                Event::Window { win_event: WindowEvent::Minimized, .. } => game.system_hide(),
                Event::KeyDown { keycode: Some(k), keymod, repeat, scancode, .. } => {
                    use sdl3::keyboard::{Keycode, Mod};
                    let alt = keymod.intersects(Mod::LALTMOD | Mod::RALTMOD);
                    if k == Keycode::F11 || k == Keycode::F || (alt && k == Keycode::Return) {
                        if !repeat {
                            let w = canvas.window_mut();
                            let fs = !matches!(w.fullscreen_state(), sdl3::video::FullscreenType::Off);
                            let _ = w.set_fullscreen(!fs);
                        }
                        continue;
                    }
                    if k == Keycode::M && !repeat {
                        game.tones.muted = !game.tones.muted;
                        continue;
                    }
                    if matches!(k, Keycode::Equals | Keycode::Plus | Keycode::KpPlus) {
                        game.tones.volume = (game.tones.volume + 0.25).min(2.0);
                        continue;
                    }
                    if matches!(k, Keycode::Minus | Keycode::KpMinus) {
                        game.tones.volume = (game.tones.volume - 0.25).max(0.0);
                        continue;
                    }
                    if let Some(code) = map_key(k) {
                        if !repeat {
                            let src = SRC_KEY | scancode.map(|s| s as u64).unwrap_or(k as u64 & 0xffff_ffff);
                            input.press(src, code, game);
                        }
                    }
                }
                Event::KeyUp { keycode: Some(k), scancode, .. } => {
                    if map_key(k).is_some() {
                        let src = SRC_KEY | scancode.map(|s| s as u64).unwrap_or(k as u64 & 0xffff_ffff);
                        input.release(src, game);
                    }
                }
                _ => {}
            }
        }
        if game.quit {
            break;
        }

        let now = Instant::now();
        let dt = (now - last).as_secs_f64() * 1000.0;
        last = now;
        let dt = dt.min(250.0);
        input.tick(dt, game);
        game.update(dt, g);

        if game.rumble.take().is_some() && rumble {
            pads.rumble(0xc000, 0x8000, 180);
        }

        // keep ~60ms of audio queued
        if let Some(s) = &stream {
            let queued = s.queued_bytes().unwrap_or(0).max(0) as usize / 8;
            let target = (sound::RATE as usize) * 60 / 1000;
            if queued < target {
                game.tones.render(target - queued, &mut abuf);
                let _ = s.put_data_f32(&abuf);
            }
        } else {
            game.tones.render((sound::RATE as f64 * dt / 1000.0) as usize, &mut abuf);
        }

        present(&mut canvas, &mut tex, g)?;
        if dt < 4.0 {
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    log!("quit");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Renders the first-start screen in its three states to
    /// target/setup-screen-*.png for a visual check.
    #[test]
    fn setup_screen_renders() {
        let mut g = Surface::new(128, 128, Rc::new(font::Font::new()));
        let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        let states = [
            ("idle", None),
            ("checking", Some(("Checking...".to_string(), false))),
            (
                "wrong",
                Some((
                    "fake.jar isn't the supported Bounce. It needs the Nokia 128x128 release of Bounce 1.9, Bounce.jar (56375 bytes)."
                        .to_string(),
                    true,
                )),
            ),
        ];
        for (name, status) in states {
            draw_setup(&mut g, &status);
            save_png(&dir.join(format!("setup-screen-{name}.png")), &g.img.px, 3);
        }
    }
}
