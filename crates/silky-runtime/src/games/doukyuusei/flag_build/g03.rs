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
const FIRST_UNDONE: u16 = 298;
const SECOND_UNDONE: u16 = 299;

pub(super) fn build(cx: &mut Ctx) -> bool {
    let hanaya = ev("hanaya.mes", 278, 0, 0, -1);
    let r = if cx.set(170) {
        cx.done(hanaya, 486, 487)
    } else {
        let d = if cx.clock.date < 814 {
            [810, 813, 900, 1855]
        } else {
            [817, 830, 900, 1855]
        };
        let mut r = Ctx::rec(hanaya, d);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 170);

    let koutei1 = ev("koutei1.mes", 271, 0, 3, -1);
    let r = if cx.set(271) {
        cx.done(koutei1, 488, 489)
    } else {
        // A late start locks the later events, but this event failing to
        // fit does not.
        if cx.prev(1).d0.max(814) > 816 {
            cx.locked = true;
        }
        let keep = cx.locked;
        let mut r = Ctx::rec(koutei1, [814, 816, 830, 1755]);
        cx.fit(&mut r);
        cx.locked = keep;
        r
    };
    cx.push(r, 271);

    // The later events only look at these two.
    let closed = cx.byte(729) != 0 || cx.byte(733) != 0;
    cx.locked = closed;
    let clear = cx.set(733) || cx.set(729);
    let housou1 = ev("housou1.mes", 279, 4, 0, -1);
    let mut r = if cx.set(274) {
        cx.done(housou1, 490, 491)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(housou1, [d0, 830, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    if clear {
        r.reason = 0;
    }
    cx.push(r, 274);

    let koutei2 = ev("koutei2.mes", 280, 0, 3, 8002);
    let mut r = if cx.set(277) {
        cx.done(koutei2, 492, 493)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(koutei2, [d0, 830, 830, 1730]);
        cx.fit(&mut r);
        r
    };
    if r.d0 >= 817 {
        r.script = "koutei.mes";
    }
    if clear {
        r.reason = 0;
    }
    cx.push(r, 277);

    if cx.byte(223) == 0 || cx.byte(313) == 0 || cx.set(1242) {
        cx.locked = true;
    }
    let eiga4 = Ev {
        cost: 36,
        ..ev("eiga4.mes", 281, 1, 0, -1)
    };
    let mut r = if cx.set(323) && cx.set(324) {
        cx.done(eiga4, 494, 495)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(eiga4, [d0, 830, 1030, 1840]);
        cx.fit(&mut r);
        r
    };
    if clear {
        r.reason = 0;
    } else if cx.byte(223) == 0 {
        r.reason = FIRST_UNDONE;
    } else if cx.byte(313) == 0 {
        r.reason = SECOND_UNDONE;
    }
    cx.push(r, 323);
    cx.b[324] = 1;

    if cx.byte(685) == 0 {
        cx.locked = true;
    }
    let koutei = ev("koutei.mes", 280, 0, 3, 8002);
    let mut r = if cx.set(716) {
        cx.done(koutei, 496, 497)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(821);
        let mut r = Ctx::rec(koutei, [d0, 829, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    chain_reason(cx, &mut r, clear);
    cx.push(r, 716);

    let shut = clear || cx.set(8020) || cx.set(711);
    if cx.set(8020) || cx.set(711) {
        cx.locked = true;
    }
    let myhou16 = ev("myhou16.mes", 270, 0, 0, -1);
    let mut r = if cx.set(731) && cx.set(728) && cx.set(710) {
        cx.done(myhou16, 498, 499)
    } else {
        let d = if cx.clock.date == 830 {
            [830, -1, 1905, 2055]
        } else {
            let d0 = (cx.prev(1).d0 + 1).max(827);
            [d0, 829, 1905, 2355]
        };
        let mut r = Ctx::rec(myhou16, d);
        cx.fit(&mut r);
        r
    };
    chain_reason(cx, &mut r, shut);
    cx.push(r, 731);
    cx.b[728] = 1;
    cx.b[710] = 1;
    true
}

/// The reason line of the last events: none when `clear`, else the first
/// of the earlier events that is not done.
fn chain_reason(cx: &Ctx, r: &mut crate::games::doukyuusei::flaglist::Record, clear: bool) {
    if clear {
        r.reason = 0;
    } else if cx.byte(223) == 0 {
        r.reason = FIRST_UNDONE;
    } else if cx.byte(313) == 0 {
        r.reason = SECOND_UNDONE;
    } else if cx.byte(685) == 0 {
        r.reason = FIRST_UNDONE;
    }
}
