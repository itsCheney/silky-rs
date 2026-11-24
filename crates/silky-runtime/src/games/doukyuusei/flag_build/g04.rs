use super::{sort_into, Ctx, Ev, UNLOCKED};
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

/// Adds the event `e` to `dest`: done from its story words, or waiting
/// with the days `d` makes from the records already in `dest`.
fn add(
    cx: &mut Ctx,
    dest: &mut Vec<Record>,
    e: Ev,
    flag: usize,
    words: (usize, usize),
    d: impl FnOnce(&Vec<Record>) -> [i32; 4],
) {
    let r = if cx.set(flag) {
        cx.done(e, words.0, words.1)
    } else {
        let mut r = Ctx::rec(e, d(dest));
        cx.fit(&mut r);
        r
    };
    dest.push(r);
    cx.b[flag] = 1;
}

/// Runs a sub-builder: it starts from the list's opening state alone, and
/// its outcome joins the girl's lock.
fn sub(cx: &mut Ctx, f: impl FnOnce(&mut Ctx)) {
    let saved = cx.locked;
    cx.locked = cx.b[UNLOCKED] != 1;
    f(cx);
    cx.locked |= saved;
}

/// The last day of the record before, at least `min`.
fn last_day(v: &[Record], min: i32) -> i32 {
    Ctx::back(v, 1).d0.max(min)
}

fn first_pair(cx: &mut Ctx, dest: &mut Vec<Record>, from: i32) {
    let kouens8 = ev("kouens8.mes", 266, 0, 0, -1);
    add(cx, dest, kouens8, 501, (578, 579), |_| {
        [from.max(818), 830, 805, 1755]
    });
    let myhous9 = ev("myhous9.mes", 270, 0, 0, -1);
    add(cx, dest, myhous9, 483, (582, 583), |_| {
        [from.max(819), 830, 805, 2355]
    });
}

fn second_pair(cx: &mut Ctx, dest: &mut Vec<Record>, from: i32) {
    let yabuki8 = ev("yabuki8.mes", 268, 1, 2, -1);
    add(cx, dest, yabuki8, 502, (580, 581), |_| {
        [from.max(819), 829, 805, 1455]
    });
    let timtim = ev("timtim.mes", 275, 0, 0, 8002);
    add(cx, dest, timtim, 492, (584, 585), |v| {
        [last_day(v, 819), 830, 805, 1855]
    });
    let okujyo4 = ev("okujyo4.mes", 284, 5, 0, -1);
    add(cx, dest, okujyo4, 484, (586, 587), |v| {
        [(Ctx::back(v, 1).d0 + 1).max(820), 830, 830, 1755]
    });
}

pub(super) fn build(cx: &mut Ctx) -> bool {
    let mut out = Vec::new();

    let timtim1 = ev("timtim1.mes", 275, 0, 0, -1);
    add(cx, &mut out, timtim1, 250, (574, 575), |_| {
        [812, 830, 1005, 1855]
    });
    if cx.set(235) {
        let butick = ev("butick.mes", 282, 0, 0, 8002);
        add(cx, &mut out, butick, 480, (330, 331), |v| {
            [last_day(v, 817), 830, 1000, 1855]
        });
    }
    let sakima4 = ev("sakima4.mes", 283, 0, 1, -1);
    add(cx, &mut out, sakima4, 481, (576, 577), |v| {
        [last_day(v, 818), 830, 805, 1755]
    });

    // A group of events, sorted before it is added.
    let mut group = Vec::new();
    let from = last_day(&out, 0);
    let mut part = Vec::new();
    sub(cx, |cx| first_pair(cx, &mut part, from));
    sort_into(part, &mut group);
    let from = last_day(&group, 0);
    let mut part = Vec::new();
    sub(cx, |cx| second_pair(cx, &mut part, from));
    sort_into(part, &mut group);

    let kouens7 = ev("kouens7.mes", 266, 0, 0, -1);
    add(cx, &mut group, kouens7, 485, (588, 589), |v| {
        [last_day(v, 821), 830, 805, 2355]
    });
    let eiga6 = ev("eiga6.mes", 281, 1, 0, -1);
    add(cx, &mut group, eiga6, 486, (590, 591), |v| {
        [last_day(v, 822), 830, 1005, 1855]
    });
    sort_into(group, &mut out);

    // Later events wait while one of these is set.
    if cx.set(544) || cx.set(8018) {
        cx.locked = true;
    }
    let myhou11 = ev("myhou11.mes", 270, 0, 0, -1);
    let late = cx.clock.date >= 829;
    add(cx, &mut out, myhou11, 515, (592, 593), |v| {
        if late {
            [last_day(v, 829), 830, 1805, 2055]
        } else {
            [last_day(v, 825), 828, 1805, 2155]
        }
    });
    cx.out = out;
    true
}
