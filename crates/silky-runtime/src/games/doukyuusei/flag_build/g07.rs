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

/// The text line that says an earlier event is not done.
const NOT_YET: u16 = 300;

pub(super) fn build(cx: &mut Ctx) -> bool {
    if cx.byte(412) == 0 {
        cx.locked = true;
    }

    let sakima3 = ev("sakima3.mes", 283, 0, 1, -1);
    let mut r = if cx.set(463) {
        cx.done(sakima3, 532, 533)
    } else {
        let mut r = Ctx::rec(sakima3, [817, 819, 1005, 1555]);
        cx.fit(&mut r);
        r
    };
    // Looked at before the event's byte is set.
    if cx.byte(412) == 0 || cx.byte(463) == 0 {
        r.reason = NOT_YET;
    }
    cx.push(r, 463);
    cx.b[464] = 1;

    if cx.set(8023) {
        cx.locked = true;
    }
    let closed = cx.set(8023) || cx.set(1449) || cx.set(1450);

    let eiga5 = ev("eiga5.mes", 281, 1, 0, -1);
    let mut r = if cx.set(466) {
        cx.done(eiga5, 534, 535)
    } else {
        let mut r = Ctx::rec(eiga5, [820, -1, 1500, 1755]);
        cx.fit(&mut r);
        r
    };
    if closed {
        r.reason = 0;
    } else if cx.byte(412) == 0 || cx.byte(463) == 0 {
        r.reason = NOT_YET;
    }
    cx.push(r, 465);

    let chihar = ev("chihar.mes", 287, 0, 0, -1);
    let mut r = if cx.set(467) {
        cx.done(chihar, 536, 537)
    } else {
        let mut r = Ctx::rec(chihar, [822, -1, 1230, 1330]);
        cx.fit(&mut r);
        r
    };
    if closed {
        r.reason = 0;
    } else if cx.byte(412) == 0 || cx.byte(463) == 0 {
        r.reason = NOT_YET;
    }
    cx.push(r, 467);
    true
}
