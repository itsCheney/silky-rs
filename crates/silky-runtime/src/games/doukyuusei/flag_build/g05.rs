use super::Record;
use super::{Ctx, Ev};

const fn ev(script: &'static str, place: u16, f0: i32) -> Ev {
    Ev {
        script,
        place,
        stage: 0,
        sub: 0,
        cost: 0,
        f0,
    }
}

/// Why an event is out of reach: the first of her conditions not met.
fn reason(cx: &Ctx, with_second_pair: bool) -> u16 {
    if !cx.set(170) {
        295
    } else if !cx.set(253) {
        296
    } else if with_second_pair && !cx.set(413) {
        300
    } else if with_second_pair && !cx.set(462) {
        296
    } else {
        0
    }
}

/// Gives `r` the reason `id` unless there is none, keeping the cash one.
fn why(r: &mut Record, id: u16) {
    if id != 0 {
        r.reason = id;
    }
}

pub(super) fn build(cx: &mut Ctx) -> bool {
    let madcin = ev("madcin.mes", 285, -1);
    let r = if cx.set(171) {
        cx.done(madcin, 500, 501)
    } else {
        let mut r = Ctx::rec(madcin, [810, 830, 900, 1855]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 171);

    // Later events wait while one of these is not set.
    if !cx.set(170) || !cx.set(253) {
        cx.locked = true;
    }
    let madcin1 = ev("madcin1.mes", 285, -1);
    let mut r = if cx.set(311) {
        cx.done(madcin1, 502, 503)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(813);
        let mut r = Ctx::rec(madcin1, [d0, 816, 1005, 1355]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, false));
    cx.push(r, 311);
    cx.b[313] = 1;
    cx.b[314] = 1;
    cx.b[317] = 1;

    let madcin2 = ev("madcin2.mes", 285, -1);
    let mut r = if cx.set(399) {
        cx.done(madcin2, 504, 505)
    } else {
        let d0 = cx.prev(1).d0.max(813);
        let mut r = Ctx::rec(madcin2, [d0, 830, 900, 1855]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, false));
    cx.push(r, 399);

    let madcin_b = ev("madcin.mes", 285, 8002);
    let mut r = if cx.set(460) {
        cx.done(madcin_b, 506, 507)
    } else {
        let d0 = cx.prev(1).d0.max(818);
        let mut r = Ctx::rec(madcin_b, [d0, 830, 1005, 1355]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, false));
    cx.push(r, 460);

    if !cx.set(413) || !cx.set(462) {
        cx.locked = true;
    }
    let madcin3 = ev("madcin3.mes", 285, -1);
    let mut r = if cx.set(663) || cx.set(664) {
        cx.done(madcin3, 508, 509)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(824);
        let mut r = Ctx::rec(madcin3, [d0, 830, 905, 1455]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, true));
    cx.push(r, 663);

    let town1 = ev("town1.mes", 273, 8008);
    let mut r = if cx.set(664) {
        cx.done(town1, 510, 511)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(town1, [d0, 830, 0, 2355]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, true));
    cx.push(r, 664);

    let madcin_c = ev("madcin.mes", 285, -1);
    let mut r = if cx.set(665) {
        cx.done(madcin_c, 512, 513)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(madcin_c, [d0, 830, 900, 1855]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, true));
    cx.push(r, 665);

    if cx.set(604) {
        cx.locked = true;
    }
    let madcin4 = ev("madcin4.mes", 285, -1);
    let mut r = if cx.set(603) || cx.set(1231) {
        cx.done(madcin4, 514, 515)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(826);
        let mut r = Ctx::rec(madcin4, [d0, 830, 900, 1855]);
        cx.fit(&mut r);
        r
    };
    // While that byte is set the reason is cleared, the cash one too.
    if cx.set(604) {
        r.reason = 0;
    } else {
        why(&mut r, reason(cx, true));
    }
    cx.push(r, 603);
    cx.b[1231] = 1;
    true
}
