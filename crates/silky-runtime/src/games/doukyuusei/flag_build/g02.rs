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

/// Text lines that say why an event is out of reach.
const NOT_YET: u16 = 297;
const TOO_LOW: u16 = 305;

pub(super) fn build(cx: &mut Ctx) -> bool {
    let tim_in = ev("tim_in.mes", 275, 0, 0, -1);
    let r = if cx.set(412) {
        cx.done(tim_in, 476, 477)
    } else {
        let mut r = Ctx::rec(tim_in, [817, 830, 1000, 1900]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 412);

    let town1 = ev("town1.mes", 273, 0, 0, 8007);
    let r = if cx.set(413) {
        cx.done(town1, 478, 479)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(town1, [d0, 830, 1545, 1745]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 413);

    // These two close the list for what follows.
    let closed = cx.set(765) || cx.set(760);
    if closed {
        cx.locked = true;
    }
    let gakko2 = ev("gakko2.mes", 276, 3, 0, 8002);
    let mut r = if cx.set(414) {
        cx.done(gakko2, 480, 481)
    } else {
        let d0 = cx.prev(1).d0 + 1;
        let mut r = Ctx::rec(gakko2, [d0, 830, 830, 1730]);
        cx.fit(&mut r);
        r
    };
    if closed {
        r.reason = 0;
    }
    cx.push(r, 414);

    if cx.byte(633) == 0 {
        cx.locked = true;
    }
    let myroom = ev("myroom.mes", 277, 0, 0, -1);
    let mut r = if cx.set(417) {
        cx.done(myroom, 482, 483)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(820);
        let mut r = Ctx::rec(myroom, [d0, 829, 2105, 2340]);
        cx.fit(&mut r);
        r
    };
    if closed {
        r.reason = 0;
    } else if cx.byte(633) == 0 {
        r.reason = NOT_YET;
    }
    cx.push(r, 417);

    let shut = cx.set(8026);
    if shut || cx.byte(105) < 8 {
        cx.locked = true;
    }
    let kouens5 = ev("kouens5.mes", 266, 0, 0, -1);
    let mut r = if cx.set(419) {
        cx.done(kouens5, 484, 485)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(kouens5, [d0, -1, 2120, 2355]);
        cx.fit(&mut r);
        r
    };
    if closed || shut {
        r.reason = 0;
    } else if cx.byte(105) < 8 {
        r.reason = TOO_LOW;
    } else if cx.byte(633) == 0 {
        r.reason = NOT_YET;
    }
    cx.push(r, 419);
    true
}
