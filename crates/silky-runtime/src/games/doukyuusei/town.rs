//! The walking maps of 同級生 Remake (`Game.20`, the program's `CCps`):
//! the town, the school and the station, read from the executable and
//! checked under Wine.
//!
//! A map is a grid of 48-pixel tiles over one big picture (`sakima.akb`,
//! 111 x 105 tiles) with a second picture drawn over the walkers
//! (`sakima_o.akb`). Three files go with it:
//!
//! - `.MPX`: width and height in tiles, then 5 bytes per tile, rows from
//!   the top; the fifth byte is a wall.
//! - `.CCD`: the walkers (14 bytes each: flags, look, width and height in
//!   tiles, x, y, direction and step, program, command, count, program
//!   counter), the programs they follow, and the places the hero starts
//!   from (camera x and y, hero x and y).
//! - `.EVE`: the places that end the walk, 12 bytes each (id, left, top,
//!   right, bottom, directions), up to id 0xffff.
//!
//! The hero is a walker that follows the player: the arrow keys, or the
//! pointer held down (towards it, diagonals included). One step a tile,
//! every 33 ms (the map speed setting), two with Shift or Ctrl. The camera
//! shows 40 x 23 tiles and keeps the hero 20 tiles from its sides and 12
//! from its top and bottom. A walk ends when the hero stands in a place,
//! facing one of its directions; the call returns the place's id.

use crate::assets::Assets;
use crate::render::Canvas;
use crate::{Error, Result};

pub const TILE: i32 = 48;
/// Tiles on screen.
const VIEW: (i32, i32) = (40, 23);
/// Distance the camera keeps from the hero: sides, top and bottom.
const MARGIN: (i32, i32) = (20, 12);
/// Size of a walker's frame on its sheet.
const FRAME: u32 = 144;
/// Raw bytes kept for saves: camera x and y.
const RAW_CAMERA: usize = 7097;
/// Raw bytes kept for saves: x, y and direction of each walker.
const RAW_WALKERS: usize = 7091;
/// Milliseconds a step takes at each map speed (slow, normal, fast).
pub const STEP_MS: [u64; 3] = [66, 33, 16];

/// Walker flags.
pub mod flags {
    /// Drawn.
    pub const SHOWN: u8 = 0x01;
    /// Stopped by walls.
    pub const WALLS: u8 = 0x10;
    /// The camera follows it.
    pub const CAMERA: u8 = 0x20;
    /// The hero.
    pub const HERO: u8 = 0x40;
}

/// Direction bits of the player's input.
pub mod dir {
    pub const UP: u8 = 1;
    pub const LEFT: u8 = 2;
    pub const RIGHT: u8 = 4;
    pub const DOWN: u8 = 8;
}

/// Facing, the high half of a walker's direction byte.
const UP: u8 = 0;
const DOWN: u8 = 1;
const LEFT: u8 = 2;
const RIGHT: u8 = 3;

/// The player's input for a step.
#[derive(Debug, Clone, Copy, Default)]
pub struct Input {
    /// Arrow keys held ([`dir`] bits).
    pub keys: u8,
    /// Pointer held down at this position (game pixels).
    pub pointer: Option<(i32, i32)>,
    /// Shift or Ctrl held: two tiles a step.
    pub fast: bool,
}

#[derive(Debug, Clone, Default)]
struct Grid {
    w: i32,
    h: i32,
    walls: Vec<bool>,
}

impl Grid {
    fn parse(d: &[u8]) -> Result<Self> {
        let w = u16_at(d, 0)? as i32;
        let h = u16_at(d, 2)? as i32;
        let n = (w * h) as usize;
        if d.len() < 4 + 5 * n {
            return Err(Error::Invalid("short map grid".into()));
        }
        let walls = (0..n).map(|i| d[4 + 5 * i + 4] != 0).collect();
        Ok(Self { w, h, walls })
    }

    /// Outside the map counts as open, as in the program (its grid is
    /// bigger than the map and zeroed).
    fn wall(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.walls[(y * self.w + x) as usize]
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Walker {
    pub flags: u8,
    /// Row of looks on the sheet (the program's patterns; unused with
    /// sheets).
    pub look: u8,
    pub w: u8,
    pub h: u8,
    pub x: u16,
    pub y: u16,
    /// Facing in the high half, step of the walk (0..12) in the low half.
    pub dir: u8,
    pub program: u8,
    command: u8,
    count: u8,
    pc: u16,
}

impl Walker {
    fn facing(&self) -> u8 {
        self.dir >> 4
    }

    /// Faces `d`, keeping the step (back to 0 past 11).
    fn face(&mut self, d: u8) {
        let s = self.dir & 0xf;
        self.dir = (d << 4) | if s < 12 { s } else { 0 };
    }

    /// The next step of the walk.
    fn advance(&mut self) {
        let s = (self.dir & 0xf) + 1;
        self.dir = (self.dir & 0xf0) | if s < 12 { s } else { 0 };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Place {
    id: u16,
    left: u16,
    top: u16,
    right: u16,
    bottom: u16,
    /// Facings that enter it, bit per facing.
    facings: u16,
}

#[derive(Debug, Clone, Copy, Default)]
struct Camera {
    x: i32,
    y: i32,
}

/// A walking map and its walkers.
#[derive(Debug, Clone, Default)]
pub struct Town {
    grid: Grid,
    camera: Camera,
    ccd: Vec<u8>,
    pub walkers: Vec<Walker>,
    places: Vec<Place>,
    /// 2: a place is looked for after each step of the hero and found
    /// once; 3: looked for always, found when it changes.
    place_mode: u8,
    place_armed: bool,
    last_place: u16,
    /// Last 32 positions of the hero, for walkers that follow.
    trail: [(u16, u16, u8); 32],
    trail_at: usize,
    pub picture: Option<String>,
    pub over: Option<String>,
    /// Walker sheets: the hero's, the others'.
    pub sheets: [Option<String>; 2],
    /// A diagonal step is being taken (no sliding along walls).
    diagonal: bool,
    fast: bool,
}

fn u16_at(d: &[u8], at: usize) -> Result<u16> {
    d.get(at..at + 2)
        .map(|b| u16::from_le_bytes([b[0], b[1]]))
        .ok_or_else(|| Error::Invalid("short walking map file".into()))
}

fn set_raw(raw: &mut Vec<u8>, at: usize, v: u8) {
    if raw.len() <= at {
        raw.resize(at + 1, 0);
    }
    raw[at] = v;
}

fn get_raw(raw: &[u8], at: usize) -> u8 {
    raw.get(at).copied().unwrap_or(0)
}

impl Town {
    /// `Game.20(name, 0)`: the grid; the camera goes to the top left.
    pub fn load_grid(&mut self, data: &[u8]) -> Result<()> {
        self.grid = Grid::parse(data)?;
        self.camera = Camera::default();
        self.place_mode = 0;
        self.place_armed = false;
        self.last_place = 0xffff;
        Ok(())
    }

    /// `Game.20(name, 4)`: the walkers and their programs.
    pub fn load_walkers(&mut self, data: Vec<u8>) -> Result<()> {
        let first = u16_at(&data, 0)? as usize;
        let programs = u16_at(&data, 2)? as usize;
        let count = programs.saturating_sub(9) / 14;
        let mut walkers = Vec::with_capacity(count);
        for i in 0..count {
            let at = first + 14 * i;
            let r = data
                .get(at..at + 14)
                .ok_or_else(|| Error::Invalid("short walker table".into()))?;
            let mut w = Walker {
                flags: r[0],
                look: r[1],
                w: r[2],
                h: r[3],
                x: u16::from_le_bytes([r[4], r[5]]),
                y: u16::from_le_bytes([r[6], r[7]]),
                dir: r[8],
                program: r[9],
                ..Default::default()
            };
            // The program's start is looked up four bytes a program here
            // (two everywhere else), and the counter is not moved past the
            // first command, which then runs twice.
            w.pc = u16_at(&data, programs + 4 * w.program as usize).unwrap_or(0);
            let b = data.get(w.pc as usize).copied().unwrap_or(0);
            w.command = b >> 4;
            w.count = (b & 0xf) + 1;
            walkers.push(w);
        }
        self.walkers = walkers;
        self.ccd = data;
        Ok(())
    }

    /// `Game.20(mode, name, 12)`: the places.
    pub fn load_places(&mut self, data: &[u8], mode: i32) -> Result<()> {
        let mut places = Vec::new();
        let mut at = 0;
        loop {
            let id = u16_at(data, at)?;
            if id == 0xffff {
                break;
            }
            let v = |k: usize| u16_at(data, at + 2 * k);
            places.push(Place {
                id,
                left: v(1)?,
                top: v(2)?,
                right: v(3)?,
                bottom: v(4)?,
                facings: v(5)?,
            });
            at += 12;
        }
        self.places = places;
        self.place_mode = mode as u8;
        self.place_armed = mode != 0;
        self.last_place = 0xffff;
        Ok(())
    }

    fn clamp_camera(&mut self) {
        if self.camera.x + VIEW.0 >= self.grid.w {
            self.camera.x = self.grid.w - VIEW.0;
        }
        if self.camera.y + VIEW.1 >= self.grid.h {
            self.camera.y = self.grid.h - VIEW.1;
        }
    }

    fn store_camera(&self, raw: &mut Vec<u8>) {
        set_raw(raw, RAW_CAMERA, self.camera.x as u8);
        set_raw(raw, RAW_CAMERA + 1, self.camera.y as u8);
    }

    fn store_walker(&self, i: usize, raw: &mut Vec<u8>) {
        let w = self.walkers[i];
        set_raw(raw, RAW_WALKERS + 3 * i, w.x as u8);
        set_raw(raw, RAW_WALKERS + 3 * i + 1, w.y as u8);
        set_raw(raw, RAW_WALKERS + 3 * i + 2, w.dir);
    }

    fn fill_trail(&mut self, i: usize) {
        let w = self.walkers[i];
        self.trail = [(w.x, w.y, w.dir); 32];
    }

    fn push_trail(&mut self, i: usize) {
        let w = self.walkers[i];
        self.trail[self.trail_at] = (w.x, w.y, w.dir);
        self.trail_at = (self.trail_at + 1) & 31;
    }

    /// `Game.20(facing, walker, start, 1)`: the camera and a walker at
    /// one of the starting places of the `.CCD`.
    pub fn place(&mut self, start: i32, i: usize, facing: i32, raw: &mut Vec<u8>) {
        let Ok(table) = u16_at(&self.ccd, 4) else {
            return;
        };
        let at = table as usize + 4 * start.max(0) as usize;
        let Some(s) = self.ccd.get(at..at + 4) else {
            return;
        };
        let s = [s[0], s[1], s[2], s[3]];
        self.camera = Camera {
            x: s[0] as i32,
            y: s[1] as i32,
        };
        self.clamp_camera();
        self.store_camera(raw);
        let Some(w) = self.walkers.get_mut(i) else {
            return;
        };
        w.x = s[2] as u16;
        w.y = s[3] as u16;
        w.dir = (facing as u8) << 4;
        self.store_walker(i, raw);
        self.fill_trail(i);
    }

    /// `Game.20(walker, 31)`: the camera and a walker where the raw
    /// bytes keep them (back on the map).
    pub fn restore(&mut self, i: usize, raw: &mut Vec<u8>) {
        self.camera = Camera {
            x: get_raw(raw, RAW_CAMERA) as i32,
            y: get_raw(raw, RAW_CAMERA + 1) as i32,
        };
        self.clamp_camera();
        let Some(w) = self.walkers.get_mut(i) else {
            return;
        };
        w.x = get_raw(raw, RAW_WALKERS + 3 * i) as u16;
        w.y = get_raw(raw, RAW_WALKERS + 3 * i + 1) as u16;
        w.dir = get_raw(raw, RAW_WALKERS + 3 * i + 2);
        self.fill_trail(i);
    }

    /// `Game.20(value, walker, 7)`.
    pub fn set_flags(&mut self, i: usize, v: i32) {
        if let Some(w) = self.walkers.get_mut(i) {
            w.flags = v as u8;
        }
    }

    /// `Game.20(facing, walker, 22)`.
    pub fn set_facing(&mut self, i: usize, v: i32) {
        if let Some(w) = self.walkers.get_mut(i) {
            w.dir = (v as u8) << 4;
        }
    }

    /// `Game.20(program, walker, 5)`: the walker starts a program.
    pub fn set_program(&mut self, i: usize, program: i32) {
        let start = self.program_start(program as u8);
        let first = self.byte(start);
        if let Some(w) = self.walkers.get_mut(i) {
            if w.flags == 0xff {
                return;
            }
            w.program = program as u8;
            w.pc = start.wrapping_add(1);
            w.command = first >> 4;
            w.count = (first & 0xf) + 1;
        }
    }

    /// `Game.20(n, walker, 20)`: a follower `n` steps behind the hero.
    pub fn follow(&mut self, i: usize, n: i32) {
        let at = (self.trail_at as i32 - 1).rem_euclid(32) - 2 * n;
        let at = if at < 0 { at + 32 } else { at };
        let (x, y, d) = self.trail[(at & 31) as usize];
        if let Some(w) = self.walkers.get_mut(i) {
            w.x = x;
            w.y = y;
            w.dir = d;
        }
    }

    /// `Game.20(facing, walker, 14)`: one step of a walker.
    pub fn step_walker(&mut self, i: usize, facing: i32) {
        if i >= self.walkers.len() {
            return;
        }
        match facing {
            0 => self.up(i, true),
            1 => self.down(i, true),
            2 => self.left(i, true),
            3 => self.right(i, true),
            _ => 0,
        };
        self.push_trail(i);
    }

    fn byte(&self, at: u16) -> u8 {
        self.ccd.get(at as usize).copied().unwrap_or(0)
    }

    fn program_start(&self, program: u8) -> u16 {
        let table = u16_at(&self.ccd, 2).unwrap_or(0) as usize;
        u16_at(&self.ccd, table + 2 * program as usize).unwrap_or(0)
    }

    fn hero(&self) -> Option<usize> {
        for (i, w) in self.walkers.iter().enumerate() {
            if w.flags == 0xff {
                return None;
            }
            if w.flags & flags::HERO != 0 {
                return Some(i);
            }
        }
        None
    }

    /// The place the hero stands in, as the program looks for it after a
    /// step; 0xffff for none (or none new).
    pub fn find_place(&mut self) -> u16 {
        if !self.place_armed {
            return 0xffff;
        }
        let Some(h) = self.hero() else {
            return 0xffff;
        };
        let w = self.walkers[h];
        let bit = 1u16 << w.facing();
        let (x, y) = (w.x as u32, w.y as u32);
        let id = self
            .places
            .iter()
            .find(|p| {
                x <= p.right as u32
                    && x + w.w as u32 > p.left as u32
                    && y + 1 <= p.bottom as u32
                    && y + w.h as u32 > p.top as u32
                    && p.facings & bit != 0
            })
            .map_or(0xffff, |p| p.id);
        match self.place_mode {
            2 => {
                self.place_armed = false;
                id
            }
            3 => {
                if self.last_place == id {
                    0xffff
                } else {
                    self.last_place = id;
                    id
                }
            }
            _ => id,
        }
    }

    /// One step of every walker (the program's step timer), then the
    /// place the hero reached, if any.
    pub fn tick(&mut self, input: &Input, raw: &mut Vec<u8>) -> Option<u16> {
        self.fast = input.fast;
        self.diagonal = false;
        for i in 0..self.walkers.len() {
            let f = self.walkers[i].flags;
            if f == 0 || f == 0xff {
                continue;
            }
            self.run(i, input);
            self.store_walker(i, raw);
        }
        self.store_camera(raw);
        match self.find_place() {
            0xffff => None,
            id => Some(id),
        }
    }

    /// One step of a walker's program.
    fn run(&mut self, i: usize, input: &Input) -> u16 {
        if self.walkers[i].count == 0 {
            let mut pc = self.walkers[i].pc;
            let mut b = self.byte(pc);
            pc = pc.wrapping_add(1);
            let mut guard = 0;
            while b == 0 && guard < 4 {
                pc = self.program_start(self.walkers[i].program);
                b = self.byte(pc);
                pc = pc.wrapping_add(1);
                guard += 1;
            }
            let w = &mut self.walkers[i];
            w.pc = pc;
            w.command = b >> 4;
            w.count = b & 0xf;
        }
        let command = self.walkers[i].command;
        let r: u16 = match command {
            0 => {
                self.walkers[i].pc = 0;
                0
            }
            1 | 8 => {
                let w = &mut self.walkers[i];
                w.dir = w.count.wrapping_sub(1) << 4;
                w.count = 1;
                0
            }
            2 => self.up(i, true),
            3 => self.down(i, true),
            4 => self.left(i, true),
            5 => self.right(i, true),
            6 => {
                let w = self.walkers[i];
                let at = (self.trail_at as u8)
                    .wrapping_sub(w.count.wrapping_mul(2))
                    .wrapping_sub(1)
                    & 31;
                let (x, y, d) = self.trail[at as usize];
                let w = &mut self.walkers[i];
                w.x = x;
                w.y = y;
                w.dir = d;
                0
            }
            9 => {
                let w = &mut self.walkers[i];
                w.count = (w.count >> 1) + 1;
                0
            }
            14 => self.player(i, input),
            // 7 (wandering) and 10..=12 are not in the program; 13 and 15
            // walk to a target (`Game.20` sub 15), not read yet.
            7 => 0xffff,
            _ => 0,
        };
        let low = r & 0xff;
        if low != 0 {
            return if low == 0xff { 0 } else { r };
        }
        let w = &mut self.walkers[i];
        if w.count != 0xff {
            w.count = w.count.wrapping_sub(1);
        }
        if self.place_mode == 2 && w.flags & flags::HERO != 0 {
            self.place_armed = true;
        }
        0xffff
    }

    /// The player moves the hero.
    fn player(&mut self, i: usize, input: &Input) -> u16 {
        let d = self.direction(i, input);
        let moved = |r: u16| r != 0xffff && r & 0xff != 0xff;
        let mut any = false;
        match d {
            dir::UP => any |= moved(self.up(i, true)),
            dir::LEFT => any |= moved(self.left(i, true)),
            dir::RIGHT => any |= moved(self.right(i, true)),
            dir::DOWN => any |= moved(self.down(i, true)),
            3 | 5 | 10 | 12 => {
                self.diagonal = true;
                any |= moved(if d & dir::LEFT != 0 {
                    self.left(i, true)
                } else {
                    self.right(i, true)
                });
                any |= moved(if d & dir::UP != 0 {
                    self.up(i, true)
                } else {
                    self.down(i, true)
                });
                self.diagonal = false;
            }
            _ => return 0xffff,
        }
        if !any {
            return 0xff;
        }
        self.push_trail(i);
        0
    }

    /// The player's direction: towards the pointer while it is held
    /// (none on the walker itself), else the arrow keys.
    fn direction(&self, i: usize, input: &Input) -> u8 {
        let Some((mx, my)) = input.pointer else {
            return input.keys;
        };
        if mx < 0 || my < 0 || mx / TILE >= VIEW.0 || my / TILE >= VIEW.1 {
            return 0;
        }
        let w = self.walkers[i];
        let hx = (w.x as i32 - self.camera.x) * TILE;
        let hy = (w.y as i32 - self.camera.y) * TILE;
        let (bw, bh) = (w.w as i32 * TILE, w.h as i32 * TILE);
        let (hx16, hy16) = (hx & 0xffff, hy & 0xffff);
        if mx >= hx.max(0) && mx <= hx16 + bw && my >= hy.max(0) && my <= hy16 + bh {
            return 0;
        }
        let mut d = 0;
        if my < hy16 {
            d |= dir::UP;
        }
        if my > hy16 + bh {
            d |= dir::DOWN;
        }
        if mx < hx16 {
            d |= dir::LEFT;
        }
        if mx > hx16 + bw {
            d |= dir::RIGHT;
        }
        d
    }

    fn follows(&self, i: usize) -> bool {
        self.walkers[i].flags & flags::CAMERA != 0
    }

    fn walls(&self, i: usize) -> bool {
        self.walkers[i].flags & flags::WALLS != 0
    }

    fn camera_up(&mut self, i: usize) {
        let y = self.walkers[i].y as i32;
        if self.follows(i) && self.camera.y + MARGIN.1 > y && self.camera.y != 0 {
            self.camera.y -= 1;
        }
    }

    fn camera_down(&mut self, i: usize) {
        let y = self.walkers[i].y as i32;
        if self.follows(i)
            && self.camera.y + VIEW.1 - 1 - MARGIN.1 <= y
            && self.camera.y + VIEW.1 < self.grid.h
        {
            self.camera.y += 1;
        }
    }

    fn camera_left(&mut self, i: usize) {
        let x = self.walkers[i].x as i32;
        if self.follows(i) && self.camera.x + MARGIN.0 > x && self.camera.x != 0 {
            self.camera.x -= 1;
        }
    }

    fn camera_right(&mut self, i: usize) {
        let x = self.walkers[i].x as i32;
        if self.follows(i)
            && self.camera.x + VIEW.0 - 1 - MARGIN.0 <= x
            && self.camera.x + VIEW.0 < self.grid.w
        {
            self.camera.x += 1;
        }
    }

    /// A wall in row `y` over the walker's columns.
    fn wall_row(&self, i: usize, y: i32) -> bool {
        let w = self.walkers[i];
        (0..w.w as i32).any(|k| self.grid.wall(w.x as i32 + k, y))
    }

    /// A wall in column `x` beside the walker (all rows but its top).
    fn wall_column(&self, i: usize, x: i32) -> bool {
        let w = self.walkers[i];
        (1..w.h as i32).any(|k| self.grid.wall(x, w.y as i32 + k))
    }

    /// Two steps a tick with Shift or Ctrl: the second step's result
    /// unless it was blocked.
    fn again(&mut self, i: usize, anim: bool, r: u16, f: fn(&mut Self, usize, bool) -> u16) -> u16 {
        if self.fast && anim {
            let r2 = f(self, i, false);
            if r2 != 0xffff {
                return r2;
            }
        }
        r
    }

    fn up(&mut self, i: usize, anim: bool) -> u16 {
        if anim {
            self.walkers[i].face(UP);
        }
        let w = self.walkers[i];
        if self.walls(i) && self.wall_row(i, w.y as i32) {
            let r = self.slide_sideways(i, anim);
            return self.again(i, anim, r, Self::up);
        }
        if w.y > 0 {
            self.walkers[i].y -= 1;
        }
        if anim {
            self.walkers[i].advance();
        }
        self.camera_up(i);
        self.again(i, anim, 0, Self::up)
    }

    fn down(&mut self, i: usize, anim: bool) -> u16 {
        if anim {
            self.walkers[i].face(DOWN);
        }
        let w = self.walkers[i];
        let below = w.y as i32 + w.h as i32;
        if below >= self.grid.h {
            return 0xffff;
        }
        if self.walls(i) && self.wall_row(i, below) {
            let r = self.slide_sideways(i, anim);
            return self.again(i, anim, r, Self::down);
        }
        self.walkers[i].y += 1;
        if anim {
            self.walkers[i].advance();
        }
        self.camera_down(i);
        self.again(i, anim, 0, Self::down)
    }

    fn left(&mut self, i: usize, anim: bool) -> u16 {
        if anim {
            self.walkers[i].face(LEFT);
        }
        let w = self.walkers[i];
        if w.x == 0 {
            return 0xffff;
        }
        if self.walls(i) && self.wall_column(i, w.x as i32 - 1) {
            let r = self.slide_vertically(i, anim);
            return self.again(i, anim, r, Self::left);
        }
        self.walkers[i].x -= 1;
        if anim {
            self.walkers[i].advance();
        }
        self.camera_left(i);
        self.again(i, anim, 0, Self::left)
    }

    fn right(&mut self, i: usize, anim: bool) -> u16 {
        if anim {
            self.walkers[i].face(RIGHT);
        }
        let w = self.walkers[i];
        let beside = w.x as i32 + w.w as i32;
        if beside >= self.grid.w {
            return 0xffff;
        }
        if self.walls(i) && self.wall_column(i, beside) {
            let r = self.slide_vertically(i, anim);
            return self.again(i, anim, r, Self::right);
        }
        self.walkers[i].x += 1;
        if anim {
            self.walkers[i].advance();
        }
        self.camera_right(i);
        self.again(i, anim, 0, Self::right)
    }

    /// Up or down into a wall: sideways along it when the tile beside the
    /// walker's second row is open (right first).
    fn slide_sideways(&mut self, i: usize, anim: bool) -> u16 {
        if self.diagonal {
            return 0xffff;
        }
        let w = self.walkers[i];
        let row = w.y as i32 + 1;
        let open_right = !self.grid.wall(w.x as i32 + w.w as i32, row);
        let open_left = !self.grid.wall(w.x as i32 - 1, row);
        if open_right {
            self.slide_right(i, anim, open_left)
        } else if open_left {
            self.slide_left(i, anim)
        } else {
            0xffff
        }
    }

    fn slide_right(&mut self, i: usize, anim: bool, open_left: bool) -> u16 {
        let w = self.walkers[i];
        let beside = w.x as i32 + w.w as i32;
        if beside >= self.grid.w || (self.walls(i) && self.wall_column(i, beside)) {
            return if open_left {
                self.slide_left(i, anim)
            } else {
                0xffff
            };
        }
        self.walkers[i].x += 1;
        if anim {
            self.walkers[i].face(RIGHT);
            self.walkers[i].advance();
        }
        self.camera_right(i);
        0
    }

    fn slide_left(&mut self, i: usize, anim: bool) -> u16 {
        let w = self.walkers[i];
        if w.x == 0 || (self.walls(i) && self.wall_column(i, w.x as i32 - 1)) {
            return 0xffff;
        }
        self.walkers[i].x -= 1;
        if anim {
            self.walkers[i].face(LEFT);
            self.walkers[i].advance();
        }
        self.camera_left(i);
        0
    }

    /// Left or right into a wall: up along it when the tile above the
    /// walker's left column is open, else down when the one below is.
    fn slide_vertically(&mut self, i: usize, anim: bool) -> u16 {
        if self.diagonal {
            return 0xffff;
        }
        let w = self.walkers[i];
        let (x, y) = (w.x as i32, w.y as i32);
        let open_up = !self.grid.wall(x, y);
        let open_down = !self.grid.wall(x, y + w.h as i32);
        if !open_up || y == 0 || (self.walls(i) && self.wall_row(i, y)) {
            return if open_down {
                self.slide_down(i, anim)
            } else {
                0xffff
            };
        }
        self.walkers[i].y -= 1;
        if anim {
            self.walkers[i].face(UP);
            self.walkers[i].advance();
        }
        self.camera_up(i);
        0
    }

    fn slide_down(&mut self, i: usize, anim: bool) -> u16 {
        let w = self.walkers[i];
        let below = w.y as i32 + w.h as i32;
        if below >= self.grid.h || (self.walls(i) && self.wall_row(i, below)) {
            return 0xffff;
        }
        self.walkers[i].y += 1;
        if anim {
            self.walkers[i].face(DOWN);
            self.walkers[i].advance();
        }
        self.camera_down(i);
        0
    }

    /// The places: id, left, top, right, bottom and the facings that
    /// enter them (bit 1 up, 2 down, 4 left, 8 right).
    pub fn places(&self) -> Vec<(u16, u16, u16, u16, u16, u16)> {
        self.places
            .iter()
            .map(|p| (p.id, p.left, p.top, p.right, p.bottom, p.facings))
            .collect()
    }

    /// Width and height of the map in tiles.
    pub fn size(&self) -> (i32, i32) {
        (self.grid.w, self.grid.h)
    }

    /// A wall at a tile.
    pub fn wall(&self, x: i32, y: i32) -> bool {
        self.grid.wall(x, y)
    }

    /// What to draw: the map at the camera, the walkers shown (by their
    /// rows, top first) and the picture over them.
    pub fn view(&self) -> View {
        let mut walkers: Vec<(usize, Walker)> = self
            .walkers
            .iter()
            .copied()
            .enumerate()
            .filter(|(_, w)| w.flags != 0xff && w.flags & flags::SHOWN != 0)
            .collect();
        walkers.sort_by_key(|(_, w)| w.y);
        let sprites = walkers
            .into_iter()
            .filter_map(|(i, w)| {
                let sheet = self.sheets[if i == 0 { 0 } else { 1 }].clone()?;
                let facing = w.facing() as u32;
                let sx = [864, 0, 0, 864][facing as usize & 3] + (w.dir as u32 & 0xf) % 6 * FRAME;
                let sy = facing / 2 * FRAME;
                let x = (w.x as i32 - self.camera.x) * TILE;
                let y = (w.y as i32 - self.camera.y) * TILE;
                Some(Sprite {
                    sheet,
                    sx,
                    sy,
                    x,
                    y,
                })
            })
            .collect();
        View {
            picture: self.picture.clone(),
            over: self.over.clone(),
            camera: (self.camera.x * TILE, self.camera.y * TILE),
            sprites,
        }
    }
}

/// A walker as drawn.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Sprite {
    pub sheet: String,
    pub sx: u32,
    pub sy: u32,
    pub x: i32,
    pub y: i32,
}

/// A walking map as drawn into the background.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct View {
    pub picture: Option<String>,
    pub over: Option<String>,
    /// Top left of the screen on the map picture, in pixels.
    pub camera: (i32, i32),
    pub sprites: Vec<Sprite>,
}

impl View {
    pub fn draw(&self, canvas: &mut Canvas, assets: &mut Assets) -> Result<()> {
        let (cx, cy) = (self.camera.0.max(0) as u32, self.camera.1.max(0) as u32);
        let area = (cx, cy, 1920, (VIEW.1 * TILE) as u32);
        if let Some(p) = &self.picture {
            let img = assets.image(p)?;
            canvas.draw_part(&img, area, 0, 0, false, 255);
        }
        for s in &self.sprites {
            let img = assets.image(&s.sheet)?;
            canvas.draw_part(&img, (s.sx, s.sy, FRAME, FRAME), s.x, s.y, false, 255);
        }
        if let Some(p) = &self.over {
            let img = assets.image(p)?;
            canvas.draw_part(&img, area, 0, 0, false, 255);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn town() -> Town {
        // A 50 x 40 map, a wall at (10, 10), one 3 x 3 hero.
        let (w, h) = (50u16, 40u16);
        let mut mpx = Vec::new();
        mpx.extend_from_slice(&w.to_le_bytes());
        mpx.extend_from_slice(&h.to_le_bytes());
        for y in 0..h {
            for x in 0..w {
                mpx.extend_from_slice(&[0, 0, 0, 0, (x == 10 && y == 10) as u8]);
            }
        }
        let mut t = Town::default();
        t.load_grid(&mpx).unwrap();
        // Header: walkers at 8, programs at 23 (one walker), starts at 27.
        let mut ccd = vec![8, 0, 23, 0, 27, 0, 0, 0];
        ccd.extend_from_slice(&[0x71, 0, 3, 3, 20, 0, 20, 0, 0x10, 0, 0, 0, 0, 0]);
        ccd.push(0xff);
        ccd.extend_from_slice(&[25, 0]); // program 0 at 25
        ccd.extend_from_slice(&[0xe0, 0x00]); // follow the player
        ccd.extend_from_slice(&[0, 0, 5, 5]); // start 0
        t.load_walkers(ccd).unwrap();
        // One place at (5..7, 5), entered facing up.
        let mut eve = Vec::new();
        for v in [7u16, 5, 5, 7, 5, 1, 0xffff] {
            eve.extend_from_slice(&v.to_le_bytes());
        }
        t.load_places(&eve, 3).unwrap();
        t
    }

    #[test]
    fn walks_into_a_place() {
        let mut t = town();
        let mut raw = Vec::new();
        t.place(0, 0, 1, &mut raw);
        assert_eq!((t.walkers[0].x, t.walkers[0].y), (5, 5));
        assert_eq!(raw[RAW_WALKERS..RAW_WALKERS + 3], [5, 5, 0x10]);
        let up = Input {
            keys: dir::UP,
            ..Default::default()
        };
        // Facing down in the place: nothing; one step up: found.
        assert_eq!(t.tick(&Input::default(), &mut raw), None);
        assert_eq!(t.tick(&up, &mut raw), Some(7));
        assert_eq!((t.walkers[0].x, t.walkers[0].y), (5, 4));
        assert_eq!(t.walkers[0].dir, 0x01);
    }

    #[test]
    fn walls_and_sliding() {
        let mut t = town();
        let mut raw = Vec::new();
        t.place(0, 0, 1, &mut raw);
        // Below the wall at (10, 10): the hero's rows y + 1 and y + 2
        // collide, so from (9, 11) up is blocked and slides right.
        t.walkers[0].x = 9;
        t.walkers[0].y = 10;
        let up = Input {
            keys: dir::UP,
            ..Default::default()
        };
        t.tick(&up, &mut raw);
        assert_eq!((t.walkers[0].x, t.walkers[0].y), (10, 10));
        // The pointer on the hero does not move it.
        let on = Input {
            pointer: Some((10 * TILE + 5, 10 * TILE + 5)),
            ..Default::default()
        };
        t.tick(&on, &mut raw);
        assert_eq!((t.walkers[0].x, t.walkers[0].y), (10, 10));
    }
}
