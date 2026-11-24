use super::{Ctx, Ev};
use crate::games::doukyuusei::flaglist::Record;

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
const NOT_YET: u16 = 295;
const NOT_NOW: u16 = 296;

pub(super) fn build(cx: &mut Ctx) -> bool {
    if cx.byte(710) != 0 {
        cx.locked = true;
    }

    let l02 = ev("l02.mes", 271, 0, 3, -1);
    let r = if cx.set(223) {
        cx.done(l02, 470, 471)
    } else {
        let d = if cx.clock.date < 814 {
            [810, 814, 830, 1755]
        } else {
            [817, 830, 830, 1755]
        };
        let mut r = Ctx::rec(l02, d);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 223);

    if cx.byte(324) == 0 {
        cx.locked = true;
    }
    let kyoshi = ev("kyoshi.mes", 272, 5, 0, 8002);
    let mut r = if cx.set(685) {
        cx.done(kyoshi, 472, 473)
    } else {
        let d0 = cx.prev(1).d0.max(814);
        let mut r = Ctx::rec(kyoshi, [d0, 830, 830, 1755]);
        cx.fit(&mut r);
        r
    };
    if cx.byte(710) == 1 {
        r.reason = 0;
    } else if cx.byte(324) == 0 {
        r.reason = NOT_YET;
    }
    cx.push(r, 685);

    if cx.byte(462) == 0 {
        cx.locked = true;
    }
    let town1 = ev("town1.mes", 273, 0, 0, 8006);
    let mut r = if cx.set(690) {
        cx.done(town1, 474, 475)
    } else {
        let d0 = cx.prev(1).d0.max(818);
        let mut r = Ctx::rec(town1, [d0, 820, 835, 1630]);
        cx.fit(&mut r);
        r
    };
    reason(cx, &mut r, false);
    cx.push(r, 690);

    // Two more bytes close the list for the last event.
    let closed = cx.set(8024) || cx.set(8027);
    if closed {
        cx.locked = true;
    }
    let hoken6 = ev("hoken6.mes", 274, 2, 0, -1);
    let mut r = if cx.set(711) {
        cx.done(hoken6, 332, 333)
    } else {
        let d = if cx.byte(708) == 0 {
            let d0 = cx.prev(1).d0.max(826);
            [d0, 830, 1015, 1600]
        } else {
            [cx.h(151), -1, 1015, 1600]
        };
        let mut r = Ctx::rec(hoken6, d);
        cx.fit(&mut r);
        r
    };
    reason(cx, &mut r, closed);
    cx.push(r, 711);
    true
}

/// The line that says why a later event is out of reach: none once the
/// first one is set, else one of two lines while the first events wait.
fn reason(cx: &Ctx, r: &mut Record, clear: bool) {
    if cx.byte(710) == 1 || clear {
        r.reason = 0;
    } else if cx.byte(324) == 0 {
        r.reason = NOT_YET;
    } else if cx.byte(462) == 0 {
        r.reason = NOT_NOW;
    }
}
