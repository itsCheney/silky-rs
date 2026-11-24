use super::{Ctx, Ev};

const fn ev(script: &'static str, place: u16, stage: i32, sub: i32) -> Ev {
    Ev {
        script,
        place,
        stage,
        sub,
        cost: 0,
        f0: -1,
    }
}

/// The evening window of the cat visits, or the first hour of the night
/// when the clock is already past it.
fn evening(cx: &Ctx) -> (i32, i32) {
    if cx.clock.time < 2356 {
        (1800, 2355)
    } else {
        (0, 100)
    }
}

pub(super) fn build(cx: &mut Ctx) -> bool {
    // (event, byte, day word, time word)
    let cats = ev("cats.mes", 288, 1, 0);
    let r = if cx.set(200) {
        cx.done(cats, 552, 553)
    } else {
        let (t0, t1) = evening(cx);
        let mut r = Ctx::rec(cats, [810, 830, t0, t1]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 200);

    let cats1 = ev("cats1.mes", 288, 1, 0);
    let r = if cx.set(217) {
        cx.done(cats1, 554, 555)
    } else {
        let d0 = cx.prev(1).d0 + 1;
        let mut r = Ctx::rec(cats1, [d0, 830, 105, 1755]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 217);

    let yabuki2 = ev("yabuki2.mes", 268, 1, 2);
    let r = if cx.set(261) {
        cx.done(yabuki2, 556, 557)
    } else {
        let d0 = cx.prev(1).d0.max(812);
        let mut r = Ctx::rec(yabuki2, [d0, 830, 600, 2155]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 261);

    let r = if cx.set(568) {
        cx.done(cats, 558, 559)
    } else {
        let d0 = cx.prev(1).d0.max(817);
        let (t0, t1) = evening(cx);
        let mut r = Ctx::rec(cats, [d0, 830, t0, t1]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 568);

    let r = if cx.set(566) {
        cx.done(cats, 560, 561)
    } else {
        let d0 = cx.prev(1).d0 + 1;
        let (t0, t1) = evening(cx);
        let mut r = Ctx::rec(cats, [d0, 830, t0, t1]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 566);

    // The cafe waits while either of these is unset.
    if cx.byte(281) == 0 || cx.byte(466) == 0 {
        cx.locked = true;
    }
    let cafe2 = ev("cafe2.mes", 289, 1, 0);
    let mut r = if cx.set(571) {
        cx.done(cafe2, 562, 563)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(820);
        let mut r = Ctx::rec(cafe2, [d0, 829, 1700, 2255]);
        cx.fit(&mut r);
        r
    };
    if cx.byte(281) == 0 {
        r.reason = 302;
    }
    if cx.byte(466) == 0 && cx.byte(465) == 0 {
        r.reason = 303;
    }
    cx.push(r, 571);
    true
}
