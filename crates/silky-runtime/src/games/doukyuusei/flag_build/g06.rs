use super::{Ctx, Ev, Record};

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

/// Gives `r` the reason `id` unless there is none, keeping the cash one.
fn why(r: &mut Record, id: u16) {
    if id != 0 {
        r.reason = id;
    }
}

/// Why an event is out of reach: the first of her conditions not met.
fn reason(cx: &Ctx, with_third: bool) -> u16 {
    if !cx.set(311) || !cx.set(460) {
        299
    } else if with_third && !cx.set(578) {
        301
    } else {
        0
    }
}

pub(super) fn build(cx: &mut Ctx) -> bool {
    let hoken1 = ev("hoken1.mes", 274, 2, -1);
    let r = if cx.set(221) {
        cx.done(hoken1, 516, 517)
    } else {
        let mut r = Ctx::rec(hoken1, [810, 830, 835, 1755]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 221);

    let r = if cx.set(222) {
        cx.done(hoken1, 518, 519)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(hoken1, [d0, 830, 835, 1755]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 222);

    let gakko2 = ev("gakko2.mes", 286, 3, 8009);
    let r = if cx.set(253) {
        cx.done(gakko2, 520, 521)
    } else {
        let d0 = cx.prev(1).d0;
        let mut r = Ctx::rec(gakko2, [d0, 830, 835, 1755]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 253);

    // Later events wait while one of these is not set.
    if [170, 171, 311, 460].iter().any(|&i| !cx.set(i)) {
        cx.locked = true;
    }
    let hoken3 = ev("hoken3.mes", 274, 2, 8002);
    let mut r = if cx.set(462) {
        cx.done(hoken3, 522, 523)
    } else {
        let d0 = cx.prev(1).d0.max(818);
        let mut r = Ctx::rec(hoken3, [d0, 830, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, false));
    cx.push(r, 462);

    let okujyo6 = ev("okujyo6.mes", 284, 5, -1);
    let mut r = if cx.set(594) {
        cx.done(okujyo6, 524, 525)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(819);
        let mut r = Ctx::rec(okujyo6, [d0, 830, 835, 1555]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, false));
    cx.push(r, 462);

    if !cx.set(578) {
        cx.locked = true;
    }
    let myroom = ev("myroom.mes", 277, 0, 8002);
    let mut r = if cx.set(580) {
        cx.done(myroom, 526, 527)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(820);
        let mut r = Ctx::rec(myroom, [d0, 830, 0, 2355]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, true));
    cx.push(r, 580);

    let hoken4 = ev("hoken4.mes", 274, 2, 8002);
    let mut r = if cx.set(599) {
        cx.done(hoken4, 528, 529)
    } else {
        let d0 = cx.prev(1).d0.max(820);
        let mut r = Ctx::rec(hoken4, [d0, 830, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    why(&mut r, reason(cx, true));
    cx.push(r, 599);
    cx.b[600] = 1;

    if !cx.set(598) || cx.set(1314) || cx.set(603) {
        cx.locked = true;
    }
    let hoken5 = ev("hoken5.mes", 274, 2, -1);
    let mut r = if cx.set(604) {
        cx.done(hoken5, 530, 531)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(820);
        let mut r = Ctx::rec(hoken5, [d0, 830, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    // While one of those two bytes is set the cash reason is cleared.
    if cx.set(1314) || cx.set(603) {
        r.reason = 0;
    }
    let id = match reason(cx, true) {
        0 if !cx.set(598) => 302,
        id => id,
    };
    why(&mut r, id);
    cx.push(r, 604);
    true
}
