//! The engine's `Anime` class (native class 13): frame programs from an
//! `.ax` file (see [`silky_format::ax`]) that draw cells of a picture
//! sheet over the background, such as the blinking faces of 同級生 Remake.
//!
//! Read from 同級生 Remake's program. The manager holds 10 groups of 32
//! programs, each with a state: running, stopping at its next hold,
//! pausing at its next hold, paused, or off. Every frame each program
//! that is neither paused nor off takes a step. The cells come from the
//! picture in sprite 5 and are drawn into the background (sprite 0), so a
//! cell stays until the background is loaded again; a cell of kind 3 puts
//! the background back under itself first.
//!
//! Calls (values counted from the last one pushed): `Anime.0(file)` loads
//! the programs; `Anime.1(index, group)` picks a program, `Anime.2` starts
//! it, `Anime.3` stops it at its next hold, `Anime.4` stops it now;
//! `Anime.5(group)` and `Anime.13(_, group)` stop a group, `Anime.6`
//! stops every program at its next hold, `Anime.7` stops them all now;
//! `Anime.9` and `Anime.11` let paused programs go on; `Anime.10` pauses
//! the running ones at their next hold and `Anime.12` waits for a program
//! to end. The two waits are not kept: here `Anime.10` pauses at once and
//! `Anime.12` runs the program to its end at once.

use std::sync::Arc;

use serde::{Deserialize, Serialize};
use silky_format::ax::{Ax, Op, GROUPS, PROGRAMS};

const RUNNING: u8 = 0;
const STOPPING: u8 = 1;
const PAUSING: u8 = 3;
const PAUSED: u8 = 4;
const OFF: u8 = 0xff;

/// The sprite whose picture the cells come from.
pub const SHEET_SPRITE: i32 = 5;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
struct Track {
    state: u8,
    start: Option<u32>,
    pc: u32,
    wait: u32,
    hold: u32,
    /// Two loop levels: count left, where the loop starts, whether it
    /// counts.
    loops: [(u32, u32, bool); 2],
}

impl Default for Track {
    fn default() -> Self {
        Self {
            state: OFF,
            start: None,
            pc: 0,
            wait: 0,
            hold: 0,
            loops: [(0, 0, false); 2],
        }
    }
}

/// A cell drawn into the background.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Shown {
    pub sheet: String,
    /// Left, top, width, height in the sheet.
    pub part: (u32, u32, u32, u32),
    pub x: i32,
    pub y: i32,
    /// Put the background back under the cell first.
    pub restore: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Anime {
    /// The `.ax` file loaded.
    pub file: Option<String>,
    /// The picture in sprite 5.
    pub sheet: Option<String>,
    tracks: Vec<Track>,
    /// Cells drawn into the background, in the order drawn. A cell drawn
    /// where another was replaces it.
    pub shown: Vec<Shown>,
}

impl Anime {
    fn track(&mut self, group: i32, index: i32) -> Option<&mut Track> {
        if !(0..GROUPS as i32).contains(&group) || !(0..PROGRAMS as i32).contains(&index) {
            return None;
        }
        if self.tracks.is_empty() {
            self.tracks = vec![Track::default(); GROUPS * PROGRAMS];
        }
        self.tracks
            .get_mut(group as usize * PROGRAMS + index as usize)
    }

    pub fn running(&self, group: i32, index: i32) -> bool {
        if !(0..GROUPS as i32).contains(&group) || !(0..PROGRAMS as i32).contains(&index) {
            return false;
        }
        self.tracks
            .get(group as usize * PROGRAMS + index as usize)
            .is_some_and(|t| t.state == RUNNING)
    }

    /// Whether a program takes steps.
    pub fn active(&self) -> bool {
        self.tracks
            .iter()
            .any(|t| t.state != OFF && t.state != PAUSED)
    }

    pub fn load(&mut self, file: &str) {
        self.file = Some(file.to_string());
    }

    /// `Anime.1`: picks program `index` of `group`, off until started.
    pub fn set(&mut self, ax: &Ax, group: i32, index: i32) {
        let start = ax.program(group.max(0) as usize, index.max(0) as usize);
        if let Some(t) = self.track(group, index) {
            *t = Track {
                start,
                ..Track::default()
            };
        }
    }

    /// `Anime.2`: starts a picked program.
    pub fn start(&mut self, group: i32, index: i32) {
        if let Some(t) = self.track(group, index) {
            if t.start.is_some() {
                t.state = RUNNING;
            }
        }
    }

    fn set_state(&mut self, group: i32, index: i32, state: u8) {
        if let Some(t) = self.track(group, index) {
            t.state = state;
        }
    }

    fn each(&mut self, mut f: impl FnMut(&mut Track)) {
        self.tracks.iter_mut().for_each(&mut f);
    }

    /// One engine call; false for the ones not known.
    pub fn call(&mut self, ax: Option<&Ax>, sub: i32, args: &[silky_vm::Value]) -> bool {
        let back = |k: usize| {
            args.len()
                .checked_sub(k + 1)
                .and_then(|i| args.get(i))
                .map_or(0, silky_vm::Value::int)
        };
        let (group, index) = (back(0), back(1));
        match sub {
            1 => {
                let Some(ax) = ax else { return false };
                self.set(ax, group, index);
            }
            2 => self.start(group, index),
            3 => self.set_state(group, index, STOPPING),
            4 => self.set_state(group, index, OFF),
            5 => self.stop_group(group),
            6 => self.each(|t| {
                if t.state != OFF {
                    t.state = STOPPING;
                }
            }),
            7 => self.each(|t| t.state = OFF),
            9 => {
                if let Some(t) = self.track(group, index) {
                    if t.state == PAUSED {
                        t.state = RUNNING;
                    }
                }
            }
            10 => self.each(|t| {
                if t.state == RUNNING {
                    t.state = PAUSED;
                }
            }),
            11 => self.each(|t| {
                if t.state == PAUSED {
                    t.state = RUNNING;
                }
            }),
            12 => {
                if let Some(ax) = ax {
                    let i = group as usize * PROGRAMS + index as usize;
                    // A program that never ends is cut off.
                    for _ in 0..100_000 {
                        match self.tracks.get(i) {
                            Some(t) if t.state != OFF && t.state != PAUSED => {
                                self.step_one(ax, i);
                            }
                            _ => break,
                        }
                    }
                    self.set_state(group, index, OFF);
                }
            }
            13 => self.stop_group(group),
            _ => return false,
        }
        true
    }

    pub fn stop_group(&mut self, group: i32) {
        for i in 0..PROGRAMS as i32 {
            self.set_state(group, i, OFF);
        }
    }

    /// The background was loaded again: the cells drawn into it are gone.
    pub fn clear_shown(&mut self) {
        self.shown.clear();
    }

    /// Takes `frames` steps; true if a cell was drawn.
    pub fn tick(&mut self, ax: &Ax, frames: u32) -> bool {
        let mut drawn = false;
        for _ in 0..frames {
            for i in 0..self.tracks.len() {
                let s = self.tracks[i].state;
                if s != OFF && s != PAUSED {
                    drawn |= self.step_one(ax, i);
                }
            }
        }
        drawn
    }

    /// One step of program `i`; true if it drew a cell.
    fn step_one(&mut self, ax: &Ax, i: usize) -> bool {
        let t = &mut self.tracks[i];
        let Some(start) = t.start else {
            t.state = OFF;
            return false;
        };
        if t.wait > 0 {
            t.wait -= 1;
            return false;
        }
        if t.hold > 0 {
            match t.state {
                STOPPING => t.state = OFF,
                PAUSING => t.state = PAUSED,
                _ => {}
            }
            t.hold -= 1;
            return false;
        }
        let (op, next) = ax.op(start + t.pc);
        t.pc = next - start;
        match op {
            Op::Nothing => {}
            Op::Hold(n) => {
                match t.state {
                    STOPPING => t.state = OFF,
                    PAUSING => t.state = PAUSED,
                    _ => {}
                }
                t.hold = n;
            }
            Op::Wait(n) => t.wait = n,
            Op::Restart => t.pc = 0,
            Op::End => t.state = OFF,
            Op::Loop { level, count } => {
                t.loops[level as usize] = (count, t.pc, count != 0);
            }
            Op::Next { level } => {
                let l = &mut t.loops[level as usize];
                if l.2 {
                    l.0 = l.0.wrapping_sub(1);
                    if l.0 == 0 {
                        return false;
                    }
                }
                t.pc = l.1;
            }
            Op::Draw(c) => {
                let (Some(cell), Some(sheet)) = (ax.cells.get(c as usize), self.sheet.clone())
                else {
                    return false;
                };
                let shown = Shown {
                    sheet,
                    part: (
                        cell.sx.max(0) as u32,
                        cell.sy.max(0) as u32,
                        cell.w.max(0) as u32,
                        cell.h.max(0) as u32,
                    ),
                    x: cell.x,
                    y: cell.y,
                    restore: cell.kind == 3,
                };
                let same = |s: &Shown| {
                    (s.x, s.y, s.part.2, s.part.3) == (shown.x, shown.y, shown.part.2, shown.part.3)
                };
                self.shown.retain(|s| !same(s));
                self.shown.push(shown);
                return true;
            }
        }
        false
    }
}

/// A loaded `.ax` file, kept by name.
pub type Loaded = Option<(String, Arc<Ax>)>;
