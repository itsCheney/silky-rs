use super::{Ctx, Ev};

const fn ev(script: &'static str, place: u16, stage: i32, sub: i32, f0: i32) -> Ev {
    Ev {
        script,
        place,
        stage,
        sub,
        cost: 0,
        f0,
    }
}

pub(super) fn build(cx: &mut Ctx) -> bool {
    // (event, byte, day word, time word)
    let pool = ev("pool.mes", 265, 0, 4, -1);
    let r = if cx.set(247) {
        cx.done(pool, 450, 460)
    } else {
        let mut r = Ctx::rec(pool, [810, 830, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 247);

    // Later events wait while one of these is set.
    if [760, 773, 753, 1448].iter().any(|&i| cx.set(i)) {
        cx.locked = true;
    }
    let pool1 = ev("pool1.mes", 265, 0, 4, -1);
    let r = if cx.set(248) {
        cx.done(pool1, 451, 461)
    } else {
        let mut r = Ctx::rec(pool1, [810, 829, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 248);

    let pool2 = ev("pool2.mes", 265, 0, 4, -1);
    let r = if cx.set(305) {
        cx.done(pool2, 452, 462)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(812);
        let mut r = Ctx::rec(pool2, [d0, 830, 900, 1555]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 305);

    let kouens3 = ev("kouens3.mes", 266, 0, 0, -1);
    let r = if cx.set(306) {
        cx.done(kouens3, 453, 463)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(812);
        let mut r = Ctx::rec(kouens3, [d0, 829, 805, 1955]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 306);

    let myhous4 = ev("myhous4.mes", 267, 0, 0, -1);
    let r = if cx.set(308) {
        cx.done(myhous4, 454, 464)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(myhous4, [d0, 829, 805, 2055]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 308);

    let pool7 = ev("pool7.mes", 265, 0, 4, -1);
    let r = if cx.set(745) {
        cx.done(pool7, 455, 465)
    } else {
        let d0 = (cx.prev(2).d0 + 1).max(818);
        let mut r = Ctx::rec(pool7, [d0, 823, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 745);

    let yabuki = ev("yabuki.mes", 268, 1, 2, 8002);
    let r = if cx.set(746) {
        cx.done(yabuki, 456, 466)
    } else {
        let mut r = Ctx::rec(yabuki, [824, -1, 955, 1045]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 746);
    cx.b[762] = 1;

    let hotel5 = ev("hotel5.mes", 269, 0, 0, -1);
    let r = if cx.set(765) {
        cx.done(hotel5, 457, 467)
    } else {
        let mut r = Ctx::rec(hotel5, [827, 830, 1800, 2355]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 765);

    let myhou17 = ev("myhou17.mes", 267, 0, 0, -1);
    let r = if cx.set(771) {
        cx.done(myhou17, 458, 194)
    } else {
        let d0 = cx.prev(1).d0.max(828);
        let mut r = Ctx::rec(myhou17, [d0, 830, 805, 2355]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 771);

    let v05 = ev("v05.mes", 267, 0, 0, -1);
    let r = if cx.set(8022) {
        cx.done(v05, 459, 469)
    } else {
        let p = cx.prev(1);
        let d0 = if p.t0 < 2300 { p.d0 } else { p.d0 + 1 };
        let mut r = Ctx::rec(v05, [d0, 830, 2105, 2355]);
        cx.fit(&mut r);
        // Not while her mood is under 11 on a day it could happen.
        let day = cx.clock.date;
        if r.d0 <= day && day <= r.d1 && cx.byte(111) < 11 {
            r.state = 2;
        }
        r
    };
    cx.push(r, 8022);
    true
}
