//! `Ball`: physics, collision against the tile map, pickups.

use super::Game;
use crate::midp::gfx::{TOP, LEFT};
use crate::midp::sound::Sound;

const AT: i32 = TOP | LEFT; // anchor 20

pub const TRI_TILE_DATA: [[u8; 12]; 12] = [
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
    [0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1],
    [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1],
    [0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1],
    [0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1],
    [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1],
    [0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1],
    [0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
];

pub const SMALL_BALL_DATA: [[u8; 12]; 12] = [
    [0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0],
    [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0],
    [0, 0, 0, 0, 1, 1, 1, 1, 0, 0, 0, 0],
];

pub const LARGE_BALL_DATA: [[u8; 16]; 16] = [
    [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
    [0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0],
    [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0],
    [0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0],
    [0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0, 0, 0],
    [0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 0, 0, 0, 0, 0],
];

/// Ball images are `tileImages` entries (setBallImages).
const SMALL_BALL_IMAGE: usize = 47;
const POPPED_IMAGE: usize = 48;
const LARGE_BALL_IMAGE: usize = 49;

/// Bit of the ball's shape, `var11[row][col]`.
fn ball_px(size: i32, row: i32, col: i32) -> u8 {
    if size == 16 {
        LARGE_BALL_DATA[row as usize][col as usize]
    } else {
        SMALL_BALL_DATA[row as usize][col as usize]
    }
}

#[derive(Default, Clone)]
pub struct Ball {
    pub xPos: i32,
    pub yPos: i32,
    pub globalBallX: i32,
    pub globalBallY: i32,
    pub xOffset: i32,
    pub xSpeed: i32,
    pub ySpeed: i32,
    pub direction: i32,
    pub mBallSize: i32,
    pub mHalfBallSize: i32,
    pub respawnX: i32,
    pub respawnY: i32,
    pub respawnSize: i32,
    pub ballState: i32,
    pub jumpOffset: i32,
    pub speedBonusCntr: i32,
    pub gravBonusCntr: i32,
    pub jumpBonusCntr: i32,
    pub mGroundedFlag: bool,
    pub mCDRubberFlag: bool,
    pub mCDRampFlag: bool,
    pub slideCntr: i32,
    /// index into tileImages
    pub mBallImage: usize,
    popCntr: i32,
}

impl Ball {
    /// `new Ball(x, y, size, canvas)`
    pub fn new(x: i32, y: i32, size: i32) -> Ball {
        let mut b = Ball { xPos: x, yPos: y, ..Default::default() };
        b.mBallSize = size;
        if b.mBallSize == 12 {
            b.mHalfBallSize = 6;
            b.mBallImage = SMALL_BALL_IMAGE;
        } else {
            b.mHalfBallSize = 8;
            b.mBallImage = LARGE_BALL_IMAGE;
        }
        b
    }

    pub fn setRespawn(&mut self, x: i32, y: i32) {
        self.respawnX = x;
        self.respawnY = y;
        self.respawnSize = self.mBallSize;
    }

    pub fn setDirection(&mut self, d: i32) {
        if d == 8 || d == 4 || d == 2 || d == 1 {
            self.direction |= d;
        }
    }

    pub fn releaseDirection(&mut self, d: i32) {
        if d == 8 || d == 4 || d == 2 || d == 1 {
            self.direction &= !d;
        }
    }

    pub fn resetDirections(&mut self) {
        self.direction &= -16;
    }
}

impl Game {
    /// `tileMap[r][c]`; outside the map reads as empty space (Java would throw).
    pub fn tm(&self, r: i32, c: i32) -> i32 {
        self.tileMap
            .as_ref()
            .and_then(|m| m.get(r as usize))
            .and_then(|row| row.get(c as usize))
            .copied()
            .filter(|_| r >= 0 && c >= 0)
            .unwrap_or(0)
    }

    /// `tileMap[r][c] = (short) v`; writes outside the map are dropped.
    pub fn set_tm(&mut self, r: i32, c: i32, v: i32) {
        if r < 0 || c < 0 {
            return;
        }
        if let Some(cell) = self.tileMap.as_mut().and_then(|m| m.get_mut(r as usize)).and_then(|row| row.get_mut(c as usize)) {
            *cell = v as i16 as i32;
        }
    }

    fn play(&mut self, s: &Sound) {
        self.tones.play(s);
    }

    pub fn collisionDetection(&mut self, var1: i32, var2: i32) -> bool {
        let mut var3 = 0;
        if var2 < 0 {
            var3 = 12;
        }
        let b = &mut self.mBall;
        let var4 = (var1 - b.mHalfBallSize) / 12;
        let var5 = (var2 - var3 - b.mHalfBallSize) / 12;
        b.globalBallX = var1 - b.mHalfBallSize;
        b.globalBallY = var2 - b.mHalfBallSize;
        if b.xPos < self.divisorLine {
            b.globalBallX += self.tileX * 12;
            b.globalBallY += self.tileY * 12;
        } else {
            b.globalBallX += (self.divTileX - 13) * 12 - self.divisorLine;
            b.globalBallY += self.divTileY * 12;
        }

        let var6 = (var1 - 1 + b.mHalfBallSize) / 12 + 1;
        let var7 = (var2 - var3 - 1 + b.mHalfBallSize) / 12 + 1;
        let mut var8 = true;

        for var9 in var4..var6 {
            for var10 in var5..var7 {
                if var9 * 12 > 156 {
                    var8 = self.testTile(self.tileY + var10, self.tileX + var9 - 13, var8);
                } else if self.mBall.xPos < self.divisorLine {
                    var8 = self.testTile(self.tileY + var10, self.tileX + var9, var8);
                } else {
                    let var11 = self.divTileX - 13 - self.divisorLine / 12;
                    var8 = self.testTile(self.divTileY + var10, var11 + var9, var8);
                }
            }
        }
        var8
    }

    pub fn enlargeBall(&mut self) {
        let mut var1 = 2;
        self.mBall.mBallSize = 16;
        self.mBall.mHalfBallSize = 8;
        self.mBall.mBallImage = LARGE_BALL_IMAGE;
        let mut var2 = false;
        while !var2 {
            var2 = true;
            let (x, y) = (self.mBall.xPos, self.mBall.yPos);
            if self.collisionDetection(x, y - var1) {
                self.mBall.yPos -= var1;
            } else if self.collisionDetection(x - var1, y - var1) {
                self.mBall.xPos -= var1;
                self.mBall.yPos -= var1;
            } else if self.collisionDetection(x + var1, y - var1) {
                self.mBall.xPos += var1;
                self.mBall.yPos -= var1;
            } else if self.collisionDetection(x, y + var1) {
                self.mBall.yPos += var1;
            } else if self.collisionDetection(x - var1, y + var1) {
                self.mBall.xPos -= var1;
                self.mBall.yPos += var1;
            } else if self.collisionDetection(x + var1, y + var1) {
                self.mBall.xPos += var1;
                self.mBall.yPos += var1;
            } else {
                var2 = false;
                var1 += 1;
                if var1 > 64 {
                    // port-only guard: Java would spin here forever
                    crate::log!("enlargeBall: no room to grow");
                    break;
                }
            }
        }
    }

    pub fn shrinkBall(&mut self) {
        let var1 = 2;
        self.mBall.mBallSize = 12;
        self.mBall.mHalfBallSize = 6;
        self.mBall.mBallImage = SMALL_BALL_IMAGE;
        let (x, y) = (self.mBall.xPos, self.mBall.yPos);
        if self.collisionDetection(x, y + var1) {
            self.mBall.yPos += var1;
        } else if self.collisionDetection(x, y - var1) {
            self.mBall.yPos -= var1;
        }
    }

    pub fn popBall(&mut self) {
        if !self.mInvincible {
            let b = &mut self.mBall;
            b.popCntr = 5;
            b.ballState = 2;
            b.xOffset = 0;
            self.numLives -= 1;
            b.speedBonusCntr = 0;
            b.gravBonusCntr = 0;
            b.jumpBonusCntr = 0;
            self.rumble = Some(());
            self.mPaintUIFlag = true;
            let s = self.mSoundPop.clone();
            self.play(&s);
        }
    }

    pub fn addRing(&mut self) {
        self.add2Score(500);
        self.numRings += 1;
        self.mPaintUIFlag = true;
    }

    pub fn redirectBall(&mut self, var1: i32) {
        let b = &mut self.mBall;
        let var2 = b.xSpeed;
        match var1 {
            30 | 32 => {
                b.xSpeed = -(b.ySpeed >> 1);
                b.ySpeed = -(var2 >> 1);
            }
            31 | 33 => {
                b.xSpeed = b.ySpeed >> 1;
                b.ySpeed = var2 >> 1;
            }
            34 | 36 => {
                b.xSpeed = -b.ySpeed;
                b.ySpeed = -var2;
            }
            35 | 37 => {
                b.xSpeed = b.ySpeed;
                b.ySpeed = var2;
            }
            _ => {}
        }
    }

    pub fn squareCollide(&self, var1: i32, var2: i32) -> bool {
        let b = &self.mBall;
        let var3 = var2 * 12;
        let var4 = var1 * 12;
        let var5 = b.globalBallX - var3;
        let var6 = b.globalBallY - var4;
        let (var7, mut var8) = if var5 >= 0 { (var5, 12) } else { (0, b.mBallSize + var5) };
        let (var9, mut var10) = if var6 >= 0 { (var6, 12) } else { (0, b.mBallSize + var6) };
        if var8 > 12 {
            var8 = 12;
        }
        if var10 > 12 {
            var10 = 12;
        }
        for var12 in var7..var8 {
            for var13 in var9..var10 {
                if ball_px(b.mBallSize, var13 - var6, var12 - var5) != 0 {
                    return true;
                }
            }
        }
        false
    }

    pub fn triangleCollide(&mut self, var1: i32, var2: i32, var3: i32) -> bool {
        let var4 = var2 * 12;
        let var5 = var1 * 12;
        let var6 = self.mBall.globalBallX - var4;
        let var7 = self.mBall.globalBallY - var5;
        let mut var8 = 0;
        let mut var9 = 0;
        match var3 {
            30 | 34 => {
                var9 = 11;
                var8 = 11;
            }
            31 | 35 => var9 = 11,
            33 | 37 => var8 = 11,
            _ => {}
        }
        let size = self.mBall.mBallSize;
        let (var10, mut var11) = if var6 >= 0 { (var6, 12) } else { (0, size + var6) };
        let (var12, mut var13) = if var7 >= 0 { (var7, 12) } else { (0, size + var7) };
        if var11 > 12 {
            var11 = 12;
        }
        if var13 > 12 {
            var13 = 12;
        }
        for var15 in var10..var11 {
            for var16 in var12..var13 {
                let tri = TRI_TILE_DATA[(var16 - var9).unsigned_abs() as usize][(var15 - var8).unsigned_abs() as usize];
                if tri & ball_px(size, var16 - var7, var15 - var6) != 0 {
                    if !self.mBall.mGroundedFlag {
                        self.redirectBall(var3);
                    }
                    return true;
                }
            }
        }
        false
    }

    pub fn thinCollide(&self, var1: i32, var2: i32, var3: i32) -> bool {
        let mut var4 = var2 * 12;
        let mut var5 = var1 * 12;
        let mut var6 = var4 + 12;
        let mut var7 = var5 + 12;
        match var3 {
            3 | 5 | 9 | 13 | 14 | 17 | 18 | 21 | 22 | 43 | 45 => {
                var4 += 4;
                var6 -= 4;
            }
            4 | 6 | 15 | 16 | 19 | 20 | 23 | 24 | 44 | 46 => {
                var5 += 4;
                var7 -= 4;
            }
            _ => {}
        }
        let b = &self.mBall;
        Self::rectCollide(b.globalBallX, b.globalBallY, b.globalBallX + b.mBallSize, b.globalBallY + b.mBallSize, var4, var5, var6, var7)
    }

    pub fn edgeCollide(&self, var1: i32, var2: i32, var3: i32) -> bool {
        let mut var4 = var2 * 12;
        let mut var5 = var1 * 12;
        let mut var6 = var4 + 12;
        let mut var7 = var5 + 12;
        let b = &self.mBall;
        let hit = |a: i32, c: i32, d: i32, e: i32| {
            Self::rectCollide(b.globalBallX, b.globalBallY, b.globalBallX + b.mBallSize, b.globalBallY + b.mBallSize, a, c, d, e)
        };
        match var3 {
            13 | 17 => {
                var4 += 6;
                var6 -= 6;
                var7 -= 11;
                hit(var4, var5, var6, var7)
            }
            14 | 18 | 22 | 26 => {
                var4 += 6;
                var6 -= 6;
                var5 += 11;
                hit(var4, var5, var6, var7)
            }
            15 | 19 | 23 | 27 => {
                var5 += 6;
                var7 -= 6;
                var6 -= 11;
                hit(var4, var5, var6, var7)
            }
            16 | 20 | 24 | 28 => {
                var5 += 6;
                var7 -= 6;
                var4 += 11;
                hit(var4, var5, var6, var7)
            }
            21 | 25 => {
                // `var7 = var5--;`
                var7 = var5;
                var5 -= 1;
                var4 += 6;
                var6 -= 6;
                hit(var4, var5, var6, var7)
            }
            _ => false,
        }
    }

    pub fn testTile(&mut self, var1: i32, var2: i32, mut var3: bool) -> bool {
        if var1 >= self.mTileMapHeight || var1 < 0 || var2 >= self.mTileMapWidth || var2 < 0 {
            return false;
        }
        if self.mBall.ballState == 2 {
            return false;
        }
        let var4 = self.tm(var1, var2) & 64;
        let var5 = self.tm(var1, var2) & -65 & -129;
        let mut var6: Option<Sound> = None;
        match var5 {
            1 => {
                if self.squareCollide(var1, var2) {
                    var3 = false;
                } else {
                    self.mBall.mCDRampFlag = true;
                }
            }
            2 => {
                if self.squareCollide(var1, var2) {
                    self.mBall.mCDRubberFlag = true;
                    var3 = false;
                } else {
                    self.mBall.mCDRampFlag = true;
                }
            }
            3..=6 => {
                if self.thinCollide(var1, var2, var5) {
                    var3 = false;
                    self.popBall();
                }
            }
            7 => {
                self.add2Score(200);
                let (rx, ry) = (self.mBall.respawnX, self.mBall.respawnY);
                self.set_tm(ry, rx, 128);
                self.mBall.setRespawn(var2, var1);
                self.set_tm(var1, var2, 136);
                var6 = Some(self.mSoundPickup.clone());
            }
            9 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.mOpenFlag {
                        self.mLeaveGame = true;
                        var6 = Some(self.mSoundPickup.clone());
                    } else {
                        var3 = false;
                    }
                }
            }
            10 => {
                let var7 = self.findSpikeIndex(var2, var1);
                if var7 != -1 {
                    let i = var7 as usize;
                    let var8 = self.mMOTopLeft[i][0] * 12 + self.mMOOffset[i][0];
                    let var9 = self.mMOTopLeft[i][1] * 12 + self.mMOOffset[i][1];
                    let b = &self.mBall;
                    if Self::rectCollide(
                        b.globalBallX,
                        b.globalBallY,
                        b.globalBallX + b.mBallSize,
                        b.globalBallY + b.mBallSize,
                        var8,
                        var9,
                        var8 + 24,
                        var9 + 24,
                    ) {
                        var3 = false;
                        self.popBall();
                    }
                }
            }
            13 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.mBall.mBallSize == 16 {
                        var3 = false;
                    } else {
                        if self.edgeCollide(var1, var2, var5) {
                            var3 = false;
                        }
                        self.addRing();
                        self.set_tm(var1, var2, 145 | var4);
                        self.set_tm(var1 + 1, var2, 146 | var4);
                        var6 = Some(self.mSoundHoop.clone());
                    }
                }
            }
            14 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.mBall.mBallSize == 16 {
                        var3 = false;
                    } else {
                        self.addRing();
                        self.set_tm(var1, var2, 146 | var4);
                        self.set_tm(var1 - 1, var2, 145 | var4);
                        var6 = Some(self.mSoundHoop.clone());
                    }
                }
            }
            15 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.mBall.mBallSize == 16 {
                        var3 = false;
                    } else {
                        if self.edgeCollide(var1, var2, var5) {
                            var3 = false;
                        }
                        self.addRing();
                        self.set_tm(var1, var2, 147 | var4);
                        self.set_tm(var1, var2 + 1, 148 | var4);
                        var6 = Some(self.mSoundHoop.clone());
                    }
                }
            }
            16 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.mBall.mBallSize == 16 {
                        var3 = false;
                    } else {
                        if self.edgeCollide(var1, var2, var5) {
                            var3 = false;
                        }
                        self.addRing();
                        self.set_tm(var1, var2, 148 | var4);
                        self.set_tm(var1, var2 - 1, 147 | var4);
                        var6 = Some(self.mSoundHoop.clone());
                    }
                }
            }
            17 | 19 | 20 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.mBall.mBallSize == 16 {
                        var3 = false;
                    } else if self.edgeCollide(var1, var2, var5) {
                        var3 = false;
                    }
                }
            }
            18 => {
                if self.thinCollide(var1, var2, var5) && self.mBall.mBallSize == 16 {
                    var3 = false;
                }
            }
            21 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.edgeCollide(var1, var2, var5) {
                        var3 = false;
                    }
                    self.addRing();
                    self.set_tm(var1, var2, 153 | var4);
                    self.set_tm(var1 + 1, var2, 154 | var4);
                    var6 = Some(self.mSoundHoop.clone());
                }
            }
            22 => {
                if self.thinCollide(var1, var2, var5) {
                    self.addRing();
                    self.set_tm(var1, var2, 154 | var4);
                    self.set_tm(var1 - 1, var2, 153 | var4);
                    var6 = Some(self.mSoundHoop.clone());
                }
            }
            23 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.edgeCollide(var1, var2, var5) {
                        var3 = false;
                    } else {
                        self.addRing();
                        self.set_tm(var1, var2, 155 | var4);
                        self.set_tm(var1, var2 + 1, 156 | var4);
                        var6 = Some(self.mSoundHoop.clone());
                    }
                }
            }
            24 => {
                if self.thinCollide(var1, var2, var5) {
                    if self.edgeCollide(var1, var2, var5) {
                        var3 = false;
                    }
                    self.addRing();
                    self.set_tm(var1, var2, 156 | var4);
                    self.set_tm(var1, var2 - 1, 155 | var4);
                    var6 = Some(self.mSoundHoop.clone());
                }
            }
            25 | 27 | 28 => {
                if self.edgeCollide(var1, var2, var5) {
                    var3 = false;
                }
            }
            29 => {
                self.add2Score(1000);
                if self.numLives < 5 {
                    self.numLives += 1;
                    self.mPaintUIFlag = true;
                }
                self.set_tm(var1, var2, 128);
                var6 = Some(self.mSoundPickup.clone());
            }
            30..=33 => {
                if self.triangleCollide(var1, var2, var5) {
                    var3 = false;
                    self.mBall.mCDRampFlag = true;
                }
            }
            34..=37 => {
                if self.triangleCollide(var1, var2, var5) {
                    self.mBall.mCDRubberFlag = true;
                    var3 = false;
                    self.mBall.mCDRampFlag = true;
                }
            }
            38 => {
                self.mBall.speedBonusCntr = 300;
                var6 = Some(self.mSoundPickup.clone());
                var3 = false;
            }
            39..=42 => {
                var3 = false;
                if self.mBall.mBallSize == 16 {
                    self.shrinkBall();
                }
            }
            43..=46 => {
                if self.thinCollide(var1, var2, var5) {
                    var3 = false;
                    if self.mBall.mBallSize == 12 {
                        self.enlargeBall();
                    }
                }
            }
            47..=50 => {
                self.mBall.gravBonusCntr = 300;
                var6 = Some(self.mSoundPickup.clone());
                var3 = false;
            }
            51..=54 => {
                self.mBall.jumpBonusCntr = 300;
                var6 = Some(self.mSoundPickup.clone());
                var3 = false;
            }
            _ => {}
        }
        if let Some(s) = var6 {
            self.play(&s);
        }
        var3
    }

    /// `Ball.update()`
    pub fn ball_update(&mut self) {
        let var1 = self.mBall.xPos;
        let mut var2: i32;
        let mut var3: i32;
        let var4: i32;
        let mut var5 = false;
        if self.mBall.ballState == 2 {
            self.mBall.xOffset = 0;
            self.mBall.popCntr -= 1;
            if self.mBall.popCntr == 0 {
                self.mBall.ballState = 1;
                if self.numLives < 0 {
                    self.mLeaveGame = true;
                }
            }
            return;
        }

        let mut var6 = self.mBall.xPos / 12;
        let mut var7 = self.mBall.yPos / 12;
        if self.mBall.xPos >= 156 {
            var6 = self.tileX + var6 - 13;
            var7 += self.tileY;
        } else if self.mBall.xPos < self.divisorLine {
            var6 += self.tileX;
            var7 += self.tileY;
        } else {
            var6 = self.divTileX - 13 - self.divisorLine / 12 + var6;
            var7 += self.divTileY;
        }

        if self.tm(var7, var6) & 64 != 0 {
            if self.mBall.mBallSize == 16 {
                var3 = -30;
                var2 = -2;
                if self.mBall.mGroundedFlag {
                    self.mBall.ySpeed = -10;
                }
            } else {
                var3 = 42;
                var2 = 6;
            }
        } else if self.mBall.mBallSize == 16 {
            var3 = 38;
            var2 = 3;
        } else {
            var3 = 80;
            var2 = 4;
        }

        if self.mBall.gravBonusCntr != 0 {
            var5 = true;
            var3 *= -1;
            var2 *= -1;
            self.mBall.gravBonusCntr -= 1;
            if self.mBall.gravBonusCntr == 0 {
                var5 = false;
                self.mBall.mGroundedFlag = false;
                var3 *= -1;
                var2 *= -1;
            }
        }

        if self.mBall.jumpBonusCntr != 0 {
            if -self.mBall.jumpOffset.abs() > -80 {
                self.mBall.jumpOffset = if var5 { 80 } else { -80 };
            }
            self.mBall.jumpBonusCntr -= 1;
        }

        self.mBall.slideCntr += 1;
        if self.mBall.slideCntr == 3 {
            self.mBall.slideCntr = 0;
        }

        let b = &mut self.mBall;
        b.ySpeed = b.ySpeed.clamp(-150, 150);
        b.xSpeed = b.xSpeed.clamp(-150, 150);

        let mut var8 = 0;
        while var8 < self.mBall.ySpeed.abs() / 10 {
            let mut var9 = 0;
            if self.mBall.ySpeed != 0 {
                var9 = if self.mBall.ySpeed < 0 { -1 } else { 1 };
            }
            let (x, y) = (self.mBall.xPos, self.mBall.yPos);
            if self.collisionDetection(x, y + var9) {
                self.mBall.yPos += var9;
                self.mBall.mGroundedFlag = false;
                if var3 == -30 {
                    var7 = self.tileY + self.mBall.yPos / 12;
                    if self.tm(var7, var6) & 64 == 0 {
                        self.mBall.ySpeed >>= 1;
                        if self.mBall.ySpeed <= 10 && self.mBall.ySpeed >= -10 {
                            self.mBall.ySpeed = 0;
                        }
                    }
                }
            } else {
                if self.mBall.mCDRampFlag && self.mBall.xSpeed < 10 && self.mBall.slideCntr == 0 {
                    let var10 = 1;
                    let (x, y) = (self.mBall.xPos, self.mBall.yPos);
                    if self.collisionDetection(x + var10, y + var9) {
                        self.mBall.xPos += var10;
                        self.mBall.yPos += var9;
                        self.mBall.mCDRampFlag = false;
                    } else if self.collisionDetection(x - var10, y + var9) {
                        self.mBall.xPos -= var10;
                        self.mBall.yPos += var9;
                        self.mBall.mCDRampFlag = false;
                    }
                }

                let b = &mut self.mBall;
                if var9 > 0 || var5 && var9 < 0 {
                    b.ySpeed = b.ySpeed * -1 / 2;
                    b.mGroundedFlag = true;
                    if b.mCDRubberFlag && (b.direction & 8) != 0 {
                        b.mCDRubberFlag = false;
                        if var5 {
                            b.jumpOffset += 10;
                        } else {
                            b.jumpOffset += -10;
                        }
                    } else if b.jumpBonusCntr == 0 {
                        b.jumpOffset = 0;
                    }
                    if b.ySpeed < 10 && b.ySpeed > -10 {
                        b.ySpeed = if var5 { -10 } else { 10 };
                    }
                    break;
                }
                if var9 < 0 || var5 && var9 > 0 {
                    b.ySpeed = if var5 { -20 } else { 20 };
                }
            }
            var8 += 1;
        }

        let b = &mut self.mBall;
        if var5 {
            if var2 == -2 && b.ySpeed < var3 {
                b.ySpeed += var2;
                if b.ySpeed > var3 {
                    b.ySpeed = var3;
                }
            } else if !b.mGroundedFlag && b.ySpeed > var3 {
                b.ySpeed += var2;
                if b.ySpeed < var3 {
                    b.ySpeed = var3;
                }
            }
        } else if var2 == -2 && b.ySpeed > var3 {
            b.ySpeed += var2;
            if b.ySpeed < var3 {
                b.ySpeed = var3;
            }
        } else if !b.mGroundedFlag && b.ySpeed < var3 {
            b.ySpeed += var2;
            if b.ySpeed > var3 {
                b.ySpeed = var3;
            }
        }

        if b.speedBonusCntr != 0 {
            var4 = 100;
            b.speedBonusCntr -= 1;
        } else {
            var4 = 50;
        }

        if (b.direction & 2) != 0 && b.xSpeed < var4 {
            b.xSpeed += 6;
        } else if (b.direction & 1) != 0 && b.xSpeed > -var4 {
            b.xSpeed -= 6;
        } else if b.xSpeed > 0 {
            b.xSpeed -= 4;
        } else if b.xSpeed < 0 {
            b.xSpeed += 4;
        }

        if b.mBallSize == 16 && b.jumpBonusCntr == 0 {
            if var5 {
                b.jumpOffset += 5;
            } else {
                b.jumpOffset += -5;
            }
        }

        if b.mGroundedFlag && (b.direction & 8) != 0 {
            if var5 {
                b.ySpeed = 67 + b.jumpOffset;
            } else {
                b.ySpeed = -67 + b.jumpOffset;
            }
            b.mGroundedFlag = false;
        }

        let var20 = b.xSpeed.abs();
        let var21 = var20 / 10;
        for _ in 0..var21 {
            let mut var12 = 0;
            if self.mBall.xSpeed != 0 {
                var12 = if self.mBall.xSpeed < 0 { -1 } else { 1 };
            }
            let (x, y) = (self.mBall.xPos, self.mBall.yPos);
            if self.collisionDetection(x + var12, y) {
                self.mBall.xPos += var12;
            } else if self.mBall.mCDRampFlag {
                self.mBall.mCDRampFlag = false;
                let var13 = if var5 { 1 } else { -1 };
                let (x, y) = (self.mBall.xPos, self.mBall.yPos);
                if self.collisionDetection(x + var12, y + var13) {
                    self.mBall.xPos += var12;
                    self.mBall.yPos += var13;
                } else if self.collisionDetection(x + var12, y - var13) {
                    self.mBall.xPos += var12;
                    self.mBall.yPos -= var13;
                } else {
                    self.mBall.xSpeed = -(self.mBall.xSpeed >> 1);
                }
            }
        }

        let b = &mut self.mBall;
        b.xOffset = b.xPos - var1;
        if b.xPos > 156 + b.mBallSize {
            b.xPos -= 156;
            if self.scrollOffset - 10 > 156 + b.mBallSize {
                self.scrollOffset -= 156;
            }
        }
        if b.xPos - b.mBallSize < 0 {
            b.xPos += 156;
            if self.scrollOffset - b.mBallSize < 10 {
                self.scrollOffset += 156;
            }
        }
    }

    /// `Ball.paint(Graphics)` into the game buffer.
    pub fn ball_paint(&mut self) {
        let b = self.mBall.clone();
        if b.ballState == 2 {
            let img = self.tileImages[POPPED_IMAGE].clone();
            self.mGameBuffer.draw_image(&img, b.xPos - 6, b.yPos - 6, AT);
            if b.xPos > 144 {
                self.mGameBuffer.draw_image(&img, b.xPos - 156 - 6, b.yPos - 6, AT);
            }
        } else {
            let img = self.tileImages[b.mBallImage].clone();
            self.mGameBuffer.draw_image(&img, b.xPos - b.mHalfBallSize, b.yPos - b.mHalfBallSize, AT);
            if b.xPos > 156 - b.mBallSize {
                self.mGameBuffer.draw_image(&img, b.xPos - 156 - b.mHalfBallSize, b.yPos - b.mHalfBallSize, AT);
            }
        }
        self.dirtyTiles();
    }

    pub fn dirtyTiles(&mut self) {
        let b = &self.mBall;
        let var1 = (b.xPos - b.mHalfBallSize) / 12;
        let mut var2 = (b.yPos - b.mHalfBallSize) / 12;
        let var3 = (b.xPos - 1 + b.mHalfBallSize) / 12 + 1;
        let mut var4 = (b.yPos - 1 + b.mHalfBallSize) / 12 + 1;
        if var2 < 0 {
            var2 = 0;
        }
        if var4 > 8 {
            var4 = 8;
        }
        for var7 in var1..var3 {
            for var8 in var2..var4 {
                let (var5, var6);
                if var7 * 12 >= 156 {
                    var5 = self.tileX + var7 - 13;
                    var6 = self.tileY + var8;
                } else if self.mBall.xPos < self.divisorLine {
                    var5 = self.tileX + var7;
                    var6 = self.tileY + var8;
                } else {
                    var5 = self.divTileX - 13 - self.divisorLine / 12 + var7;
                    var6 = self.divTileY + var8;
                }
                let v = self.tm(var6, var5) | 128;
                self.set_tm(var6, var5, v);
            }
        }
    }
}
