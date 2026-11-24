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
    let myroom = ev("myroom.mes", 270, 0, 0, 8012);
    let r = if cx.set(280) {
        cx.done(myroom, 594, 595)
    } else {
        let mut r = Ctx::rec(myroom, [813, 830, 705, 1655]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 280);

    let reiko4 = ev("reiko4.mes", 293, 0, 0, -1);
    let r = if cx.set(554) {
        cx.done(reiko4, 596, 597)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(815);
        let mut r = Ctx::rec(reiko4, [d0, 830, 805, 1755]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 554);

    let sakima = ev("sakima.mes", 283, 0, 1, 8002);
    let r = if cx.set(555) {
        cx.done(sakima, 598, 599)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(817);
        let mut r = Ctx::rec(sakima, [d0, 830, 805, 1455]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 555);
    true
}
