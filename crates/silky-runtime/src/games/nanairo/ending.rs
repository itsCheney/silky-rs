//! The ending of なないろリンカネーション: the message overlay of the last
//! card (`Game.50`) and the staff roll (`Game.34`).

use silky_vm::{Flow, Vm};

use super::effects::{clear, scaled, Fx, ENDING, STAFF, Z_EFFECT, Z_FLASH};
use crate::games::apeiria::anim::{Key, Look, Stage, Step, TICK_MS};
use crate::games::apeiria::effects::User;
use crate::games::Call;
use crate::host::{FollowUp, Pause, Runtime, Transition};

/// The picture of handwriting the overlay shows pieces of.
const MOJI: &str = "moji.akb";
/// Width of the rule pictures (they are as big as the screen).
const RULE_WIDTH: u32 = 1280;

/// `Game.50.1(rule, frames, height, width, y, x)`: the piece of the
/// handwriting picture in that rectangle is wiped in, in its place,
/// through the rule picture, over `frames` frames. The rule picture is
/// laid on the piece's top left corner. The piece stays until `50.2`. The
/// script waits for the wipe.
pub fn write_piece(st: &mut Stage, a: [i32; 6], factor: f32) {
    let [rule, frames, h, w, y, x] = a;
    let slot = st.sprites.iter().filter(|s| s.owner == ENDING).count() as u32;
    let mut fx = Fx::new(st, ENDING);
    let part = (
        x.max(0) as u32,
        y.max(0) as u32,
        w.max(1) as u32,
        h.max(1) as u32,
    );
    let id = fx.sprite(
        slot,
        Look::Reveal {
            source: Box::new(Look::part(MOJI, part)),
            rule: rule.max(0) as u32,
            threshold: 0.0,
            erasing: false,
            stride: RULE_WIDTH,
        },
        Z_EFFECT,
    );
    if let Some(Step::Add(s)) = fx.steps.last_mut() {
        s.x = x as f32;
        s.y = y as f32;
    }
    let n = scaled(frames, factor);
    // The rule's values go up to 255 and the edge is as wide again.
    let step = if n > 1 { 512.0 / (n - 1) as f32 } else { 512.0 };
    let u = fx.user(User::Ramp {
        owner: ENDING,
        sprite: id,
        step,
    });
    fx.play(vec![(id, vec![Key::new(n).show(true).user(u)])], true);
    fx.run();
}

/// Total time of the roll, which is the length of its music.
pub const ROLL_MS: u64 = 130_330;
/// The music of the roll.
const ROLL_MUSIC: &str = "31.WAV";
const FRAME: u64 = TICK_MS;

fn frames(ms: u64) -> i32 {
    (ms / FRAME) as i32
}

/// The keys of a full screen picture that comes in at `from` (ms), fades
/// in over `fade_in`, stays for `hold` and fades out over `fade_out`
/// (nothing: it stays until the end of the roll).
fn page(from: u64, fade_in: u64, hold: u64, fade_out: u64) -> Vec<Key> {
    let mut keys = vec![
        Key::new(1).wait(frames(from)).show(true).fade(0, 0),
        Key::new(frames(fade_in).max(1)).fade(0, 255),
        Key::new(1).wait(frames(hold)),
    ];
    if fade_out > 0 {
        keys.push(Key::new(frames(fade_out).max(1)).show(false).fade(255, 0));
    }
    keys
}

/// The staff roll on the stage: `picture` (the ending's picture) is shown
/// for 4 seconds after a fade in, two credit pages follow, the strip of
/// credits scrolls up for 106.7 seconds, and at the end the company's name
/// comes up on black and is turned into the closing page. The script
/// waits [`ROLL_MS`] (it is the music's length).
///
/// Approximate: the times of the stages are the original's; the curves
/// inside them are not known. The pages fade in and out, the strip moves
/// at an even pace from below the screen to above it (its pace is not
/// known; this one makes it end where the original's does), and the end
/// is a fade in of the logo and a fade over to the closing page.
pub fn roll(st: &mut Stage, picture: &str, strip: &str) {
    clear(st, STAFF);
    let mut fx = Fx::new(st, STAFF);
    let z = Z_FLASH + 1;
    let mut timeline: Vec<(u32, Vec<Key>)> = Vec::new();
    // Black under everything, so that the scene is not seen through.
    let black = fx.sprite(
        0,
        Look::Fill {
            color: [0, 0, 0, 255],
            w: 1280,
            h: 720,
        },
        z,
    );
    timeline.push((black, vec![Key::new(1).show(true)]));
    let pages = [
        (1, picture, page(0, 1000, 3000, 1000)),
        (2, "staff_p2.akb", page(5000, 1000, 3000, 1000)),
        (3, "staff_p3.akb", page(10_330, 500, 2000, 500)),
        // The logo on black at 120 seconds, the closing page over it at
        // 128; the logo fades out as the page comes in, which stays to the end.
        (5, "staff_p8.akb", page(120_000, 1000, 7000, 2000)),
        (6, "staff_p1.akb", page(128_000, 2000, ROLL_MS - 130_000, 0)),
    ];
    for (slot, name, keys) in pages {
        let z = if slot >= 5 {
            z + 3 + slot as i32
        } else {
            z + 1
        };
        let id = fx.sprite(slot, Look::picture(name), z);
        timeline.push((id, keys));
    }
    // The strip, centred, from just below the screen to just above it.
    let strip_id = fx.sprite(4, Look::picture(strip), z + 2);
    let height = 3214;
    let x = (1280 - 575) / 2;
    for step in fx.steps.iter_mut() {
        if let Step::Add(s) = step {
            if s.id == strip_id {
                s.x = x as f32;
                s.y = 720.0;
            }
        }
    }
    timeline.push((
        strip_id,
        vec![
            Key::new(1).wait(frames(13_330)).show(true),
            Key::new(frames(120_000 - 13_330)).moving((x, 720), (x, -height)),
            Key::new(1).show(false),
        ],
    ));
    // One timeline for all of it; the roll is taken away when it ends.
    let g = fx.st.id();
    let ids: Vec<u32> = timeline.iter().map(|(id, _)| *id).collect();
    for (id, keys) in timeline {
        fx.steps.push(Step::Keys(id, keys));
    }
    fx.steps.push(Step::Start(g, ids.clone()));
    fx.steps.push(Step::Wait(g));
    fx.steps.push(Step::Remove(ids));
    fx.run();
}

impl Runtime {
    /// `Game.50`: the message overlay of the ending's last card.
    /// 0 prepares it, 1 wipes in a piece of the handwriting (the script
    /// waits), 2 takes it away. With the scenes skipped in the
    /// configuration nothing is done.
    pub(crate) fn nanairo_ending(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        if self.scene_skip() {
            return self.done();
        }
        let f = self.system.settings.effect_scale();
        match c.sub {
            0 | 2 => clear(&mut self.scene.effects, ENDING),
            1 => {
                let a = [c.int(0), c.int(1), c.int(2), c.int(3), c.int(4), c.int(5)];
                write_piece(&mut self.scene.effects, a, f);
                return self.nanairo_effect_wait();
            }
            _ => return None,
        }
        self.done()
    }

    /// `Game.34(picture, strip)`: the staff roll. The music starts, the
    /// pictures play for [`ROLL_MS`] while the script waits, and the
    /// music is stopped after it. A click (or the skip mode) ends it
    /// early. It leaves 0 on the script's stack (the original's value is
    /// not known; the scripts do not use it).
    pub(crate) fn nanairo_staff(
        &mut self,
        vm: &mut Vm,
        c: &Call,
    ) -> Option<silky_vm::Result<Flow>> {
        vm.push(0);
        let (Some(picture), Some(strip)) = (c.str(0), c.str(1)) else {
            return self.done();
        };
        // The roll covers the whole screen, the message window too.
        self.scene.text = None;
        self.scene.speaker_name = None;
        roll(&mut self.scene.effects, &picture, &strip);
        let _ = self.play_music_once(ROLL_MUSIC);
        self.then = Some(FollowUp::EndStaff);
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

    #[test]
    fn a_piece_is_written_in_its_frames_and_stays() {
        let mut st = Stage::default();
        write_piece(&mut st, [150, 72, 70, 274, 290, 100], 1.0);
        let wait = st.blocking_ms().unwrap() / TICK_MS;
        assert!((72..=74).contains(&wait), "{wait}");
        st.advance(36 * TICK_MS);
        let s = &st.sprites[0];
        assert!(
            matches!(&s.look, Look::Reveal { threshold, .. } if *threshold > 200.0 && *threshold < 300.0)
        );
        assert_eq!((s.x, s.y), (100.0, 290.0));
        st.settle();
        assert!(
            matches!(&st.sprites[0].look, Look::Reveal { threshold, .. } if *threshold >= 510.0)
        );
        // The next piece is another sprite; the first stays.
        write_piece(&mut st, [151, 90, 70, 346, 290, 377], 1.0);
        assert_eq!(st.sprites.len(), 2);
        // At the second effect speed the wipe takes half the frames.
        let mut st = Stage::default();
        write_piece(&mut st, [150, 72, 70, 274, 290, 100], 0.5);
        let wait = st.blocking_ms().unwrap() / TICK_MS;
        assert!((36..=38).contains(&wait), "{wait}");
    }

    #[test]
    fn the_roll_goes_through_its_stages_and_leaves_nothing() {
        let mut st = Stage::default();
        roll(&mut st, "staff01.akb", "staff_p4.akb");
        let shown = |st: &Stage, name: &str| {
            st.sprites.iter().any(|s| {
                s.shown
                    && s.alpha > 0
                    && matches!(&s.look, Look::Picture { name: n, .. } if n == name)
            })
        };
        st.advance(2_000);
        assert!(shown(&st, "staff01.akb"));
        st.advance(4_000);
        assert!(shown(&st, "staff_p2.akb") && !shown(&st, "staff01.akb"));
        st.advance(5_500);
        assert!(shown(&st, "staff_p3.akb"));
        // 60 seconds in, the strip is about halfway up.
        st.advance(46_000);
        let strip = st
            .sprites
            .iter()
            .find(|s| matches!(&s.look, Look::Picture { name, .. } if name == "staff_p4.akb"))
            .unwrap();
        assert!(strip.y < 0.0 && strip.y > -2500.0, "{}", strip.y);
        assert_eq!(strip.x, 352.0);
        st.advance(66_000);
        assert!(shown(&st, "staff_p8.akb"));
        // Settling (a click) takes it all away.
        st.settle();
        assert!(st.sprites.is_empty());
    }
}
