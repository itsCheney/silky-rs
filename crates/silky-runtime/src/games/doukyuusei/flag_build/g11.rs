use super::{sort_into, Ctx, Ev};

/// The text line that says why an event is out of reach.
const REASON: u16 = 304;

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

/// The reason line the later events carry: cleared until byte 235 is set,
/// then set when byte 481 (and for the last ones also 486) is not, and
/// otherwise left as the fit made it.
fn reason_early(cx: &Ctx) -> Option<u16> {
    if cx.byte(235) == 0 {
        Some(0)
    } else if cx.byte(481) == 0 {
        Some(REASON)
    } else {
        None
    }
}

fn reason_late(cx: &Ctx) -> Option<u16> {
    if cx.byte(235) == 0 {
        Some(0)
    } else if cx.byte(481) == 0 || cx.byte(486) == 0 {
        Some(REASON)
    } else {
        None
    }
}

/// The two events built on their own after the hospital one, with the
/// first day not before `from`.
fn side_events(cx: &mut Ctx, from: i32) -> Vec<super::super::flaglist::Record> {
    let mut sub = Vec::new();
    let kouen_s = ev("kouen_s.mes", 266, 0, 0, 8002);
    let reason = reason_early(cx);
    let mut r = if cx.set(499) {
        cx.done(kouen_s, 318, 319)
    } else {
        let mut r = Ctx::rec(kouen_s, [from.max(819), 830, 805, 1955]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    sub.push(r);
    cx.b[499] = 1;

    let yabuki7 = ev("yabuki7.mes", 268, 1, 2, -1);
    let mut r = if cx.set(489) {
        cx.done(yabuki7, 320, 321)
    } else {
        let mut r = Ctx::rec(yabuki7, [(from + 1).max(820), 830, 805, 1955]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    sub.push(r);
    cx.b[489] = 1;
    sub
}

pub(super) fn build(cx: &mut Ctx) -> bool {
    cx.locked = !cx.set(8013);

    let sakima1 = ev("sakima1.mes", 283, 0, 1, -1);
    let r = if cx.set(151) {
        cx.done(sakima1, 314, 315)
    } else {
        let mut r = Ctx::rec(sakima1, [810, -1, 1055, 1135]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 151);
    if !cx.locked {
        cx.b[235] = 1;
    }

    if cx.byte(235) == 0 || cx.byte(481) == 0 {
        cx.locked = true;
    }
    let hospta3 = ev("hospta3.mes", 290, 0, 0, -1);
    let reason = reason_early(cx);
    let mut r = if cx.set(488) {
        cx.done(hospta3, 316, 317)
    } else {
        let mut r = Ctx::rec(hospta3, [819, 830, 805, 1955]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 488);

    // The side events keep a lock of their own, then pass on whether it
    // was set.
    let main_lock = cx.locked;
    cx.locked = cx.byte(8013) != 1 || cx.byte(235) == 0 || cx.byte(481) == 0;
    let from = cx.prev(1).d0;
    let sub = side_events(cx, from);
    let sub_lock = cx.locked;
    cx.locked = main_lock || sub_lock;
    sort_into(sub, &mut cx.out);

    let kaisha5 = ev("kaisha5.mes", 291, 0, 0, -1);
    let reason = reason_early(cx);
    let mut r = if cx.set(490) {
        cx.done(kaisha5, 322, 323)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(821);
        let mut r = Ctx::rec(kaisha5, [d0, 830, 1005, 1455]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 490);

    if cx.byte(486) == 0 {
        cx.locked = true;
    }
    let kouenb4 = ev("kouenb4.mes", 268, 1, 0, -1);
    let reason = reason_late(cx);
    let mut r = if cx.set(491) {
        cx.done(kouenb4, 324, 325)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(822);
        let mut r = Ctx::rec(kouenb4, [d0, 830, 1005, 1855]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 491);

    let timtim5 = ev("timtim5.mes", 275, 0, 0, -1);
    let reason = reason_late(cx);
    let mut r = if cx.set(513) {
        cx.done(timtim5, 326, 327)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(822);
        let mut r = Ctx::rec(timtim5, [d0, 826, 1005, 1855]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 513);

    if cx.byte(515) == 1 {
        cx.locked = true;
    }
    let yabuki = ev("yabuki.mes", 268, 1, 2, 8011);
    // Once that byte is set the fit's own reason is dropped.
    let reason = reason_late(cx).or((cx.byte(515) == 1).then_some(0));
    let mut r = if cx.set(539) {
        cx.done(yabuki, 328, 329)
    } else {
        let mut r = Ctx::rec(yabuki, [827, -1, 945, 1035]);
        cx.fit(&mut r);
        r
    };
    if let Some(x) = reason {
        r.reason = x;
    }
    cx.push(r, 539);
    true
}
