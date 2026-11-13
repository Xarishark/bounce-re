//! `TileCanvas`: the level map, its tile images, and the 156x96 game buffer
//! that scrolls horizontally as a ring (13 tile columns, the `divisorLine`
//! marks where newly drawn columns wrap around).

use super::Game;
use crate::midp::assets;
use crate::midp::gfx::{Image, Img, Surface, FLIP_HORIZONTAL, FLIP_VERTICAL, LEFT, TOP};
use crate::midp::lang;
use crate::midp::rms::DataIn;
use std::rc::Rc;

const AT: i32 = TOP | LEFT; // anchor 20
pub const BACKGROUND_COLOUR: i32 = 11591920;
pub const WATER_COLOUR: i32 = 1073328;

impl Game {
    /// Constructor part of `TileCanvas(Display)`.
    pub(super) fn TileCanvas(&mut self) {
        self.scrollFlag = true;
        self.divisorLine = 156;
        self.rightDrawEdge = 142;
        self.leftDrawEdge = 14;
        self.mGameBuffer = Surface::new(156, 96, self.font.clone());
        self.tmpTileImage = Surface::new(12, 12, self.font.clone());
        self.loadTileImages();
        self.mLoadLevelFlag = false;
        self.tileX = 0;
        self.tileY = 0;
        self.mExitPos = -1;
        self.divTileX = self.tileX + 13;
        self.divTileY = self.tileY;
        self.tileMap = None;
        self.hoopList.clear();
    }

    pub fn loadLevel(&mut self, var1: i32) {
        self.mLoadLevelFlag = false;
        let n = self.mLevelNum.to_string();
        self.mLevelNumStr = lang::s1(14, &n);
        self.mLevelCompletedStr = lang::s1(15, &n);
        let var4 = if var1 < 10 {
            format!("00{var1}")
        } else if var1 < 100 {
            format!("0{var1}")
        } else {
            String::new()
        };
        let Some(data) = assets::get(&format!("/levels/J2MElvl.{var4}")) else {
            // getResourceAsStream returned null: Java throws a NullPointerException here
            crate::log!("missing level {var1}");
            return;
        };
        let mut var3 = DataIn::new(data);
        self.mStartCol = var3.read();
        self.mStartRow = var3.read();
        let var6 = var3.read();
        self.mStartBallSize = if var6 == 0 { 12 } else { 16 };
        self.mExitPosX = var3.read();
        self.mExitPosY = var3.read();
        let exit = self.tileImages[12].clone();
        self.createExitTileObject(self.mExitPosX, self.mExitPosY, exit);
        self.mTotalNumRings = var3.read();
        self.mTileMapWidth = var3.read();
        self.mTileMapHeight = var3.read();
        let mut map = vec![vec![0i32; self.mTileMapWidth.max(0) as usize]; self.mTileMapHeight.max(0) as usize];
        for row in map.iter_mut() {
            for cell in row.iter_mut() {
                *cell = var3.read() as i16 as i32;
            }
        }
        self.tileMap = Some(map);
        self.mNumMoveObj = var3.read();
        if self.mNumMoveObj != 0 {
            self.createMovingObj(&mut var3);
        }
    }

    /// `manipulateImage(img, mode)`: flips and rotations into a new
    /// transparent image of the same size.
    pub fn manipulateImage(&self, var0: &Image, var1: i32) -> Img {
        let mut s = Surface::with_fill(var0.w, var0.h, 0, self.font.clone());
        let m = match var1 {
            0 => FLIP_HORIZONTAL,
            1 => FLIP_VERTICAL,
            2 => FLIP_HORIZONTAL | FLIP_VERTICAL,
            3 => 90,
            4 => 180,
            5 => 270,
            _ => 0,
        };
        s.draw_image_m(var0, 0, 0, AT, m);
        s.into_img()
    }

    fn createMovingObj(&mut self, var1: &mut DataIn) {
        let n = self.mNumMoveObj.max(0) as usize;
        self.mMOTopLeft = vec![[0; 2]; n];
        self.mMOBotRight = vec![[0; 2]; n];
        self.mMODirection = vec![[0; 2]; n];
        self.mMOOffset = vec![[0; 2]; n];
        for i in 0..n {
            self.mMOTopLeft[i][0] = var1.read();
            self.mMOTopLeft[i][1] = var1.read();
            self.mMOBotRight[i][0] = var1.read();
            self.mMOBotRight[i][1] = var1.read();
            self.mMODirection[i][0] = var1.read();
            self.mMODirection[i][1] = var1.read();
            let var2 = var1.read();
            let var3 = var1.read();
            self.mMOOffset[i][0] = var2;
            self.mMOOffset[i][1] = var3;
        }
        let mut var5 = Surface::new(24, 24, self.font.clone());
        let spike = self.tileImages[46].clone();
        var5.draw_image(&spike, 0, 0, AT);
        var5.draw_image(&self.manipulateImage(&spike, 0), 12, 0, AT);
        var5.draw_image(&self.manipulateImage(&spike, 4), 12, 12, AT);
        var5.draw_image(&self.manipulateImage(&spike, 1), 0, 12, AT);
        self.mSpikeImgPtr = Some(var5.into_img());
    }

    pub fn disposeLevel(&mut self) {
        self.tileMap = None;
    }

    pub fn updateMovingSpikeObj(&mut self) {
        for i in 0..self.mNumMoveObj.max(0) as usize {
            let var2 = self.mMOTopLeft[i][0];
            let var3 = self.mMOTopLeft[i][1];
            let mut var4 = self.mMOOffset[i][0];
            let mut var5 = self.mMOOffset[i][1];
            self.mMOOffset[i][0] = (self.mMOOffset[i][0] + self.mMODirection[i][0]) as i16 as i32;
            let var8 = (self.mMOBotRight[i][0] - var2 - 2) * 12;
            let var9 = (self.mMOBotRight[i][1] - var3 - 2) * 12;
            if self.mMOOffset[i][0] < 0 {
                self.mMOOffset[i][0] = 0;
            } else if self.mMOOffset[i][0] > var8 {
                self.mMOOffset[i][0] = var8 as i16 as i32;
            }
            if self.mMOOffset[i][0] == 0 || self.mMOOffset[i][0] == var8 {
                self.mMODirection[i][0] = -self.mMODirection[i][0];
            }

            self.mMOOffset[i][1] = (self.mMOOffset[i][1] + self.mMODirection[i][1]) as i16 as i32;
            if self.mMOOffset[i][1] < 0 {
                self.mMOOffset[i][1] = 0;
            } else if self.mMOOffset[i][1] > var9 {
                self.mMOOffset[i][1] = var9 as i16 as i32;
            }
            if self.mMOOffset[i][1] == 0 || self.mMOOffset[i][1] == var9 {
                self.mMODirection[i][1] *= -1;
            }

            let mut var6 = self.mMOOffset[i][0];
            let mut var7 = self.mMOOffset[i][1];
            if var6 < var4 {
                std::mem::swap(&mut var6, &mut var4);
            }
            if var7 < var5 {
                std::mem::swap(&mut var7, &mut var5);
            }
            var6 += 23;
            var7 += 23;
            var4 /= 12;
            var5 /= 12;
            var6 = var6 / 12 + 1;
            var7 = var7 / 12 + 1;
            for var19 in var4..var6 {
                for var11 in var5..var7 {
                    let v = self.tm(var3 + var11, var2 + var19) | 128;
                    self.set_tm(var3 + var11, var2 + var19, v);
                }
            }
        }
    }

    pub fn findSpikeIndex(&self, var1: i32, var2: i32) -> i32 {
        for i in 0..self.mNumMoveObj.max(0) as usize {
            if self.mMOTopLeft[i][0] <= var1 && self.mMOBotRight[i][0] > var1 && self.mMOTopLeft[i][1] <= var2 && self.mMOBotRight[i][1] > var2 {
                return i as i32;
            }
        }
        -1
    }

    pub fn drawTile(&mut self, var1: i32, var2: i32, var3: i32, var4: i32) {
        if self.tm(var2, var1) & 128 != 0 {
            let v = self.tm(var2, var1) & 0xFF7F;
            self.set_tm(var2, var1, v);
        }
        let mut var5 = self.tm(var2, var1);
        let var6 = (var5 & 64) != 0;
        if var6 {
            var5 &= -65;
        }
        let ti = |i: usize| self.tileImages[i].clone();
        let g = &mut self.mGameBuffer;
        g.set_color(if var6 { WATER_COLOUR } else { BACKGROUND_COLOUR });
        // the hoop tiles: background, back half now, front half after the ball
        let hoop = |g: &mut Surface, back: Img, front: Img, list: &mut Vec<(Img, i32, i32)>| {
            g.fill_rect(var3, var4, 12, 12);
            g.draw_image(&back, var3, var4, AT);
            list.push((front, var3, var4));
        };
        match var5 {
            0 => g.fill_rect(var3, var4, 12, 12),
            1 => g.draw_image(&ti(0), var3, var4, AT),
            2 => g.draw_image(&ti(1), var3, var4, AT),
            3 => g.draw_image(&ti(if var6 { 6 } else { 2 }), var3, var4, AT),
            4 => g.draw_image(&ti(if var6 { 9 } else { 5 }), var3, var4, AT),
            5 => g.draw_image(&ti(if var6 { 7 } else { 3 }), var3, var4, AT),
            6 => g.draw_image(&ti(if var6 { 8 } else { 4 }), var3, var4, AT),
            7 => g.draw_image(&ti(10), var3, var4, AT),
            8 => g.draw_image(&ti(11), var3, var4, AT),
            9 => {
                let var12 = (var1 - self.mTopLeftExitTileCol) * 12;
                let var13 = (var2 - self.mTopLeftExitTileRow) * 12;
                g.set_clip(var3, var4, 12, 12);
                g.draw_image(&self.mExitTileImage.img, var3 - var12, var4 - var13, AT);
                g.set_clip(0, 0, 156, 96);
                self.mExitPos = var3 - var12 + 12 - 1;
            }
            10 => {
                let var7 = self.findSpikeIndex(var1, var2);
                if var7 != -1 {
                    let i = var7 as usize;
                    let var8 = (var1 - self.mMOTopLeft[i][0]) * 12;
                    let var9 = (var2 - self.mMOTopLeft[i][1]) * 12;
                    let var10 = self.mMOOffset[i][0] - var8;
                    let var11 = self.mMOOffset[i][1] - var9;
                    let g = &mut self.mGameBuffer;
                    if (var10 <= -36 || var10 >= 12) && (var11 <= -36 || var11 >= 12) {
                        g.set_color(BACKGROUND_COLOUR);
                        g.fill_rect(var3, var4, 12, 12);
                    } else {
                        let t = &mut self.tmpTileImage;
                        t.set_color(BACKGROUND_COLOUR);
                        t.fill_rect(0, 0, 12, 12);
                        if let Some(sp) = &self.mSpikeImgPtr {
                            t.draw_image(sp, var10, var11, AT);
                        }
                        g.draw_image(&t.img, var3, var4, AT);
                    }
                }
            }
            13 => hoop(g, ti(35), ti(33), &mut self.hoopList),
            14 => hoop(g, ti(36), ti(34), &mut self.hoopList),
            15 => hoop(g, ti(17), ti(18), &mut self.hoopList),
            16 => hoop(g, ti(19), ti(20), &mut self.hoopList),
            17 => hoop(g, ti(43), ti(41), &mut self.hoopList),
            18 => hoop(g, ti(44), ti(42), &mut self.hoopList),
            19 => hoop(g, ti(25), ti(26), &mut self.hoopList),
            20 => hoop(g, ti(27), ti(28), &mut self.hoopList),
            21 => hoop(g, ti(31), ti(29), &mut self.hoopList),
            22 => hoop(g, ti(32), ti(30), &mut self.hoopList),
            23 => hoop(g, ti(13), ti(14), &mut self.hoopList),
            24 => hoop(g, ti(15), ti(16), &mut self.hoopList),
            25 => hoop(g, ti(39), ti(37), &mut self.hoopList),
            26 => hoop(g, ti(40), ti(38), &mut self.hoopList),
            27 => hoop(g, ti(21), ti(22), &mut self.hoopList),
            28 => hoop(g, ti(23), ti(24), &mut self.hoopList),
            29 => g.draw_image(&ti(45), var3, var4, AT),
            30 => g.draw_image(&ti(if var6 { 61 } else { 57 }), var3, var4, AT),
            31 => g.draw_image(&ti(if var6 { 60 } else { 56 }), var3, var4, AT),
            32 => g.draw_image(&ti(if var6 { 59 } else { 55 }), var3, var4, AT),
            33 => g.draw_image(&ti(if var6 { 62 } else { 58 }), var3, var4, AT),
            34 | 35 | 36 | 37 => {
                let i = [65, 64, 63, 66][(var5 - 34) as usize];
                g.fill_rect(var3, var4, 12, 12);
                g.draw_image(&ti(i), var3, var4, AT);
            }
            38 => g.draw_image(&ti(53), var3, var4, AT),
            // deflators, inflators: on the background; gravity and jump: drawn as is
            39..=54 => {
                let (base, plain) = match var5 {
                    39..=42 => (50, false),
                    43..=46 => (51, false),
                    47..=50 => (52, true),
                    _ => (54, true),
                };
                // floor, then left wall / ceiling / right wall = rotate 270 / 180 / 90
                let rot = [-1, 5, 4, 3][((var5 - 39) % 4) as usize];
                let img = ti(base);
                let img = if rot < 0 { img } else { self.manipulateImage(&img, rot) };
                let g = &mut self.mGameBuffer;
                if !plain {
                    g.fill_rect(var3, var4, 12, 12);
                }
                g.draw_image(&img, var3, var4, AT);
            }
            _ => {}
        }
    }

    pub fn createNewBuffer(&mut self) {
        for var1 in 0..13 {
            for var2 in 0..8 {
                self.drawTile(self.tileX + var1, self.tileY + var2, var1 * 12, var2 * 12);
            }
        }
    }

    pub fn cleanBuffer(&mut self, var1: bool) {
        let mut var2 = self.tileX;
        let mut var3 = self.tileY;
        for var4 in 0..13 {
            if var4 * 12 >= self.divisorLine && var2 >= self.tileX {
                var2 = self.divTileX - 13;
            }
            for var5 in 0..8 {
                if self.tm(var3, var2) & 128 != 0 {
                    let v = self.tm(var3, var2) & 0xFF7F;
                    self.set_tm(var3, var2, v);
                    if var1 {
                        self.drawTile(var2, var3, var4 * 12, var5 * 12);
                    }
                }
                var3 += 1;
            }
            var3 = self.tileY;
            var2 += 1;
        }
    }

    pub fn scrollBuffer(&mut self, _var1: i32, var2: i32, var3: i32) {
        if self.rightDrawEdge < 0 {
            self.rightDrawEdge += 156;
        }

        if self.rightDrawEdge > self.divisorLine && self.rightDrawEdge <= self.divisorLine + 12 {
            if self.tileX + self.divisorLine / 12 >= self.mTileMapWidth {
                self.leftDrawEdge -= var2;
                self.rightDrawEdge -= var2;
                if self.rightDrawEdge < 0 {
                    self.rightDrawEdge += 156;
                }
                if self.scrollFlag {
                    self.scrollFlag = false;
                    self.scrollOffset = self.rightDrawEdge - 64;
                    if self.scrollOffset < var3 {
                        self.scrollOffset += 156;
                    }
                }
            } else {
                if self.divisorLine >= 156 {
                    self.divisorLine = 0;
                    self.tileX += 13;
                }
                if self.rightDrawEdge >= 156 {
                    self.rightDrawEdge -= 156;
                }
                let var4 = self.divisorLine;
                self.divisorLine += 12;
                self.divTileX += 1;
                for var5 in 0..8 {
                    self.drawTile(self.tileX + var4 / 12, self.tileY + var5, var4, var5 * 12);
                }
            }
        } else if self.rightDrawEdge > 156 {
            self.rightDrawEdge -= 156;
        }

        if self.leftDrawEdge >= 156 {
            self.leftDrawEdge -= 156;
        }
        if self.leftDrawEdge < 0 {
            self.leftDrawEdge += 156;
        }

        if self.leftDrawEdge < self.divisorLine && self.leftDrawEdge >= self.divisorLine - 12 {
            if self.tileX - (13 - self.divisorLine / 12) <= 0 {
                self.leftDrawEdge -= var2;
                self.rightDrawEdge -= var2;
                if self.leftDrawEdge >= 156 {
                    self.leftDrawEdge -= 156;
                }
                if self.scrollFlag {
                    self.scrollFlag = false;
                    self.scrollOffset = (self.leftDrawEdge + 64) % 156;
                    if self.scrollOffset < var3 {
                        self.scrollOffset += 156;
                    }
                }
            } else {
                self.divisorLine -= 12;
                let var6 = self.divisorLine;
                self.divTileX -= 1;
                if self.divisorLine <= 0 {
                    self.divisorLine = 156;
                    self.tileX -= 13;
                }
                for var7 in 0..8 {
                    self.drawTile(self.divTileX - 13, self.divTileY + var7, var6, var7 * 12);
                }
            }
        }
    }

    pub fn testScroll(&mut self, var1: i32, mut var2: i32) {
        if !self.scrollFlag {
            if self.tileX - (13 - self.divisorLine / 12) <= 0 && var1 >= self.scrollOffset && var1 < self.scrollOffset + 10 {
                self.scrollFlag = true;
                var2 = var1 - self.scrollOffset;
            }
            if self.tileX + self.divisorLine / 12 >= self.mTileMapWidth && var1 <= self.scrollOffset && var1 > self.scrollOffset - 10 {
                self.scrollFlag = true;
                var2 = var1 - self.scrollOffset;
            }
        }
        if self.scrollFlag {
            self.leftDrawEdge += var2;
            self.rightDrawEdge += var2;
        }
    }

    fn createLargeBallImage(&self, var1: &Image) -> Img {
        let mut s = Surface::with_fill(16, 16, 0, self.font.clone());
        s.draw_image(var1, -4, -4, AT);
        s.draw_image_m(var1, 8, -4, AT, FLIP_HORIZONTAL);
        s.draw_image_m(var1, -4, 8, AT, FLIP_VERTICAL);
        s.draw_image_m(var1, 8, 8, AT, 180);
        s.into_img()
    }

    fn createExitImage(&self, var1: &Image) -> Img {
        let mut var3 = Surface::new(24, 48, self.font.clone());
        var3.set_color(BACKGROUND_COLOUR);
        var3.fill_rect(0, 0, 24, 48);
        var3.set_color(16555422);
        var3.fill_rect(4, 0, 16, 48);
        var3.set_color(14891583);
        var3.fill_rect(6, 0, 10, 48);
        var3.set_color(12747918);
        var3.fill_rect(10, 0, 4, 48);
        var3.draw_image(var1, 0, 0, AT);
        var3.draw_image(&self.manipulateImage(var1, 0), 12, 0, AT);
        var3.draw_image(&self.manipulateImage(var1, 1), 0, 12, AT);
        var3.draw_image(&self.manipulateImage(var1, 2), 12, 12, AT);
        var3.into_img()
    }

    fn loadTileImages(&mut self) {
        let var1 = assets::image("/icons/objects_nm.png").unwrap_or_else(|| Rc::new(Image::filled(48, 72, 0)));
        let x = |c, r| self.extractImage(&var1, c, r);
        let xb = |c, r, bg| self.extractImageBG(&var1, c, r, bg);
        let blank: Img = Rc::new(Image::filled(12, 12, 0));
        let mut t: Vec<Img> = vec![blank; 67];
        let m = |t: &Vec<Img>, i: usize, mode: i32| self.manipulateImage(&t[i], mode);
        t[0] = x(1, 0);
        t[1] = x(1, 2);
        t[2] = xb(0, 3, -5185296);
        t[3] = m(&t, 2, 1);
        t[4] = m(&t, 2, 3);
        t[5] = m(&t, 2, 5);
        t[6] = xb(0, 3, -15703888);
        t[7] = m(&t, 6, 1);
        t[8] = m(&t, 6, 3);
        t[9] = m(&t, 6, 5);
        t[10] = x(0, 4);
        t[11] = x(3, 4);
        t[12] = self.createExitImage(&x(2, 3));
        t[14] = x(0, 5);
        t[13] = m(&t, 14, 1);
        t[15] = m(&t, 13, 0);
        t[16] = m(&t, 14, 0);
        t[18] = x(1, 5);
        t[17] = m(&t, 18, 1);
        t[19] = m(&t, 17, 0);
        t[20] = m(&t, 18, 0);
        t[22] = x(2, 5);
        t[21] = m(&t, 22, 1);
        t[23] = m(&t, 21, 0);
        t[24] = m(&t, 22, 0);
        t[26] = x(3, 5);
        t[25] = m(&t, 26, 1);
        t[27] = m(&t, 25, 0);
        t[28] = m(&t, 26, 0);
        t[29] = m(&t, 14, 5);
        t[30] = m(&t, 29, 1);
        t[31] = m(&t, 29, 0);
        t[32] = m(&t, 30, 0);
        t[33] = m(&t, 18, 5);
        t[34] = m(&t, 33, 1);
        t[35] = m(&t, 33, 0);
        t[36] = m(&t, 34, 0);
        t[37] = m(&t, 22, 5);
        t[38] = m(&t, 37, 1);
        t[39] = m(&t, 37, 0);
        t[40] = m(&t, 38, 0);
        t[41] = m(&t, 26, 5);
        t[42] = m(&t, 41, 1);
        t[43] = m(&t, 41, 0);
        t[44] = m(&t, 42, 0);
        t[45] = x(3, 3);
        t[46] = x(1, 3);
        t[47] = x(2, 0);
        t[48] = x(0, 1);
        t[49] = self.createLargeBallImage(&x(3, 0));
        t[50] = x(3, 1);
        t[51] = x(2, 4);
        t[52] = x(3, 2);
        t[53] = x(1, 1);
        t[54] = x(2, 2);
        t[55] = xb(0, 0, -5185296);
        t[56] = m(&t, 55, 3);
        t[57] = m(&t, 55, 4);
        t[58] = m(&t, 55, 5);
        t[59] = xb(0, 0, -15703888);
        t[60] = m(&t, 59, 3);
        t[61] = m(&t, 59, 4);
        t[62] = m(&t, 59, 5);
        t[63] = x(0, 2);
        t[64] = m(&t, 63, 3);
        t[65] = m(&t, 63, 4);
        t[66] = m(&t, 63, 5);
        let (life, ring) = (x(2, 1), x(1, 4));
        self.mUILife = life;
        self.mUIRing = ring;
        self.tileImages = t;
    }

    pub fn extractImage(&self, var0: &Image, var1: i32, var2: i32) -> Img {
        self.extractImageBG(var0, var1, var2, 0)
    }

    /// `DirectUtils.createImage(12, 12, bg)` with one 12x12 cell of the sheet drawn on it.
    pub fn extractImageBG(&self, var0: &Image, var1: i32, var2: i32, var3: i32) -> Img {
        let mut s = Surface::with_fill(12, 12, var3 as u32, self.font.clone());
        s.draw_image(var0, -var1 * 12, -var2 * 12, AT);
        s.into_img()
    }

    pub fn createExitTileObject(&mut self, var1: i32, var2: i32, var3: Img) {
        self.mTopLeftExitTileCol = var1;
        self.mTopLeftExitTileRow = var2;
        self.mImgPtr = var3;
        self.mExitTileImage = Surface::new(24, 24, self.font.clone());
        self.mImageOffset = 0;
        self.repaintExitTile();
        self.mOpenFlag = false;
    }

    pub fn repaintExitTile(&mut self) {
        let img = self.mImgPtr.clone();
        self.mExitTileImage.draw_image(&img, 0, -self.mImageOffset, AT);
    }

    pub fn openExit(&mut self) {
        self.mImageOffset += 4;
        if self.mImageOffset >= 24 {
            self.mImageOffset = 24;
            self.mOpenFlag = true;
        }
        self.repaintExitTile();
    }

    #[allow(clippy::too_many_arguments)]
    pub fn rectCollide(var0: i32, var1: i32, var2: i32, var3: i32, var4: i32, var5: i32, var6: i32, var7: i32) -> bool {
        var0 <= var6 && var1 <= var7 && var4 <= var2 && var5 <= var3
    }
}
