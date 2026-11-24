use super::{sort_into, Ctx, Ev};
use crate::games::doukyuusei::flaglist::Record;

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

/// The first two events, built on their own and sorted into `group`.
/// Returns whether the lock was set while building them; the lock itself
/// is left as it was.
fn first_events(cx: &mut Ctx, group: &mut Vec<Record>) -> bool {
    let start = cx.locked;
    let mut sub = Vec::new();

    let tim_in = ev("tim_in.mes", 275, 0, -1);
    let r = if cx.set(182) {
        cx.done(tim_in, 300, 301)
    } else {
        let mut r = Ctx::rec(tim_in, [810, 816, 1000, 1855]);
        cx.fit(&mut r);
        r
    };
    sub.push(r);
    cx.b[182] = 1;

    let kaisha = ev("kaisha.mes", 291, 0, 8002);
    let r = if cx.set(152) {
        cx.done(kaisha, 302, 303)
    } else {
        let mut r = Ctx::rec(kaisha, [810, 830, 700, 1855]);
        cx.fit(&mut r);
        r
    };
    sub.push(r);
    cx.b[152] = 1;

    sort_into(sub, group);
    let ret = cx.locked;
    cx.locked = start;
    ret
}

pub(super) fn build(cx: &mut Ctx) -> bool {
    // Waiting events are out of reach until the list is open, and after
    // day 816 while the first company visit is not done.
    cx.locked = !cx.set(8013);
    if cx.clock.date > 816 && cx.byte(264) == 0 {
        cx.locked = true;
    }
    let mut group = Vec::new();
    if first_events(cx, &mut group) {
        cx.locked = true;
    }

    let kaisha1 = ev("kaisha1.mes", 291, 0, -1);
    let r = if cx.set(183) {
        cx.done(kaisha1, 304, 305)
    } else {
        let p = Ctx::back(&group, 1).d0;
        let date = cx.clock.date;
        let (d0, d1) = if date < 816 {
            (p.max(811), 815)
        } else if date < 824 {
            (p.max(818), 822)
        } else {
            (824, 829)
        };
        let mut r = Ctx::rec(kaisha1, [d0, d1, 905, 1755]);
        // Her lock is not kept from this one.
        let lock = cx.locked;
        cx.fit(&mut r);
        cx.locked = lock;
        r
    };
    group.push(r);
    cx.b[281] = 1;
    sort_into(group, &mut cx.out);

    cx.locked = cx.set(8025);
    let kaisha2 = ev("kaisha2.mes", 291, 0, -1);
    let r = if cx.set(264) {
        cx.done(kaisha2, 306, 307)
    } else {
        let d0 = cx.prev(1).d0.max(812);
        let mut r = Ctx::rec(kaisha2, [d0, 816, 1005, 1655]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 264);

    let kouenb1 = ev("kouenb1.mes", 268, 1, -1);
    let r = if cx.set(401) {
        cx.done(kouenb1, 308, 309)
    } else {
        let mut r = Ctx::rec(kouenb1, [817, -1, 1035, 1155]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 401);

    let myhous5 = ev("myhous5.mes", 270, 0, -1);
    let r = if cx.set(403) {
        cx.done(myhous5, 310, 311)
    } else {
        let d0 = (cx.prev(1).d0 + 1).max(818);
        let mut r = Ctx::rec(myhous5, [d0, 830, 805, 2355]);
        cx.fit(&mut r);
        r
    };
    cx.push(r, 403);

    let myhous6 = ev("myhous6.mes", 270, 0, -1);
    let r = if cx.set(406) {
        cx.done(myhous6, 312, 313)
    } else {
        let mut r = if cx.clock.date == 830 {
            Ctx::rec(myhous6, [830, -1, 1800, 2055])
        } else {
            let d0 = (cx.prev(1).d0 + 1).max(819);
            Ctx::rec(myhous6, [d0, 830, 1800, 2355])
        };
        cx.fit(&mut r);
        r
    };
    // The byte this sets is the one just before the one it reads.
    cx.push(r, 405);
    true
}
