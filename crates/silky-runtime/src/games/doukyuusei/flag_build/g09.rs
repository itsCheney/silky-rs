use super::{Ctx, Ev, Record, UNLOCKED};

const fn ev(script: &'static str, place: u16, cost: i32, f0: i32) -> Ev {
    Ev {
        script,
        place,
        stage: 0,
        sub: 0,
        cost,
        f0,
    }
}

/// Gives `r` the reason `id` unless there is none, keeping the cash one.
fn why(r: &mut Record, id: u16) {
    if id != 0 {
        r.reason = id;
    }
}

pub(super) fn build(cx: &mut Ctx) -> bool {
    // The first six events are gathered, sorted and then added.
    let mut group: Vec<Record> = Vec::new();

    let hospta = ev("hospta.mes", 290, 0, -1);
    let r = if cx.set(190) {
        cx.done(hospta, 564, 565)
    } else {
        let mut r = Ctx::rec(hospta, [810, 830, 905, 1855]);
        cx.fit(&mut r);
        r
    };
    group.push(r);
    cx.b[190] = 1;

    let r = if cx.set(281) {
        cx.done(hospta, 566, 567)
    } else {
        let d0 = Ctx::back(&group, 1).d0.max(811);
        let mut r = Ctx::rec(hospta, [d0, 830, 905, 1855]);
        cx.fit(&mut r);
        r
    };
    group.push(r);
    cx.b[281] = 1;

    // Later events wait while this is not set.
    if !cx.set(600) {
        cx.locked = true;
    }
    let sinryo = ev("sinryo.mes", 290, 0, -1);
    let mut r = if cx.set(598) {
        cx.done(sinryo, 568, 569)
    } else {
        let d0 = (Ctx::back(&group, 1).d0 + 1).max(820);
        let mut r = Ctx::rec(sinryo, [d0, 830, 905, 1855]);
        cx.fit(&mut r);
        r
    };
    if !cx.set(600) {
        why(&mut r, 296);
    }
    group.push(r);
    cx.b[598] = 1;

    let hospta5 = ev("hospta5.mes", 290, 0, -1);
    let mut r = if cx.set(609) {
        cx.done(hospta5, 570, 571)
    } else {
        let d0 = Ctx::back(&group, 1).d0;
        let mut r = Ctx::rec(hospta5, [d0, 830, 1905, 2355]);
        cx.fit(&mut r);
        r
    };
    if !cx.set(600) {
        why(&mut r, 296);
    }
    group.push(r);
    cx.b[609] = 1;

    if cx.set(604) {
        cx.locked = true;
    }
    let sinryo1 = ev("sinryo1.mes", 290, 0, -1);
    // The story's own byte, as it was before this list set it.
    let was_617 = cx.set(617);
    let mut r = if cx.set(617) {
        cx.done(sinryo1, 572, 573)
    } else {
        let d0 = (Ctx::back(&group, 1).d0 + 1).max(824);
        let mut r = Ctx::rec(sinryo1, [d0, 830, 905, 1855]);
        cx.fit(&mut r);
        r
    };
    // While that byte is set the reason is cleared, the cash one too.
    if cx.set(604) {
        r.reason = 0;
    } else if !cx.set(600) {
        why(&mut r, 296);
    }
    group.push(r);
    cx.b[617] = 1;
    let after_group = cx.locked;

    // This one is out of reach by its own rules, not those before it.
    let town = ev("town1.mes", 278, 0, 8017);
    let done_town = cx.set(727);
    let mut r = if done_town {
        cx.done(town, 334, 335)
    } else {
        cx.locked = cx.byte(UNLOCKED) != 1 || !cx.set(716);
        let d0 = if cx.set(716) { cx.h(496) } else { 821 };
        let mut r = Ctx::rec(town, [d0, 830, 900, 1855]);
        cx.fit(&mut r);
        r
    };
    if !cx.set(716) {
        why(&mut r, 295);
    }
    let lock_after = if done_town { after_group } else { cx.locked };
    group.push(r);
    cx.b[727] = 1;

    // Done first, then waiting, then out of reach, each by first day.
    group.sort_by_key(|r| (r.state != 1, r.state, r.d0));
    cx.out.extend(group);

    cx.locked = lock_after;
    if cx.locked && was_617 {
        cx.locked = false;
    }

    let town = ev("town1.mes", 278, 50, 8017);
    let mut r = if cx.set(1235) {
        cx.done(town, 336, 337)
    } else {
        let d0 = cx.prev(1).d0.max(824);
        let mut r = Ctx::rec(town, [d0, 830, 900, 1855]);
        cx.fit(&mut r);
        r
    };
    if was_617 {
        if !cx.set(716) {
            why(&mut r, 295);
        } else if !cx.set(600) {
            why(&mut r, 296);
        }
    } else if cx.set(604) {
        r.reason = 0;
    } else if !cx.set(716) {
        why(&mut r, 295);
    } else if !cx.set(600) {
        why(&mut r, 296);
    }
    cx.out.push(r);
    cx.b[1235] = 1;
    true
}
