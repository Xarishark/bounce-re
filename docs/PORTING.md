# How the port works

Bounce shipped in 2002 as a J2ME MIDlet for Nokia phones (MIDP 1.0,
CLDC 1.0, 128x128 screen, Nokia UI API). Its class names survived
(`com.nokia.mid.appl.boun.*`). This port was made by decompiling the JAR
with [Vineflower](https://github.com/Vineflower/vineflower)
(`tools/decompile.sh`) and rewriting every class in Rust, on top of small
stand-ins for the phone APIs the game used. Nothing Java runs at any point.

## Layout

```
src/
├─ main.rs           SDL3 front end: start-up, first-start setup screen, window,
│                    scaling, keyboard/gamepad, rumble, audio, headless test driver
├─ paths.rs          the portable game folder: save/, data/, logs/, settings.toml
├─ setup.rs          checks the player's JAR and unpacks it into data/
├─ settings.rs       settings.toml (written with comments on first start)
├─ pads.rs           every connected gamepad, hot-plugging
├─ log.rs            logs/bounce.log, last three runs
├─ midp/             phone API stand-ins
│  ├─ gfx.rs         Image, Graphics (as Surface), DirectUtils / DirectGraphics
│  │                 flips and counter-clockwise rotations
│  ├─ font.rs        proportional pixel font (stands in for the phone's small font)
│  ├─ lcdui.rs       List / Form screens, commands, soft keys, key codes
│  ├─ rms.rs         RecordStore (save/<name>.rms), DataInput/OutputStream
│  ├─ sound.rs       com.nokia.mid.sound.Sound: OTT ringtone parser + square wave
│  ├─ lang.rs        Local: the string tables (lang.xx etc.)
│  └─ assets.rs      getResourceAsStream: data/ read into memory at start
└─ game/             the game, one file per Java class
   ├─ mod.rs         Game: every class's fields in one struct, Display, GameTimer
   ├─ ui.rs          BounceUI + Bounce (MIDlet): menus, commands, RMS save format
   ├─ tile_canvas.rs TileCanvas: level loading, tile images, ring-buffer scrolling
   ├─ bounce_canvas.rs BounceCanvas: run() loop, HUD, splash, keys, cheats
   └─ ball.rs        Ball: physics, collision, pickups
```

## Conventions

- **One struct, Java names.** The objects reach into each other (the ball
  edits the canvas' tile map, the canvas calls the UI), so all fields live in
  `game::Game` and each class is an `impl Game` block. Fields and methods keep
  their Java names (`tileX`, `mBall.ySpeed`, `collisionDetection`), and local
  variables keep Vineflower's `varN`, so any line can be checked against the
  decompiled source. Name clashes get a prefix (`Ball.update` is
  `ball_update`, `resetGame(int, int)` is `resetGameSaved`).
- **Integer semantics.** Java `int` / `short` arithmetic maps to `i32`
  (division truncates the same way, `>>` is arithmetic); `(short)` casts are
  kept as `as i16 as i32` where values could overflow.
- **Exceptions.** The original relies on a few: a missing level or a write
  past the 40 saved tiles abandons the operation. Tile map reads outside the
  map return empty space instead of throwing (the Timer thread would have
  died), and `enlargeBall` gives up after 64 pixels instead of spinning.
- **Timing.** `TileCanvas.GameTimer` fires `run()` every 30 ms; `Game::update`
  runs as many ticks as real time allows (at most 8 per frame) and the frame
  shows the latest `mFullScreenBuffer`. The display refreshes at the monitor
  rate.
- **Display.** `Display.setCurrent` is `show_canvas` / `show_screen`; the
  canvas gets `hideNotify` when a screen replaces it, as on the phone (the
  game uses `mIncomingCall` to tell its own menu switches from interruptions).

## Port-only changes

Marked `port-only` in the code; see the README's list. Everything else,
including quirks (level 11 is only reachable with the level cheat or by
choosing "Continue" after finishing level 10; the extra-life pickup clears a
water flag under it), is as in the original.

## Tests

`tools/run-tests.sh` runs each script in `tests/scripts/` against a fresh
temporary game folder (`BOUNCE_HOME`) with data prepared from your JAR:

```
wait <ms>       press <key>   release <key>   tap <key>
shot <file.png> stat <label>  restart         quit
```

Keys are `up down left right fire softl softr` or a digit `0`-`9` (phone
key). `restart` quits the way the phone would (the game is saved) and starts
again. `stat` prints level, lives, rings, score, ball position and size.
`cargo test` checks the image transforms, decodes the three sounds from the
JAR (if it's in `game-files/`) and renders the setup screen to `target/`.

Cheats from the original, typed on the phone keypad during play:
7 8 7 8 9 then 9 unlocks level select (1 previous, 3 next level), or
7 8 7 8 9 then 8 makes the ball invincible.
