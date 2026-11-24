//! The flag list's records, built per girl the way the original builds
//! them: each girl's events are looked at in a fixed order, an event done
//! takes its day and time from the story words, a waiting event takes its
//! days from its own rule (often from the event before it) and is fitted to
//! now by the walking clock. Once one waiting event is out of reach, every
//! later one of that girl is too. Some girls gather a group of events and
//! sort it (done first, then by day) before adding it.
//!
//! The builders work on their own copy of the story bytes and set an
//! event's byte once they have looked at it, so a later rule that reads it
//! sees it set.

use super::flag_clock::Clock;
use super::flaglist::{Env, Record};

#[cfg(test)]
mod cases;
#[cfg(test)]
mod tests;

mod g00;
mod g01;
mod g02;
mod g03;
mod g04;
mod g05;
mod g06;
mod g07;
mod g08;
mod g09;
mod g10;
mod g11;
mod g12;
mod g13;

/// The text line that says the cash is not enough.
const NO_CASH: u16 = 306;
/// The byte that opens the list's events.
const UNLOCKED: usize = 8013;
/// Where the hero is: stage and place.
const STAGE_AT: usize = 8004;
const SUB_AT: usize = 8005;

pub(super) struct Ctx<'a> {
    /// The list's own copy of the story bytes.
    pub b: Vec<u8>,
    pub h: &'a [u16],
    pub clock: Clock,
    pub out: Vec<Record>,
    /// Once set, every later waiting event is out of reach.
    pub locked: bool,
}

/// An event's fixed parts: script, place line, stage, place in it, cost and
/// the byte it needs (-1 for none).
#[derive(Clone, Copy)]
pub(super) struct Ev {
    pub script: &'static str,
    pub place: u16,
    pub stage: i32,
    pub sub: i32,
    pub cost: i32,
    pub f0: i32,
}

impl<'a> Ctx<'a> {
    fn new(env: &Env<'a>) -> Self {
        let mut b = env.bytes.to_vec();
        if b.len() < 0x2400 {
            b.resize(0x2400, 0);
        }
        let clock = Clock {
            stage: b[STAGE_AT] as i32,
            sub: b[SUB_AT] as i32,
            time: env.time,
            date: env.date,
            cash: env.cash,
        };
        let locked = b[UNLOCKED] != 1;
        Ctx {
            b,
            h: env.words,
            clock,
            out: Vec::new(),
            locked,
        }
    }

    /// Whether byte `i` is exactly 1.
    pub fn set(&self, i: usize) -> bool {
        self.b[i] == 1
    }

    pub fn byte(&self, i: usize) -> u8 {
        self.b[i]
    }

    /// Story word `i`.
    pub fn h(&self, i: usize) -> i32 {
        self.h.get(i).copied().unwrap_or(0) as i32
    }

    /// The record `back` places from the end of `v` (the first one when
    /// there are fewer).
    pub fn back(v: &[Record], back: usize) -> &Record {
        &v[v.len()
            .saturating_sub(back)
            .max(0)
            .min(v.len().saturating_sub(1))]
    }

    /// The record `back` places from the end of the girl's list.
    pub fn prev(&self, back: usize) -> &Record {
        Self::back(&self.out, back)
    }

    /// A record of `ev` with the days and hours `d` (first day, last day or
    /// -1, first time, last time or -1).
    pub fn rec(ev: Ev, d: [i32; 4]) -> Record {
        Record {
            script: ev.script,
            place: ev.place,
            reason: 0,
            d0: d[0],
            d1: d[1],
            t0: d[2],
            t1: d[3],
            stage: ev.stage,
            sub: ev.sub,
            cost: ev.cost,
            f0: ev.f0,
            state: 0,
            jump: false,
        }
    }

    /// The record of `ev` done: its day and time from words `dw` and `tw`.
    pub fn done(&self, ev: Ev, dw: usize, tw: usize) -> Record {
        let mut r = Self::rec(ev, [self.h(dw), -1, self.h(tw), -1]);
        r.state = 1;
        r
    }

    /// Fits the waiting `r` to now; out of reach once it or one before it
    /// is.
    pub fn fit(&mut self, r: &mut Record) {
        if self.clock.fit(r, NO_CASH) {
            self.locked = true;
        }
        r.state = if self.locked { 2 } else { 0 };
    }

    /// Adds `r` to the list and marks byte `flag` looked at.
    pub fn push(&mut self, r: Record, flag: usize) {
        self.out.push(r);
        self.b[flag] = 1;
    }
}

/// Sorts a group the original's way, done first and then by first day,
/// keeping the order of equal ones, and adds it to `to`.
pub(super) fn sort_into(mut group: Vec<Record>, to: &mut Vec<Record>) {
    group.sort_by_key(|r| (r.state != 1, r.d0));
    to.extend(group);
}

/// The records of `girl`, or `None` while her rules are not built here.
pub(super) fn build(girl: usize, env: &Env) -> Vec<Record> {
    let mut cx = Ctx::new(env);
    let built = match girl {
        0 => g00::build(&mut cx),
        1 => g01::build(&mut cx),
        2 => g02::build(&mut cx),
        3 => g03::build(&mut cx),
        4 => g04::build(&mut cx),
        5 => g05::build(&mut cx),
        6 => g06::build(&mut cx),
        7 => g07::build(&mut cx),
        8 => g08::build(&mut cx),
        9 => g09::build(&mut cx),
        10 => g10::build(&mut cx),
        11 => g11::build(&mut cx),
        12 => g12::build(&mut cx),
        13 => g13::build(&mut cx),
        _ => false,
    };
    if built {
        cx.out
    } else {
        Vec::new()
    }
}
