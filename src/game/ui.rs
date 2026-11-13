//! `BounceUI` (menus, high score, saved game in RMS) and `Bounce` (the MIDlet).

use super::{Current, Game};
use crate::midp::lang;
use crate::midp::lcdui::{Cmd, Screen, Tag};
use crate::midp::rms::{DataIn, DataOut, RecordStore};

const MAIN_MENU_CONTINUE: usize = 0;
const MAIN_MENU_NEW_GAME: usize = 1;
const MAIN_MENU_HIGH_SCORE: usize = 2;
const MAIN_MENU_INSTRUCTIONS: usize = 3;
const RMS_NAME: &str = "bounceRMS";

impl Game {
    /// `new BounceUI(midlet)`
    pub(super) fn BounceUI(&mut self) {
        self.loadGameData();
        self.BounceCanvas();
        self.start();
        self.show_canvas();
        self.mMainMenuItems = [lang::s(8), lang::s(16), lang::s(12), lang::s(13)];
    }

    pub fn displayMainMenu(&mut self) {
        let mut items = Vec::new();
        if self.mState == 1 || self.mSavedValid == -6 {
            items.push(self.mMainMenuItems[MAIN_MENU_CONTINUE].clone());
        }
        for it in &self.mMainMenuItems[1..] {
            items.push(it.clone());
        }
        let mut menu = Screen::list(Tag::MainMenu, lang::s(10), items);
        menu.add_command(Cmd::Back);
        if self.mSplashIndex != -1 {
            self.mSplashIndex = -1;
            self.mSplashImage = None;
        }
        menu.set_selected_index(self.mSavedMenuItem);
        self.stop();
        self.show_screen(menu);
    }

    pub fn displayNewGameMenu(&mut self) {
        let var1: Vec<String> = (0..self.mBestLevel).map(|i| lang::s1(14, &(i + 1).to_string())).collect();
        let mut m = Screen::list(Tag::NewGameMenu, lang::s(16), var1);
        m.add_command(Cmd::Back);
        self.show_screen(m);
    }

    pub fn displayGame(&mut self, var1: bool, var2: i32) {
        self.show_canvas();
        if var1 {
            self.resetGame(var2);
        }
        self.start();
        self.mState = 1;
    }

    pub fn displayHighScore(&mut self) {
        let mut p = Screen::form(lang::s(12));
        p.append(self.mBestScore.to_string());
        p.add_command(Cmd::Back);
        self.show_screen(p);
    }

    pub fn displayInstructions(&mut self) {
        let mut p = Screen::form(lang::s(13));
        for i in 0..6 {
            p.append(lang::s(i));
        }
        p.add_command(Cmd::Back);
        self.show_screen(p);
    }

    pub fn displayGameOver(&mut self) {
        self.stop();
        let mut p = Screen::form(lang::s(11));
        p.append(lang::s(11));
        p.append("\n\n");
        if self.mNewBestScore {
            p.append(lang::s(17));
            p.append("\n\n");
        }
        p.append(self.mLastScore.to_string());
        p.add_command(Cmd::Okay);
        self.show_screen(p);
    }

    pub fn displayLevelComplete(&mut self) {
        self.stop();
        let mut p = Screen::form(String::new());
        p.append(self.mLevelCompletedStr.clone());
        p.append("\n\n");
        p.append(format!("{}\n", self.mLastScore));
        p.add_command(Cmd::Continue);
        self.show_screen(p);
    }

    pub fn commandAction(&mut self, var1: Cmd) {
        let tag = self.current_tag();
        match var1 {
            Cmd::Select => {
                let Current::Screen(s) = &self.current else { return };
                let sel = s.selected_index();
                if tag == Some(Tag::NewGameMenu) {
                    self.displayGame(true, sel + 1);
                    return;
                }
                let var3 = s.get_string(sel);
                self.mSavedMenuItem = sel;
                if var3 == self.mMainMenuItems[MAIN_MENU_CONTINUE] {
                    if self.mState == 1 {
                        self.displayGame(false, self.mLevelNum);
                    } else if self.mSavedValid == -6 {
                        self.show_canvas();
                        self.resetGameSaved(
                            self.mSavedGlobalBallX + (self.mSavedSize >> 1),
                            self.mSavedGlobalBallY + (self.mSavedSize >> 1),
                        );
                        self.mSavedTiles.clear();
                        self.mSavedTileCount = 0;
                        self.start();
                        self.mState = 1;
                    }
                } else if var3 == self.mMainMenuItems[MAIN_MENU_NEW_GAME] {
                    if self.mState != 4 {
                        if self.mBestLevel > 1 {
                            self.displayNewGameMenu();
                        } else {
                            self.mState = 4;
                            self.mNewBestScore = false;
                            self.displayGame(true, 1);
                        }
                    }
                } else if var3 == self.mMainMenuItems[MAIN_MENU_HIGH_SCORE] {
                    self.displayHighScore();
                } else if var3 == self.mMainMenuItems[MAIN_MENU_INSTRUCTIONS] {
                    self.displayInstructions();
                }
            }
            Cmd::Back | Cmd::Okay => {
                if tag == Some(Tag::MainMenu) {
                    self.destroyApp();
                    self.quit = true; // notifyDestroyed
                } else {
                    self.displayMainMenu();
                }
            }
            Cmd::Continue => self.displayGame(false, 0),
        }
    }

    pub fn loadGameData(&mut self) {
        let mut var4 = RecordStore::open(RMS_NAME);
        if var4.get_num_records() != 3 {
            var4.add_record(&[0; 1]);
            var4.add_record(&[0; 4]);
            var4.add_record(&[0; 255]);
            return;
        }
        let r1 = var4.get_record(1).unwrap_or_default();
        let r2 = var4.get_record(2).unwrap_or_default();
        let r3 = var4.get_record(3).unwrap_or_default();
        // each read stops at the first missing byte, like the EOFException the original swallowed
        let _ = (|| -> Option<()> {
            self.mBestLevel = DataIn::new(&r1).read_byte()?;
            self.mBestScore = DataIn::new(&r2).read_int()?;
            let mut d = DataIn::new(&r3);
            self.mSavedValid = d.read_byte()?;
            self.mSavedLives = d.read_byte()?;
            self.mSavedRings = d.read_byte()?;
            self.mSavedLevel = d.read_byte()?;
            self.mSavedSize = d.read_byte()?;
            self.mSavedScore = d.read_int()?;
            self.mSavedTileX = d.read_int()?;
            self.mSavedTileY = d.read_int()?;
            self.mSavedGlobalBallX = d.read_int()?;
            self.mSavedGlobalBallY = d.read_int()?;
            self.mSavedXSpeed = d.read_int()?;
            self.mSavedYSpeed = d.read_int()?;
            self.mSavedXPos = d.read_int()?;
            self.mSavedYPos = d.read_int()?;
            self.mSavedRespawnX = d.read_int()?;
            self.mSavedRespawnY = d.read_int()?;
            self.mSavedSpeedBonus = d.read_int()?;
            self.mSavedGravBonus = d.read_int()?;
            self.mSavedJumpBonus = d.read_int()?;
            self.mSavedTileCount = d.read_byte()?;
            self.mSavedTiles = vec![[0; 3]; self.mSavedTileCount.max(0) as usize];
            for i in 0..self.mSavedTiles.len() {
                self.mSavedTiles[i][0] = d.read_short()?;
                self.mSavedTiles[i][1] = d.read_short()?;
                self.mSavedTiles[i][2] = d.read_byte()?;
            }
            Some(())
        })();
    }

    pub fn saveGameData(&mut self, var1: i32) {
        let mut var3 = DataOut::default();
        match var1 {
            1 => var3.write_byte(self.mBestLevel),
            2 => var3.write_int(self.mBestScore),
            3 => {
                if !self.hasBall || self.tileMap.is_none() {
                    return; // Java: NullPointerException, nothing saved
                }
                if self.mState == 3 {
                    // port-only: after a game over the original still saved the dead
                    // game as valid, so "Continue" came back with -1 lives
                    var3.write_byte(-16);
                } else {
                    var3.write_byte(-6);
                }
                var3.write_byte(self.numLives);
                var3.write_byte(self.numRings);
                var3.write_byte(self.mLevelNum);
                var3.write_byte(self.mBall.mBallSize);
                var3.write_int(self.mScore);
                var3.write_int(self.tileX);
                var3.write_int(self.tileY);
                var3.write_int(self.mBall.globalBallX);
                var3.write_int(self.mBall.globalBallY);
                var3.write_int(self.mBall.xSpeed);
                var3.write_int(self.mBall.ySpeed);
                var3.write_int(self.mBall.xPos);
                var3.write_int(self.mBall.yPos);
                var3.write_int(self.mBall.respawnX);
                var3.write_int(self.mBall.respawnY);
                var3.write_int(self.mBall.speedBonusCntr);
                var3.write_int(self.mBall.gravBonusCntr);
                var3.write_int(self.mBall.jumpBonusCntr);
                let mut var5: Vec<[i32; 3]> = Vec::new();
                for var7 in 0..self.mTileMapHeight {
                    for var8 in 0..self.mTileMapWidth {
                        let var4 = (self.tm(var7, var8) & 0xFF7F & -65) as i8 as i32;
                        if matches!(var4, 7 | 29 | 13 | 14 | 21 | 22 | 15 | 16 | 23 | 24) {
                            if var5.len() >= 40 {
                                // Java: ArrayIndexOutOfBoundsException, nothing saved
                                return;
                            }
                            var5.push([var7, var8, var4]);
                        }
                    }
                }
                var3.write_byte(var5.len() as i32);
                for t in &var5 {
                    var3.write_short(t[0]);
                    var3.write_short(t[1]);
                    var3.write_byte(t[2]);
                }
            }
            _ => {}
        }
        let mut var9 = RecordStore::open(RMS_NAME);
        var9.set_record(var1, &var3.0);
    }

    pub fn gameOver(&mut self) {
        self.mState = 3;
        self.mSavedValid = -16;
        self.displayGameOver();
    }

    /// `Bounce.destroyApp(true)`
    pub fn destroyApp(&mut self) {
        self.stop();
        self.saveGameData(3);
    }
}
