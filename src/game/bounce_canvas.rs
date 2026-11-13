//! `BounceCanvas`: the game loop (`run`, every 30 ms), HUD, splash screen,
//! keys and cheats.

use super::ball::Ball;
use super::Game;
use crate::midp::assets;
use crate::midp::gfx::{Surface, HCENTER, LEFT, TOP, VCENTER};
use crate::midp::lcdui::game_action;
use crate::midp::sound::Sound;

const AT: i32 = TOP | LEFT; // anchor 20
const SPLASH_NAME: [&str; 2] = ["/icons/nokiagames.png", "/icons/bouncesplash.png"];

impl Game {
    /// `new BounceCanvas(ui, 1)`
    pub(super) fn BounceCanvas(&mut self) {
        self.TileCanvas();
        self.mSoundHoop = Self::loadSound("/sounds/up.ott");
        self.mSoundPickup = Self::loadSound("/sounds/pickup.ott");
        self.mSoundPop = Self::loadSound("/sounds/pop.ott");
        self.mFullScreenBuffer = Surface::new(128, 128, self.font.clone());
        self.mSplashIndex = 1;
        self.mSplashImage = assets::image(SPLASH_NAME[self.mSplashIndex as usize]);
        self.start();
    }

    pub fn resetGame(&mut self, var1: i32) {
        self.mLevelNum = var1;
        self.numRings = 0;
        self.numLives = 3;
        self.mScore = 0;
        self.mLeaveGame = false;
        self.mOpenExitFlag = false;
        self.mClearScreenFlag = false;
        self.createNewLevel();
    }

    /// `resetGame(int, int)`: continue the saved game.
    pub fn resetGameSaved(&mut self, var1: i32, var2: i32) {
        self.mLevelNum = self.mSavedLevel;
        self.numRings = self.mSavedRings;
        self.numLives = self.mSavedLives;
        self.mScore = self.mSavedScore;
        self.disposeLevel();
        self.loadLevel(self.mLevelNum);
        self.resetTiles();
        self.mLevelDisCntr = 120;
        self.mPaintUIFlag = true;
        if self.mSavedRespawnX != self.mStartCol && self.mSavedRespawnY != self.mStartRow {
            let (rx, ry) = (self.mSavedRespawnX, self.mSavedRespawnY);
            let v = 8 | self.tm(ry, rx) & 64;
            self.set_tm(ry, rx, v);
        }
        self.createBufferFocused(var1, var2, self.mSavedSize, self.mSavedXSpeed, self.mSavedYSpeed);
        self.mBall.setRespawn(self.mSavedRespawnX, self.mSavedRespawnY);
        self.mBall.speedBonusCntr = self.mSavedSpeedBonus;
        self.mBall.gravBonusCntr = self.mSavedGravBonus;
        self.mBall.jumpBonusCntr = self.mSavedJumpBonus;
    }

    fn createNewLevel(&mut self) {
        self.disposeLevel();
        self.loadLevel(self.mLevelNum);
        self.numRings = 0;
        self.mLevelDisCntr = 120;
        self.mPaintUIFlag = true;
        self.createBufferFocused(
            self.mStartCol * 12 + (self.mStartBallSize >> 1),
            self.mStartRow * 12 + (self.mStartBallSize >> 1),
            self.mStartBallSize,
            0,
            0,
        );
        self.mBall.setRespawn(self.mStartCol, self.mStartRow);
    }

    pub fn createBufferFocused(&mut self, var1: i32, var2: i32, var3: i32, var4: i32, var5: i32) {
        let var6 = var1 / 12;
        let var7 = var2 / 12;
        if var1 < 64 {
            self.scrollFlag = false;
            self.scrollOffset = 64;
            self.rightDrawEdge = 128;
            self.leftDrawEdge = 0;
            self.tileX = 0;
        } else if var1 > self.mTileMapWidth * 12 - 64 {
            self.scrollFlag = false;
            self.scrollOffset = 92;
            self.rightDrawEdge = 156;
            self.leftDrawEdge = 28;
            self.tileX = self.mTileMapWidth - 13;
        } else {
            self.scrollFlag = true;
            self.scrollOffset = 0;
            self.rightDrawEdge = 143;
            self.leftDrawEdge = 15;
            self.tileX = var6 - 6;
        }
        self.divisorLine = 156;
        self.tileY = var7 / 7 * 7;
        self.divTileX = self.tileX + 13;
        self.divTileY = self.tileY;
        let var8 = var1 - self.tileX * 12;
        let var9 = var2 - self.tileY * 12 - 1;
        self.mBall = Ball::new(var8, var9, var3);
        self.hasBall = true;
        self.mBall.xSpeed = var4;
        self.mBall.ySpeed = var5;
        // port-only: a key still held after a respawn keeps working (the phone
        // only reported presses, so the new ball ignored it until pressed again)
        self.mBall.direction = self.held;
        self.createNewBuffer();
    }

    pub fn checkData(&mut self) {
        if self.mLevelNum > self.mBestLevel {
            self.mBestLevel = self.mLevelNum;
            self.saveGameData(1);
        }
        if self.mScore > self.mBestScore {
            self.mBestScore = self.mScore;
            self.mNewBestScore = true;
            self.saveGameData(2);
        }
        self.mLastScore = self.mScore;
    }

    pub fn screenFlip(&mut self) {
        let var1 = self.mBall.xPos / 12;
        let mut var2 = self.mBall.xPos - var1 * 12 - 6;
        self.cleanBuffer(false);
        if self.mBall.yPos < 0 {
            self.tileY -= 7;
            self.divTileY -= 7;
            self.mBall.yPos += 84;
        } else if self.mBall.yPos > 96 {
            self.tileY += 7;
            self.divTileY += 7;
            self.mBall.yPos -= 84;
        }

        if !self.scrollFlag && self.tileX - (13 - self.divisorLine / 12) == 0 {
            if self.divisorLine < self.mBall.xPos {
                self.mBall.xPos -= self.divisorLine;
            } else {
                self.mBall.xPos = self.mBall.xPos - self.divisorLine + 156;
            }
            self.tileX = 0;
            self.leftDrawEdge = 0;
            self.rightDrawEdge = 128;
            self.scrollOffset = 64;
        } else if !self.scrollFlag {
            self.tileX = self.mTileMapWidth - 13;
            self.leftDrawEdge = 28;
            self.rightDrawEdge = 156;
            if self.mBall.xPos > self.divisorLine {
                self.mBall.xPos = 156 - (self.divisorLine + 156 - self.mBall.xPos);
            } else {
                self.mBall.xPos = 156 - (self.divisorLine - self.mBall.xPos);
            }
            self.scrollOffset = 92;
        } else {
            if self.mBall.xPos > self.divisorLine {
                self.tileX = self.tileX - 13 + var1 - 6;
            } else {
                self.tileX = self.tileX + var1 - 6;
            }
            if self.tileX < 0 {
                var2 += self.tileX * 12;
                self.tileX = 0;
            } else if self.tileX > self.mTileMapWidth - 13 - 1 {
                var2 += (self.tileX - self.mTileMapWidth - 13 - 1) * 12;
                self.tileX = self.mTileMapWidth - 13 - 1;
            }
            self.leftDrawEdge = 14 + var2;
            self.rightDrawEdge = 142 + var2;
            self.mBall.xPos = 78 + var2;
        }

        self.divTileX = self.tileX + 13;
        self.divisorLine = 156;
        self.createNewBuffer();
    }

    pub fn add2Score(&mut self, var1: i32) {
        self.mScore += var1;
        self.mPaintUIFlag = true;
    }

    pub fn paint2Buffer(&mut self) {
        if self.hasBall {
            self.ball_paint();
        }
        for (img, x, y) in std::mem::take(&mut self.hoopList) {
            self.mGameBuffer.draw_image(&img, x, y, AT);
        }

        let fs = &mut self.mFullScreenBuffer;
        if self.leftDrawEdge < self.rightDrawEdge {
            fs.draw_image(&self.mGameBuffer.img, -self.leftDrawEdge, 0, AT);
        } else {
            fs.draw_image(&self.mGameBuffer.img, -self.leftDrawEdge, 0, AT);
            fs.draw_image(&self.mGameBuffer.img, 156 - self.leftDrawEdge, 0, AT);
        }

        if self.mLevelDisCntr != 0 {
            fs.set_color(16777214);
            fs.draw_string(&self.mLevelNumStr, 44, 84, AT);
        }

        if self.mPaintUIFlag {
            fs.set_color(545706);
            fs.fill_rect(0, 97, 128, 32);
            for var4 in 0..self.numLives {
                fs.draw_image(&self.mUILife, 5 + var4 * (self.mUILife.w - 1), 99, AT);
            }
            for var5 in 0..self.mTotalNumRings - self.numRings {
                fs.draw_image(&self.mUIRing, 5 + var5 * (self.mUIRing.w - 1), 112, AT);
            }
            fs.set_color(16777214);
            fs.draw_string(&Self::zeroString(self.mScore), 64, 100, AT);
            if self.bonusCntrValue != 0 {
                fs.set_color(16750611);
                fs.fill_rect(1, 128 - 3 * self.bonusCntrValue / 30, 5, 128);
            }
            self.mPaintUIFlag = false;
        }
    }

    /// `paint(Graphics)` onto the phone's screen.
    pub fn paint(&mut self, var1: &mut Surface) {
        var1.set_clip(0, 0, self.mWidth, self.mHeight);
        if self.mSplashIndex != -1 {
            if let Some(img) = &self.mSplashImage {
                var1.set_color(0);
                var1.fill_rect(0, 0, self.mWidth, self.mHeight);
                var1.draw_image(img, self.mWidth >> 1, self.mHeight >> 1, HCENTER | VCENTER);
            }
        } else if self.mClearScreenFlag {
            var1.set_color(0);
            var1.fill_rect(0, 0, self.mWidth, self.mHeight);
            self.mClearScreenFlag = false;
        } else {
            var1.draw_image(&self.mFullScreenBuffer.img, 0, 0, AT);
        }
    }

    /// `run()`: one GameTimer tick.
    pub fn run(&mut self) {
        if self.mLoadLevelFlag {
            self.createNewLevel();
        } else if self.mSplashIndex != -1 {
            if self.mSplashImage.is_none() {
                self.mIncomingCall = false;
                self.displayMainMenu();
            } else if self.mSplashTimer > 30 {
                self.mSplashImage = None;
                match self.mSplashIndex {
                    0 => {
                        self.mSplashIndex = 1;
                        self.mSplashImage = assets::image(SPLASH_NAME[1]);
                    }
                    1 => {
                        self.mSplashIndex = -1;
                        self.mIncomingCall = false;
                        self.displayMainMenu();
                    }
                    _ => {}
                }
                self.mSplashTimer = 0;
            } else {
                self.mSplashTimer += 1;
            }
        } else {
            if !self.hasBall || self.tileMap.is_none() {
                return; // Java: NullPointerException ends the tick
            }
            if self.mLevelDisCntr != 0 {
                self.mLevelDisCntr -= 1;
            }

            if self.mBall.yPos >= 0 && self.mBall.yPos <= 96 {
                self.cleanBuffer(true);
                self.ball_update();
                self.testScroll(self.mBall.xPos, self.mBall.xOffset);
            } else {
                self.screenFlip();
            }

            if self.mBall.ballState == 1 {
                if self.numLives < 0 {
                    self.checkData();
                    self.stop();
                    self.mIncomingCall = false;
                    self.gameOver();
                    return;
                }
                let var1 = self.mBall.respawnX;
                let var2 = self.mBall.respawnY;
                let var3 = self.mBall.respawnSize;
                self.createBufferFocused(
                    self.mBall.respawnX * 12 + (self.mBall.respawnSize >> 1),
                    self.mBall.respawnY * 12 + (self.mBall.respawnSize >> 1),
                    self.mBall.respawnSize,
                    0,
                    0,
                );
                self.mBall.respawnX = var1;
                self.mBall.respawnY = var2;
                self.mBall.respawnSize = var3;
            }

            if self.mNumMoveObj != 0 {
                self.updateMovingSpikeObj();
            }

            if self.numRings == self.mTotalNumRings {
                self.mOpenExitFlag = true;
            }

            if self.mOpenExitFlag && self.mExitPos != -1 {
                let mut var5 = self.leftDrawEdge;
                let mut var6 = self.rightDrawEdge;
                if self.mExitPos <= self.divisorLine {
                    if self.leftDrawEdge > self.divisorLine {
                        var5 = self.leftDrawEdge - 156;
                    }
                    if self.rightDrawEdge > self.divisorLine {
                        var6 = self.rightDrawEdge - 156;
                    }
                }
                if self.mExitPos > self.divisorLine {
                    if self.leftDrawEdge < self.divisorLine {
                        var5 = self.leftDrawEdge + 156;
                    }
                    if self.rightDrawEdge < self.divisorLine {
                        var6 = self.rightDrawEdge + 156;
                    }
                }
                if self.mExitPos >= var5 && self.mExitPos <= var6 {
                    if self.mOpenFlag {
                        self.mExitPos = -1;
                        self.mOpenExitFlag = false;
                    } else {
                        self.openExit();
                    }
                    let (r, c) = (self.mTopLeftExitTileRow, self.mTopLeftExitTileCol);
                    for (dr, dc) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
                        let v = self.tm(r + dr, c + dc) | 128;
                        self.set_tm(r + dr, c + dc, v);
                    }
                    self.cleanBuffer(true);
                }
            }

            self.bonusCntrValue = 0;
            let b = &self.mBall;
            if b.speedBonusCntr != 0 || b.gravBonusCntr != 0 || b.jumpBonusCntr != 0 {
                self.bonusCntrValue = self.bonusCntrValue.max(b.speedBonusCntr).max(b.gravBonusCntr).max(b.jumpBonusCntr);
                if self.bonusCntrValue % 30 == 0 || self.bonusCntrValue == 1 {
                    self.mPaintUIFlag = true;
                }
            }

            self.scrollBuffer(self.mBall.xPos, self.mBall.xOffset, 16);
            self.paint2Buffer();
            if self.mLeaveGame {
                self.mLeaveGame = false;
                self.mOpenExitFlag = false;
                self.mLoadLevelFlag = true;
                self.mLevelNum += 1;
                self.add2Score(5000);
                self.checkData();
                if self.mLevelNum >= 11 {
                    self.mIncomingCall = false;
                    self.displayGameOver();
                } else {
                    self.mIncomingCall = false;
                    self.displayLevelComplete();
                    self.mClearScreenFlag = false;
                }
            }
        }
    }

    pub fn keyPressed(&mut self, var1: i32) {
        if self.mSplashIndex != -1 {
            self.mSplashTimer = 31;
        } else if self.hasBall {
            match var1 {
                -7 | -6 => {
                    self.mIncomingCall = false;
                    self.displayMainMenu();
                }
                -5 => {}
                49 => {
                    if self.mCheat {
                        self.mLoadLevelFlag = true;
                        self.mLevelNum -= 1;
                        if self.mLevelNum < 1 {
                            self.mLevelNum = 11;
                        }
                    }
                }
                51 => {
                    if self.mCheat {
                        self.mLoadLevelFlag = true;
                        self.mLevelNum += 1;
                        if self.mLevelNum > 11 {
                            self.mLevelNum = 1;
                        }
                    }
                }
                55 => {
                    if self.mCheatSeq != 0 && self.mCheatSeq != 2 {
                        self.mCheatSeq = 0;
                    } else {
                        self.mCheatSeq += 1;
                    }
                }
                56 => {
                    if self.mCheatSeq == 1 || self.mCheatSeq == 3 {
                        self.mCheatSeq += 1;
                    } else if self.mCheatSeq == 5 {
                        let s = self.mSoundHoop.clone();
                        self.tones.play(&s);
                        self.mInvincible = true;
                        self.mCheatSeq = 0;
                    } else {
                        self.mCheatSeq = 0;
                    }
                }
                57 => {
                    if self.mCheatSeq == 4 {
                        self.mCheatSeq += 1;
                    } else if self.mCheatSeq == 5 {
                        let s = self.mSoundPop.clone();
                        self.tones.play(&s);
                        self.mCheat = true;
                        self.mCheatSeq = 0;
                    } else {
                        self.mCheatSeq = 0;
                    }
                }
                _ => match game_action(var1) {
                    1 => self.mBall.setDirection(8),
                    2 => self.mBall.setDirection(1),
                    5 => self.mBall.setDirection(2),
                    6 => self.mBall.setDirection(4),
                    _ => {}
                },
            }
        }
    }

    pub fn keyReleased(&mut self, var1: i32) {
        if self.hasBall {
            match game_action(var1) {
                1 => self.mBall.releaseDirection(8),
                2 => self.mBall.releaseDirection(1),
                5 => self.mBall.releaseDirection(2),
                6 => self.mBall.releaseDirection(4),
                _ => {}
            }
        }
    }

    pub fn zeroString(var0: i32) -> String {
        let var1 = if var0 < 100 {
            "0000000"
        } else if var0 < 1000 {
            "00000"
        } else if var0 < 10000 {
            "0000"
        } else if var0 < 100000 {
            "000"
        } else if var0 < 1000000 {
            "00"
        } else if var0 < 10000000 {
            "0"
        } else {
            ""
        };
        format!("{var1}{var0}")
    }

    fn loadSound(var1: &str) -> Sound {
        // the original reads at most 100 bytes
        let data = assets::get(var1).unwrap_or(&[]);
        Sound::new(&data[..data.len().min(100)])
    }

    pub fn hideNotify(&mut self) {
        if self.mIncomingCall {
            if self.hasBall {
                self.mBall.resetDirections();
            }
            self.displayMainMenu();
        }
        self.mIncomingCall = true;
    }

    pub fn resetTiles(&mut self) {
        for var2 in 0..self.mTileMapHeight {
            for var3 in 0..self.mTileMapWidth {
                let var1 = self.tm(var2, var3) & 0xFF7F & -65;
                let to = match var1 {
                    7 | 29 => 0,
                    13 => 17,
                    14 => 18,
                    15 => 19,
                    16 => 20,
                    21 => 25,
                    22 => 26,
                    23 => 27,
                    24 => 28,
                    _ => continue,
                };
                if self.tileNotSavedAsActive(var2, var3) {
                    let v = to | self.tm(var2, var3) & 64;
                    self.set_tm(var2, var3, v);
                }
            }
        }
    }

    pub fn tileNotSavedAsActive(&self, var1: i32, var2: i32) -> bool {
        !self.mSavedTiles.iter().take(self.mSavedTileCount.max(0) as usize).any(|t| t[0] == var1 && t[1] == var2)
    }
}
