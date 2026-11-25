//! The staff roll of 景の海のアペイリア ～カサブランカの騎士～
//! (`Game.990`): a panel of its own that runs for a fixed time.

use silky_vm::{Flow, Vm};

use super::anim::{Key, Look, Sprite, Stage, Step, TICK_MS};
use crate::games::Call;
use crate::host::{FollowUp, Pause, Runtime, Transition};

/// Owner of the roll's sprites on the stage.
const OWNER: u32 = 990;
/// How long the panel runs, in milliseconds (0x4810f0 stops it at 105000).
pub const ROLL_MS: u64 = 105_000;
/// Bit 11 of global 55: a click or the skip mode may end the roll early.
const CAN_SKIP: i32 = 0x800;
/// Height of the strip of credits, and where it starts below the screen.
const STRIP_HEIGHT: i32 = 2640;
const SCREEN_HEIGHT: i32 = 720;

/// The frame a time falls on. The original steps by the clock, not by
/// frames, so every boundary is rounded on its own and the lengths are the
/// differences: no drift builds up over the 105 seconds.
fn at(ms: u64) -> i32 {
    ((ms + TICK_MS / 2) / TICK_MS) as i32
}

/// Frames between two times (at least 1).
fn span(from: u64, to: u64) -> i32 {
    (at(to) - at(from)).max(1)
}

/// A picture that comes in over `[from, to]`, stays until `out_from` and
/// goes out by `out_to` (all in milliseconds from the start).
fn overlay(from: u64, to: u64, out: Option<(u64, u64)>) -> Vec<Key> {
    let mut keys = vec![
        Key::new(1).wait(at(from)).show(true).fade(0, 0),
        Key::new(span(from, to)).fade(0, 255),
    ];
    if let Some((a, b)) = out {
        keys.push(Key::new(1).wait(span(to, a)));
        keys.push(Key::new(span(a, b)).show(false).fade(255, 0));
    }
    keys
}

/// Puts the roll on the stage. The ending picture comes in over the scene
/// (0.7 to 5.0 s) and stays; two credit pages fade in and out in turn; the
/// strip of credits rises from the bottom edge (16.0 to 97.9 s); the last
/// page fades in (97.7 to 99.7 s) and out (102.7 to 105.0 s) while the
/// closing picture comes in under it. Everything is taken off at 105 s.
///
/// The executable steps the fades and the strip by the clock, in even
/// ramps (0x481190, 0x4812f0, 0x4810f0); here they are those ramps in
/// 16 ms frames, each boundary rounded on its own.
pub fn roll(st: &mut Stage, picture: &str) {
    st.clear(OWNER);
    let mut steps = Vec::new();
    let mut timeline: Vec<(u32, Vec<Key>)> = Vec::new();
    let mut add = |st: &mut Stage, name: &str, z: i32, y: i32, keys: Vec<Key>| {
        let id = st.id();
        steps.push(Step::Add(
            Sprite::new(id, OWNER, Look::picture(name), z).at(0.0, y as f32),
        ));
        timeline.push((id, keys));
    };
    add(st, picture, 20, 0, overlay(700, 5033, None));
    add(
        st,
        "staff00.akb",
        21,
        0,
        overlay(5033, 6033, Some((8033, 9000))),
    );
    add(
        st,
        "staff05.akb",
        21,
        0,
        overlay(10_366, 11_333, Some((13_366, 14_333))),
    );
    add(
        st,
        "staff.akb",
        22,
        SCREEN_HEIGHT,
        vec![
            Key::new(1).wait(at(16_033)).show(true),
            Key::new(span(16_033, 97_866)).moving((0, SCREEN_HEIGHT), (0, -STRIP_HEIGHT)),
            Key::new(1).show(false),
        ],
    );
    add(
        st,
        "staff03.akb",
        23,
        0,
        overlay(97_700, 99_666, Some((102_700, 105_000))),
    );
    add(st, "bcg99.akb", 19, 0, overlay(102_700, 105_000, None));
    let g = st.id();
    let ids: Vec<u32> = timeline.iter().map(|(id, _)| *id).collect();
    for (id, keys) in timeline {
        steps.push(Step::Keys(id, keys));
    }
    steps.push(Step::Start(g, ids.clone()));
    steps.push(Step::Wait(g));
    steps.push(Step::Remove(ids));
    st.play(steps);
}

impl Runtime {
    /// `Game.990`: the staff roll. The answer comes after the wait: 0 when
    /// the roll ran to its end (as the original does on a first
    /// playthrough, traced), 1 when Escape, Delete, keypad 0 or the secondary
    /// click cut it short (0x480ff0 sets the result to 1; that the script
    /// then redraws the roll's last screen itself is inferred). Those can
    /// end it early only while bit 11 of global 55 is set; a primary click
    /// and the skip mode do nothing.
    pub(crate) fn casablanca_staff(
        &mut self,
        vm: &mut Vm,
        c: &Call,
    ) -> Option<silky_vm::Result<Flow>> {
        if c.sub != 0 {
            return None;
        }
        self.roll_cut = false;
        self.scene.text = None;
        self.scene.speaker_name = None;
        let picture = c.str(0).unwrap_or_else(|| "ending.akb".into());
        roll(&mut self.scene.effects, &picture);
        self.roll_locked = vm.var_g(55).int() & CAN_SKIP == 0;
        self.then = Some(FollowUp::EndRoll);
        self.wait_for(Pause::Transition(Transition {
            ms: ROLL_MS as u32,
            rule: None,
            spin: None,
            live: true,
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shown(st: &Stage, name: &str) -> bool {
        st.sprites.iter().any(|s| {
            s.shown && s.alpha > 0 && matches!(&s.look, Look::Picture { name: n, .. } if n == name)
        })
    }

    #[test]
    fn the_roll_goes_through_its_stages_and_leaves_nothing() {
        let mut st = Stage::default();
        roll(&mut st, "ending.akb");
        st.advance(3_000);
        assert!(shown(&st, "ending.akb"));
        st.advance(3_000);
        assert!(shown(&st, "staff00.akb"));
        st.advance(3_000);
        assert!(!shown(&st, "staff00.akb"));
        st.advance(3_000);
        assert!(shown(&st, "staff05.akb"));
        st.advance(3_000);
        assert!(!shown(&st, "staff05.akb"));
        // The strip starts at the bottom edge at 16 s and is half way
        // through its run at about 57 s.
        st.advance(40_000 - 3_000);
        let strip = st
            .sprites
            .iter()
            .find(|s| matches!(&s.look, Look::Picture { name, .. } if name == "staff.akb"))
            .unwrap();
        assert!(strip.y < 0.0 && strip.y > -2000.0, "{}", strip.y);
        st.advance(47_000);
        assert!(shown(&st, "staff03.akb"));
        st.advance(5_000);
        assert!(shown(&st, "bcg99.akb"));
        st.settle();
        assert!(st.sprites.is_empty());
    }

    /// The executable's values at a time (0x4812f0, 0x481190, 0x4810f0).
    #[test]
    fn the_ramps_follow_the_clock() {
        let sprite = |st: &Stage, name: &str| {
            st.sprites
                .iter()
                .find(|s| matches!(&s.look, Look::Picture { name: n, .. } if n == name))
                .map(|s| (s.shown, s.alpha as i32, s.y))
        };
        let mut st = Stage::default();
        roll(&mut st, "ending.akb");
        let mut t = 0u64;
        for target in [3_000u64, 5_500, 6_020, 8_500, 9_100, 12_000, 40_000, 80_000] {
            while t + TICK_MS <= target {
                st.advance(TICK_MS);
                t += TICK_MS;
            }
            let ramp = |a: u64, b: u64| ((t - a) * 255 / (b - a)) as i32;
            if t < 5_033 {
                let (_, a, _) = sprite(&st, "ending.akb").unwrap();
                assert!((a - ramp(700, 5_033)).abs() <= 6, "ending {t} {a}");
            }
            if (5_033..6_033).contains(&t) {
                let (_, a, _) = sprite(&st, "staff00.akb").unwrap();
                assert!((a - ramp(5_033, 6_033)).abs() <= 8, "staff00 {t} {a}");
            }
            if (8_033..9_000).contains(&t) {
                let (_, a, _) = sprite(&st, "staff00.akb").unwrap();
                let want = 255 - ramp(8_033, 9_000);
                assert!((a - want).abs() <= 8, "staff00 out {t} {a}");
            }
            if (16_033..97_866).contains(&t) {
                let (_, _, y) = sprite(&st, "staff.akb").unwrap();
                let want = 720.0 - (t - 16_033) as f32 * 3_360.0 / 81_833.0;
                assert!((y - want).abs() < 3.0, "strip {t} {y} {want}");
            }
        }
    }

    #[test]
    fn the_roll_takes_the_whole_time_of_the_panel() {
        let mut st = Stage::default();
        roll(&mut st, "ending.akb");
        st.advance(ROLL_MS - 500);
        assert!(!st.sprites.is_empty());
        st.advance(1_500);
        assert!(st.sprites.is_empty());
    }
}
