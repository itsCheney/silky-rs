//! Who is where in 同級生 Remake: the girl at a place on the town, the
//! yabuki or the school map at the story's current day and time. The
//! face pins of the map jump page ask it.
//!
//! The rules are decision graphs read from the executable and lifted into
//! `who_data.rs` (generated, see `docs/games/doukyuusei.md`); the story
//! state they test is used as is. A graph is one test per node; a leaf
//! is the face (a cell of `map_p.akb`) or -1 for nobody.

use silky_vm::Vm;

use super::plate::{DATE, TIME};

/// A value a test reads.
#[derive(Clone, Copy, Debug)]
pub enum Ex {
    Int(i32),
    /// The advanced clock, HHMM.
    Time,
    /// The advanced date, MMDD.
    Date,
    /// A byte variable.
    B(u16),
    /// A word variable (by word index).
    W(u16),
    /// A global.
    G(u16),
    /// The clock minus a word.
    SinceW(u16),
}

/// The tests; `Ult` and `Ule` compare as unsigned.
#[derive(Clone, Copy, Debug)]
pub enum Op {
    Eq,
    Ult,
    Ule,
    Lt,
    Le,
    /// `a` within `b..=c`.
    In,
}

/// One test: where to go when true and when false. A next is a node
/// index, or a leaf `-(face + 2)`.
#[derive(Clone, Copy, Debug)]
pub struct Node {
    op: Op,
    a: Ex,
    b: Ex,
    c: i32,
    then: i16,
    other: i16,
}

/// Shorthand for the generated tables.
pub(super) const fn n(op: Op, a: Ex, b: Ex, c: i32, then: i16, other: i16) -> Node {
    Node {
        op,
        a,
        b,
        c,
        then,
        other,
    }
}

/// What the tests read.
pub struct Env<'a> {
    pub time: i32,
    pub date: i32,
    pub bytes: &'a [u8],
    pub words: &'a [u16],
    pub globals: &'a [i32],
}

impl Env<'_> {
    fn value(&self, e: Ex) -> i32 {
        let byte = |i: u16| self.bytes.get(i as usize).copied().unwrap_or(0) as i32;
        let word = |i: u16| self.words.get(i as usize).copied().unwrap_or(0) as i32;
        match e {
            Ex::Int(v) => v,
            Ex::Time => self.time,
            Ex::Date => self.date,
            Ex::B(i) => byte(i),
            Ex::W(i) => word(i),
            Ex::G(i) => self.globals.get(i as usize).copied().unwrap_or(0),
            Ex::SinceW(i) => self.time.wrapping_sub(word(i)),
        }
    }
}

fn test(n: &Node, env: &Env) -> bool {
    let (a, b) = (env.value(n.a), env.value(n.b));
    match n.op {
        Op::Eq => a == b,
        Op::Ult => (a as u32) < (b as u32),
        Op::Ule => (a as u32) <= (b as u32),
        Op::Lt => a < b,
        Op::Le => a <= b,
        Op::In => b <= a && a <= n.c,
    }
}

/// Runs graph `(map, place)`: the face, or -1. The clock in `env` is
/// already advanced, see [`advance`].
pub fn eval(map: u8, place: u8, env: &Env) -> i32 {
    let Some((nodes, mut at)) = super::who_data::graph(map, place) else {
        return -1;
    };
    while at >= 0 {
        let n = &nodes[at as usize];
        at = if test(n, env) { n.then } else { n.other };
    }
    -(at as i32) - 2
}

/// The clock `minutes` later: `(time, date)`; no month carry.
pub fn advance(time: i32, date: i32, minutes: i32) -> (i32, i32) {
    let t = time % 100 + minutes + time / 100 * 60;
    let mut h = t / 60;
    let mut d = date;
    if h >= 24 {
        h -= 24;
        d += 1;
    }
    (h * 100 + t % 60, d)
}

/// The maps with pins.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Map {
    Town,
    Yabuki,
    /// The school, with the floor argument the page passes: 2 on the 1F,
    /// 3 on the other floors (see [`super::jump`]).
    School(u8),
}

/// The floor of each school pin.
const PIN_FLOOR: [i32; 7] = [2, 2, 3, 3, 4, 5, 4];

/// The face at `place` (the pin number of the jump page's tables) now,
/// from the story state in `vm`.
pub fn face_at(vm: &Vm, map: Map, place: u8) -> Option<u8> {
    let (id, minutes) = match map {
        Map::Town => (0, 15),
        Map::Yabuki => (1, 15),
        Map::School(floor) => {
            let pin_floor = PIN_FLOOR
                .get(place as usize)
                .copied()
                .unwrap_or(floor as i32);
            (2, 10 + 10 * (floor as i32 - pin_floor).abs())
        }
    };
    let (time, date) = advance(vm.var_g(TIME).int(), vm.var_g(DATE).int(), minutes);
    let globals: Vec<i32> = (0..20).map(|i| vm.var_g(i).int()).collect();
    let env = Env {
        time,
        date,
        bytes: &vm.bytes,
        words: &vm.words,
        globals: &globals,
    };
    u8::try_from(eval(id, place, &env)).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_advance() {
        assert_eq!(advance(830, 810, 15), (845, 810));
        assert_eq!(advance(1955, 810, 15), (2010, 810));
        assert_eq!(advance(2350, 831, 15), (5, 832));
        assert_eq!(advance(900, 810, 10), (910, 810));
    }

    /// Cases dumped from the emulator of the original code (see
    /// `fl/sch/dump_fixture.py`): `map place time date face | b | w | g`
    /// with `i=v` lists, the rest zero.
    #[test]
    fn matches_the_original() {
        let text = include_str!("../../../tests/who_cases.txt");
        let mut seen = 0;
        for line in text.lines().filter(|l| !l.starts_with('#')) {
            let f: Vec<&str> = line.split('|').collect();
            let head: Vec<&str> = f[0].split_whitespace().collect();
            let num = |s: &str| s.parse::<i32>().unwrap();
            let fill = |s: &str, size: usize| {
                let mut v = vec![0i64; size];
                for kv in s.split_whitespace() {
                    let (k, x) = kv.split_once('=').unwrap();
                    v[k.parse::<usize>().unwrap()] = x.parse().unwrap();
                }
                v
            };
            let bytes: Vec<u8> = fill(f[1], 4000).iter().map(|&x| x as u8).collect();
            let words: Vec<u16> = fill(f[2], 300).iter().map(|&x| x as u16).collect();
            let globals: Vec<i32> = fill(f[3], 20).iter().map(|&x| x as i32).collect();
            let env = Env {
                time: num(head[2]),
                date: num(head[3]),
                bytes: &bytes,
                words: &words,
                globals: &globals,
            };
            let map = match head[0] {
                "town" => 0,
                "yabuki" => 1,
                _ => 2,
            };
            assert_eq!(eval(map, num(head[1]) as u8, &env), num(head[4]), "{line}");
            seen += 1;
        }
        assert!(seen > 200);
    }
}
