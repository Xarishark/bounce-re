//! Port of Bounce's classes (`com.nokia.mid.appl.boun`).
//!
//! The Java objects point at each other freely (the ball reaches into the
//! canvas, the canvas calls the UI), so all their fields live in one
//! [`Game`] and each class is an `impl Game` block in its own file, with
//! the Java names kept so the code can be checked line by line against the
//! decompiled source (see `tools/decompile.sh`):
//!
//! - `ui.rs`            BounceUI (menus, RMS) and Bounce (the MIDlet)
//! - `tile_canvas.rs`   TileCanvas (level, tiles, scrolling ring buffer)
//! - `bounce_canvas.rs` BounceCanvas (game loop, HUD, keys, splash)
//! - `ball.rs`          Ball (physics and collision)

#![allow(non_snake_case)]

pub mod ball;
pub mod bounce_canvas;
pub mod tile_canvas;
pub mod ui;

use crate::midp::font::Font;
use crate::midp::gfx::{Img, Surface, SCREEN_H, SCREEN_W};
use crate::midp::lcdui::{self, Screen, K_DOWN, K_FIRE, K_LEFT, K_RIGHT, K_UP};
use crate::midp::sound::{Sound, Tones};
use ball::Ball;
use std::rc::Rc;

/// `TileCanvas.GameTimer` period.
pub const TICK_MS: f64 = 30.0;

/// What `Display.getCurrent()` shows.
pub enum Current {
    Canvas,
    Screen(Screen),
}

pub struct Game {
    pub font: Rc<Font>,
    pub tones: Tones,
    pub quit: bool,
    pub current: Current,
    /// the canvas' GameTimer: running, and ms until it fires next
    timer: Option<f64>,
    /// port-only: directions held on the keyboard / pad right now (Ball.direction bits)
    pub held: i32,
    /// port-only: the ball popped since the front end last looked (gamepad rumble)
    pub rumble: Option<()>,

    // ---- BounceUI
    pub mState: i32,
    pub mBestLevel: i32,
    pub mBestScore: i32,
    pub mNewBestScore: bool,
    pub mLastScore: i32,
    pub mSavedValid: i32,
    pub mSavedLives: i32,
    pub mSavedRings: i32,
    pub mSavedLevel: i32,
    pub mSavedSize: i32,
    pub mSavedScore: i32,
    pub mSavedTileX: i32,
    pub mSavedTileY: i32,
    pub mSavedGlobalBallX: i32,
    pub mSavedGlobalBallY: i32,
    pub mSavedXSpeed: i32,
    pub mSavedYSpeed: i32,
    pub mSavedXPos: i32,
    pub mSavedYPos: i32,
    pub mSavedRespawnX: i32,
    pub mSavedRespawnY: i32,
    pub mSavedSpeedBonus: i32,
    pub mSavedGravBonus: i32,
    pub mSavedJumpBonus: i32,
    pub mSavedTileCount: i32,
    pub mSavedTiles: Vec<[i32; 3]>,
    mSavedMenuItem: i32,
    mMainMenuItems: [String; 4],

    // ---- TileCanvas
    pub tileX: i32,
    pub tileY: i32,
    pub divTileX: i32,
    pub divTileY: i32,
    pub divisorLine: i32,
    pub rightDrawEdge: i32,
    pub leftDrawEdge: i32,
    pub scrollFlag: bool,
    pub scrollOffset: i32,
    pub mExitPos: i32,
    pub mGameBuffer: Surface,
    tileImages: Vec<Img>,
    tmpTileImage: Surface,
    /// hoopImageList / hoopXPosList / hoopYPosList
    hoopList: Vec<(Img, i32, i32)>,
    pub mLevelNum: i32,
    pub mLevelNumStr: String,
    pub mLevelCompletedStr: String,
    pub mLoadLevelFlag: bool,
    pub mStartCol: i32,
    pub mStartRow: i32,
    pub mStartBallSize: i32,
    pub mExitPosX: i32,
    pub mExitPosY: i32,
    /// `short[][]`, None until a level is loaded / after disposeLevel
    pub tileMap: Option<Vec<Vec<i32>>>,
    pub mTileMapWidth: i32,
    pub mTileMapHeight: i32,
    pub mTotalNumRings: i32,
    pub mNumMoveObj: i32,
    pub mMOTopLeft: Vec<[i32; 2]>,
    pub mMOBotRight: Vec<[i32; 2]>,
    pub mMODirection: Vec<[i32; 2]>,
    pub mMOOffset: Vec<[i32; 2]>,
    pub mSpikeImgPtr: Option<Img>,
    pub mUILife: Img,
    pub mUIRing: Img,
    pub mTopLeftExitTileCol: i32,
    pub mTopLeftExitTileRow: i32,
    pub mExitTileImage: Surface,
    pub mImgPtr: Img,
    pub mImageOffset: i32,
    pub mOpenFlag: bool,
    pub mWidth: i32,
    pub mHeight: i32,

    // ---- BounceCanvas
    pub mSplashIndex: i32,
    pub mSplashImage: Option<Img>,
    mSplashTimer: i32,
    pub mSoundHoop: Sound,
    pub mSoundPickup: Sound,
    pub mSoundPop: Sound,
    /// `mBall != null`
    pub hasBall: bool,
    pub mBall: Ball,
    pub numRings: i32,
    pub numLives: i32,
    pub mScore: i32,
    pub bonusCntrValue: i32,
    pub mLevelDisCntr: i32,
    pub mLeaveGame: bool,
    pub mOpenExitFlag: bool,
    pub mPaintUIFlag: bool,
    pub mFullScreenBuffer: Surface,
    pub mClearScreenFlag: bool,
    mCheat: bool,
    pub mInvincible: bool,
    mCheatSeq: i32,
    pub mIncomingCall: bool,
}

impl Game {
    /// `Bounce.startApp()`: `new BounceUI(this)`
    pub fn new(font: Rc<Font>, tones: Tones) -> Game {
        let f = || font.clone();
        let blank: Img = Rc::new(crate::midp::gfx::Image::filled(1, 1, 0));
        let mut g = Game {
            font: font.clone(),
            tones,
            quit: false,
            current: Current::Canvas,
            timer: None,
            held: 0,
            rumble: None,
            mState: 2,
            mBestLevel: 0,
            mBestScore: 0,
            mNewBestScore: false,
            mLastScore: 0,
            mSavedValid: 0,
            mSavedLives: 0,
            mSavedRings: 0,
            mSavedLevel: 0,
            mSavedSize: 0,
            mSavedScore: 0,
            mSavedTileX: 0,
            mSavedTileY: 0,
            mSavedGlobalBallX: 0,
            mSavedGlobalBallY: 0,
            mSavedXSpeed: 0,
            mSavedYSpeed: 0,
            mSavedXPos: 0,
            mSavedYPos: 0,
            mSavedRespawnX: 0,
            mSavedRespawnY: 0,
            mSavedSpeedBonus: 0,
            mSavedGravBonus: 0,
            mSavedJumpBonus: 0,
            mSavedTileCount: 0,
            mSavedTiles: Vec::new(),
            mSavedMenuItem: 0,
            mMainMenuItems: Default::default(),
            tileX: 0,
            tileY: 0,
            divTileX: 0,
            divTileY: 0,
            divisorLine: 0,
            rightDrawEdge: 0,
            leftDrawEdge: 0,
            scrollFlag: false,
            scrollOffset: 0,
            mExitPos: 0,
            mGameBuffer: Surface::new(156, 96, f()),
            tileImages: Vec::new(),
            tmpTileImage: Surface::new(12, 12, f()),
            hoopList: Vec::new(),
            mLevelNum: 0,
            mLevelNumStr: String::new(),
            mLevelCompletedStr: String::new(),
            mLoadLevelFlag: false,
            mStartCol: 0,
            mStartRow: 0,
            mStartBallSize: 0,
            mExitPosX: 0,
            mExitPosY: 0,
            tileMap: None,
            mTileMapWidth: 0,
            mTileMapHeight: 0,
            mTotalNumRings: 0,
            mNumMoveObj: 0,
            mMOTopLeft: Vec::new(),
            mMOBotRight: Vec::new(),
            mMODirection: Vec::new(),
            mMOOffset: Vec::new(),
            mSpikeImgPtr: None,
            mUILife: blank.clone(),
            mUIRing: blank.clone(),
            mTopLeftExitTileCol: 0,
            mTopLeftExitTileRow: 0,
            mExitTileImage: Surface::new(24, 24, f()),
            mImgPtr: blank,
            mImageOffset: 0,
            mOpenFlag: false,
            mWidth: SCREEN_W,
            mHeight: SCREEN_H,
            mSplashIndex: 0,
            mSplashImage: None,
            mSplashTimer: 0,
            mSoundHoop: Sound::new(&[]),
            mSoundPickup: Sound::new(&[]),
            mSoundPop: Sound::new(&[]),
            hasBall: false,
            mBall: Ball::default(),
            numRings: 0,
            numLives: 0,
            mScore: 0,
            bonusCntrValue: 0,
            mLevelDisCntr: 0,
            mLeaveGame: false,
            mOpenExitFlag: false,
            mPaintUIFlag: false,
            mFullScreenBuffer: Surface::new(128, 128, f()),
            mClearScreenFlag: false,
            mCheat: false,
            mInvincible: false,
            mCheatSeq: 0,
            mIncomingCall: true,
        };
        g.BounceUI();
        g
    }

    // ------------------------------------------------------------ Display
    /// `Display.setCurrent(canvas)`
    pub fn show_canvas(&mut self) {
        self.current = Current::Canvas;
    }

    /// `Display.setCurrent(screen)`; the canvas gets `hideNotify` when it
    /// goes out of view.
    pub fn show_screen(&mut self, s: Screen) {
        let was_canvas = matches!(self.current, Current::Canvas);
        self.current = Current::Screen(s);
        if was_canvas {
            self.hideNotify();
        }
    }

    pub fn screen_is_form(&self) -> bool {
        matches!(self.current, Current::Screen(_))
    }

    pub fn current_tag(&self) -> Option<lcdui::Tag> {
        match &self.current {
            Current::Screen(s) => Some(s.tag),
            Current::Canvas => None,
        }
    }

    // ------------------------------------------------------------ timer
    /// `TileCanvas.start()`: a Timer firing `run()` now and every 30 ms.
    pub fn start(&mut self) {
        if self.timer.is_none() {
            self.timer = Some(0.0);
        }
    }

    /// `TileCanvas.stop()`
    pub fn stop(&mut self) {
        self.timer = None;
    }

    /// Advance real time by `dt` ms and draw the current display into `screen`.
    pub fn update(&mut self, dt: f64, screen: &mut Surface) {
        let mut left = dt;
        let mut budget = 8; // after a stall, drop the backlog instead of racing
        while let Some(t) = self.timer {
            if t > left {
                self.timer = Some(t - left);
                break;
            }
            if budget == 0 {
                break;
            }
            budget -= 1;
            left -= t;
            self.timer = Some(TICK_MS);
            // run() may stop the timer, or stop and start it (fires again at once)
            self.run();
            if self.quit {
                return;
            }
        }
        self.paint_current(screen);
    }

    fn paint_current(&mut self, screen: &mut Surface) {
        match &mut self.current {
            Current::Canvas => self.paint(screen),
            Current::Screen(s) => s.draw(screen),
        }
    }

    // ------------------------------------------------------------ input
    pub fn key_pressed(&mut self, code: i32) {
        self.track_held(code, true);
        match &mut self.current {
            Current::Canvas => {
                // port-only: Fire / Space / A jump too (the phone had no use for Fire in play)
                let code = if code == K_FIRE { K_UP } else { code };
                self.keyPressed(code);
            }
            Current::Screen(s) => {
                if let Some(c) = s.key(code, &self.font) {
                    self.commandAction(c);
                }
            }
        }
    }

    pub fn key_released(&mut self, code: i32) {
        self.track_held(code, false);
        let code = if code == K_FIRE { K_UP } else { code };
        // Released directions always reach the ball, even when a menu came up
        // while the key was held, so it can't keep rolling by itself afterwards.
        if matches!(self.current, Current::Canvas) || self.hasBall {
            self.keyReleased(code);
        }
    }

    fn track_held(&mut self, code: i32, down: bool) {
        let bit = match code {
            K_UP | K_FIRE | 50 => 8,
            K_DOWN => 4,
            K_RIGHT | 54 => 2,
            K_LEFT | 52 => 1,
            _ => 0,
        };
        if down {
            self.held |= bit;
        } else {
            self.held &= !bit;
        }
    }

    /// The window was minimised: like an incoming call on the phone, the
    /// canvas is hidden and the game drops to the menu.
    pub fn system_hide(&mut self) {
        if matches!(self.current, Current::Canvas) {
            self.hideNotify();
        }
    }

    /// Window closed: `Bounce.destroyApp(true)`.
    pub fn close(&mut self) {
        self.destroyApp();
        self.quit = true;
    }
}
