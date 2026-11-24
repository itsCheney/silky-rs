use super::{Ctx, Ev};

const fn ev(script: &'static str, place: u16, stage: i32, f0: i32) -> Ev {
    Ev {
        script,
        place,
        stage,
        sub: 0,
        cost: 0,
        f0,
    }
}

/// Text lines that say why an event is out of reach.
const NOT_NOW: u16 = 299;
const NOT_YET: u16 = 300;

pub(super) fn build(cx: &mut Ctx) -> bool {
    cx.locked = !cx.set(8013);

    let shokui = ev("shokui.mes", 292, 2, -1);
    let r = if cx.set(239) {
        cx.done(shokui, 538, 539)
    } else {
        let mut r = Ctx::rec(shokui, [810, 823, 830, 1740]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 239);
    cx.b[463] = 1;
    cx.b[464] = 1;

    if cx.byte(317) == 0 {
        cx.locked = true;
    }
    // Most of the next events carry this reason while byte 317 is unset.
    let reason = (cx.byte(317) == 0).then_some(NOT_NOW);

    let okujyo1 = ev("okujyo1.mes", 284, 5, -1);
    let mut r = if cx.set(318) {
        cx.done(okujyo1, 540, 541)
    } else {
        let p = cx.prev(1).d0;
        let d0 = if p + 1 < 813 { 813 } else { p };
        let mut r = Ctx::rec(okujyo1, [d0, 830, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 318);

    let housou2 = ev("housou2.mes", 279, 4, -1);
    let mut r = if cx.set(319) {
        cx.done(housou2, 542, 543)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(housou2, [d0, 830, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 319);

    let eiga3 = ev("eiga3.mes", 281, 1, -1);
    let mut r = if cx.set(320) {
        cx.done(eiga3, 544, 545)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(eiga3, [d0, 830, 1005, 1855]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 320);

    let reiko2 = ev("reiko2.mes", 293, 0, -1);
    let mut r = if cx.set(321) {
        cx.done(reiko2, 546, 547)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(reiko2, [d0, 830, 805, 1955]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 321);

    let held_back = cx.byte(177) == 0 && cx.byte(413) == 0;
    if held_back {
        cx.locked = true;
    }
    let reason2 = if cx.byte(317) == 0 {
        Some(NOT_NOW)
    } else if held_back {
        Some(NOT_YET)
    } else {
        None
    };

    let c13 = ev("c13.mes", 270, 0, -1);
    let mut r = if cx.set(633) {
        cx.done(c13, 548, 549)
    } else {
        let d0 = cx.prev(1).d0.max(820);
        let mut r = Ctx::rec(c13, [d0, 829, 805, 1755]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason2 {
        r.reason = x;
    }
    cx.push(r, 633);

    if cx.byte(664) == 0 || cx.byte(8021) == 1 {
        cx.locked = true;
    }
    let reason3 = if cx.byte(8021) == 1 {
        Some(0)
    } else if cx.byte(317) == 0 {
        Some(NOT_NOW)
    } else if held_back {
        Some(NOT_YET)
    } else if cx.byte(664) == 0 {
        Some(NOT_NOW)
    } else {
        None
    };

    let town1 = ev("town1.mes", 294, 0, 8010);
    let mut r = if cx.set(641) {
        cx.done(town1, 550, 551)
    } else {
        let d0 = cx.prev(1).d0.max(826);
        let mut r = Ctx::rec(town1, [d0, 829, 1905, 2355]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason3 {
        r.reason = x;
    }
    cx.push(r, 641);
    true
}
