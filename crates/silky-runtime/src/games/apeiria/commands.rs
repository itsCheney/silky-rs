//! `Game` commands of 景の海のアペイリア, read from its executable.

use silky_vm::{Flow, Vm};

use super::holo::{Fade, Holo, Lights, FADE_IN_FRAMES, WIPE_RULE};
use super::mail::Mail;
use super::panels::{Battle, ItemAnim, Swarm, Tween, WindowAnim};
use crate::games::Call;
use crate::host::Runtime;

/// The αMAIL screen (`docs/game-commands.md`).
const MAIL: i32 = 1012;
/// Apeiria on the hologram panel.
const HOLO: i32 = 1013;
/// Copies of a picture at fixed spots.
const SWARM: i32 = 995;
/// The deduction board.
const BOARD: i32 = 994;
/// The notes panel and the explanation screen.
const EXPLAIN: i32 = 996;
/// The item panel.
const ITEM: i32 = 997;
/// The DNA PC's screen.
const DNA_PC: i32 = 1014;
/// The HP gauges of the battles.
const BATTLE: i32 = 1015;
/// A voice that goes on over the next messages.
const LONG_VOICE: i32 = 961;
/// A picture that changes during a voice line, from a `.vo` file.
const VOICE_PICTURE: i32 = 1007;
/// `Game.999`, a movie over the scene.
const CHANGE_MOVIE: i32 = 999;
/// `Game.1005`, the white flash of a zoomed screen.
const FLUSH: i32 = 1005;
/// `Game.993`, the screen shake.
const QUAKE: i32 = 993;
/// Offsets of the shaken screen, eight steps for each kind of shake
/// (`Game.993` sub 2), read from the program.
const QUAKE_STEPS: [[(i32, i32); 8]; 2] = [
    [
        (2, 0),
        (16, 0),
        (-2, 0),
        (-16, 0),
        (4, 0),
        (8, 0),
        (-4, 0),
        (-8, 0),
    ],
    [
        (2, 4),
        (-16, 0),
        (-4, 2),
        (0, 8),
        (8, -16),
        (0, -8),
        (-2, -4),
        (-4, -2),
    ],
];
/// Timers of the library script.
const TIMERS: i32 = 1006;
/// Saves to the auto save slot.
const AUTO_SAVE: i32 = 14;
/// Voice bus channels of `LONG_VOICE` channel 0.
const LONG_VOICE_CHANNELS: i32 = 200;
use crate::audio::Bus;
use crate::games::cmd::{CHARACTER, PICTURE, PORTRAIT};
use silky_format::game::Kind;

impl Runtime {
    /// Waits while the effects just started play out, if they make the
    /// script wait.
    fn effect_wait(&mut self) -> Option<silky_vm::Result<Flow>> {
        match self.scene.effects.blocking_ms() {
            Some(ms) if ms > 0 => {
                self.wait_for(crate::host::Pause::Transition(crate::host::Transition {
                    ms: ms as u32,
                    rule: None,
                    spin: None,
                    live: true,
                }))
            }
            _ => self.done(),
        }
    }

    /// Waits `ms` while the screen goes on changing.
    fn live_wait(&mut self, ms: u64) -> Option<silky_vm::Result<Flow>> {
        if ms == 0 {
            return self.done();
        }
        self.wait_for(crate::host::Pause::Transition(crate::host::Transition {
            ms: ms as u32,
            rule: None,
            spin: None,
            live: true,
        }))
    }

    /// `Game.993`: 0 sets the shake up, 2 (kind) picks its steps, 3
    /// starts it, 1 stops it.
    fn apeiria_quake(&mut self, c: &Call) {
        match c.sub {
            0 => self.quake_mode = 0,
            2 => {
                self.quake_mode = c.int(0).clamp(0, 1);
                if self.scene.quake_steps.is_some() {
                    self.scene.quake_steps = Some(QUAKE_STEPS[self.quake_mode as usize].to_vec());
                }
            }
            3 => self.scene.quake_steps = Some(QUAKE_STEPS[self.quake_mode as usize].to_vec()),
            1 => {
                self.scene.quake_steps = None;
                self.scene.shake = (0, 0);
            }
            _ => {}
        }
    }

    /// This game's own commands. `None` for the others.
    pub(crate) fn apeiria_game(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        match c.command {
            PORTRAIT => self.apeiria_portrait(c),
            CHARACTER => self.apeiria_character(c),
            MAIL => self.mail(vm, c),
            HOLO => self.holo(c),
            SWARM => self.swarm(c),
            BATTLE => self.battle(c),
            DNA_PC => self.dna_pc(c),
            BOARD | EXPLAIN | ITEM => self.panels(c),
            PICTURE => self.apeiria_picture(vm, c),
            LONG_VOICE => self.long_voice(c),
            // (file): a picture changes a set time into the voice line
            // (see [`super::vo`]); 1 ends the change at once.
            VOICE_PICTURE if c.sub == 0 => {
                let steps = c
                    .str(0)
                    .and_then(|f| self.assets.data(&f).ok())
                    .map(|b| super::vo::parse(&b))
                    .unwrap_or_default();
                self.vo_steps = steps;
                self.done()
            }
            VOICE_PICTURE if c.sub == 1 => {
                for (_, step) in std::mem::take(&mut self.vo_steps) {
                    step.apply(&mut self.scene);
                }
                self.done()
            }
            // (a, b, file) returns a timer, 1 stops one, 10 restarts one,
            // 11 drops them all; only the library script uses
            // them, around screens this runtime draws itself.
            TIMERS => {
                if c.sub == 0 {
                    vm.push(0);
                }
                self.done()
            }
            QUAKE => {
                self.apeiria_quake(c);
                self.done()
            }
            // (): the screen zooms to 120% and flashes white, then fades
            // back (the same steps as ふゆから、くるる。's).
            FLUSH if c.sub == 0 => {
                self.scene.zoom = Some(120);
                self.then = Some(crate::host::FollowUp::FlushWhite);
                self.handled += 1;
                Some(Ok(self.transition(1, None)))
            }
            // The screen effects of spells and attacks; the script waits
            // while one plays out.
            1001 => {
                let mut ints = vec![];
                let mut strs = vec![];
                // Sub 11: a count, then (picture, frames, rule) each.
                for v in c.args.iter().skip((c.sub == 11) as usize) {
                    match v.as_str() {
                        Some(t) => strs.push(t.to_string()),
                        None => ints.push(v.int()),
                    }
                }
                super::effects::screen_effect(&mut self.scene.effects, c.sub, &ints, &strs);
                self.effect_wait()
            }
            998 => {
                // The executable takes the arguments last pushed first.
                let ints: Vec<i32> = c.args.iter().rev().map(|v| v.int()).collect();
                super::effects::attack(&mut self.scene.effects, c.sub, &ints);
                self.effect_wait()
            }
            1000 => {
                super::effects::sprite_anime(&mut self.scene.effects, c.sub);
                self.done()
            }
            1002 => {
                let mut ints = vec![];
                let mut strs = vec![];
                for v in c.args.iter().rev() {
                    match v.as_str() {
                        Some(t) => strs.push(t.to_string()),
                        None => ints.push(v.int()),
                    }
                }
                let effects = &mut self.scene.effects;
                if let Some(cover) = super::effects::spell(effects, c.sub, &ints, &strs) {
                    self.scene.cover = cover;
                }
                self.effect_wait()
            }
            // Set up calls of the library script (991, 1003).
            991 | 1003 => self.done(),
            // (file) / (): a movie of a character changing clothes over
            // the scene (`chNN_chg.vsd`), played when movies are on; 1
            // closes it. Not played: the scene stays as with movies off,
            // and `Movie.3` answers 1 as if it had played.
            CHANGE_MOVIE if matches!(c.sub, 0 | 1) => {
                self.movie_unplayed = c.sub == 0;
                self.done()
            }
            // (): the list of conclusions drawn into the background
            // layer (see [`super::panels::CONCLUSIONS_SHEET`]).
            992 if c.sub == 0 => {
                use super::panels::{CONCLUSIONS_BYTES, CONCLUSIONS_COUNT};
                let count = (vm.var_b(CONCLUSIONS_COUNT) as usize).min(7);
                let mut cells = Vec::new();
                for i in (0..count).rev() {
                    let v = vm.var_b(CONCLUSIONS_BYTES + i);
                    if v != 0 {
                        cells.push(v);
                    }
                    if v < 100 {
                        vm.set_var_b(CONCLUSIONS_BYTES + i, v + 100);
                    }
                }
                let bg = self.scene.background.clone().unwrap_or_default();
                self.scene.panels.conclusions = Some((bg, cells));
                self.done()
            }
            // (text, page, cell): a mail drawn into the background (see
            // [`super::panels::NOTE_CELL`]).
            992 if c.sub == 1 => {
                let bg = self.scene.background.clone().unwrap_or_default();
                let text = c.str(0).unwrap_or_default();
                self.scene.panels.note = Some((bg, c.int(2), text, c.int(1)));
                self.done()
            }
            // (n) returns an entry of the save list, 1 closes it;
            // only the title script uses it.
            1016 => {
                if c.sub == 0 {
                    vm.push(0);
                }
                self.done()
            }
            // (): saves to the auto save slot.
            AUTO_SAVE if c.args.is_empty() => {
                self.auto_save = true;
                self.done()
            }
            _ => None,
        }
    }

    /// `Game.12`, event pictures, as far as they differ from the common
    /// set, and what the CG mode learns from them: the background (subs 0
    /// and 10) and layers (subs 1 and 12) count once the screen is shown
    /// (subs 2, 3, 4, 6 and 11).
    fn apeiria_picture(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let track = &mut self.cg_track;
        match (c.sub, c.args.len()) {
            (0, 1) => track.set_background(&c.str(0)?),
            // (file): the background for the CG mode only.
            (10, 1) => {
                track.set_background(&c.str(0)?);
                return self.done();
            }
            (1, 3) => track.set_layer(c.int(0), &c.str(2).unwrap_or_default()),
            // (slot, file): a layer for the CG mode only.
            (12, 2) => {
                track.set_layer(c.int(0), &c.str(1).unwrap_or_default());
                return self.done();
            }
            // (picture): marks a CG picture seen.
            (13, 1) => {
                if let Ok(i) = usize::try_from(c.int(0)) {
                    super::omake::mark(vm, i);
                }
                return self.done();
            }
            (2 | 3 | 4 | 6 | 11, _) => track.shown(vm),
            _ => {}
        }
        None
    }

    /// `Game.961`: (channel, file) plays a voice that the next message
    /// does not cut; (channel) stops it.
    fn long_voice(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        match (c.sub, c.args.len()) {
            (0, 2) => {
                let file = c.str(1)?;
                let ch = LONG_VOICE_CHANNELS + c.int(0);
                if let Err(e) = self.play(Bus::Voice, Kind::Voice, ch, &file) {
                    return Some(Err(silky_vm::Error::Host(e.to_string())));
                }
            }
            (1, 1) => self.audio.stop(Bus::Voice, LONG_VOICE_CHANNELS + c.int(0)),
            _ => return None,
        }
        self.done()
    }

    /// `Game.1014`, the DNA PC.
    fn dna_pc(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        use super::panels::{Dna, MONITOR_SCREEN};
        let scene = &mut self.scene;
        match (c.sub, c.args.len()) {
            // Its screen goes into the background layer, the monitor over
            // it.
            (0, 0) => {
                scene.background = Some(MONITOR_SCREEN.into());
                scene.panels.monitor = true;
            }
            // The system window opens (1) and closes (2) while the script
            // waits; 3 takes it and the monitor away at once.
            (1 | 2, 0) => {
                let opening = c.sub == 1;
                if opening {
                    scene.panels.system = true;
                } else if !scene.panels.system {
                    return self.done();
                }
                let a = WindowAnim::new(opening);
                scene.panels.system_anim = Some(a);
                return self.live_wait(a.length_ms());
            }
            (3, 0) => {
                scene.panels.system = false;
                scene.panels.system_anim = None;
                scene.panels.monitor = false;
                scene.panels.dna = None;
            }
            // A strand of DNA turning in the window, brought in (4) and
            // taken away (5) while the script waits.
            (4 | 5, 0) => {
                let dna = scene.panels.dna.get_or_insert_with(Dna::new);
                let ms = dna.fade(if c.sub == 4 { 255 } else { 0 });
                if c.sub == 5 && ms == 0 {
                    scene.panels.dna = None;
                }
                if ms > 0 {
                    return self.live_wait(ms);
                }
            }
            _ => return None,
        }
        self.done()
    }

    /// `Game.1015`, the HP gauges.
    fn battle(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let battle = &mut self.scene.panels.battle;
        let gauge = |i: i32| usize::try_from(i).ok().filter(|i| *i < 5);
        match (c.sub, c.args.len()) {
            // Sets the gauges up, out of view.
            (0, 0) => *battle = Some(Battle::default()),
            // (gauge): slides it in (1) or out (2) over 16 frames.
            (1 | 2, 1) => {
                if let (Some(b), Some(i)) = (battle.as_mut(), gauge(c.int(0))) {
                    b.slide(i, c.sub == 1);
                }
            }
            // (hp, gauge): moves its HP there over 128 frames (3) or sets
            // it (5).
            (3, 2) => {
                if let (Some(b), Some(i)) = (battle.as_mut(), gauge(c.int(1))) {
                    b.move_hp(i, c.int(0));
                }
            }
            (5, 2) => {
                if let (Some(b), Some(i)) = (battle.as_mut(), gauge(c.int(1))) {
                    b.hp_moves[i] = None;
                    b.hp[i] = c.int(0);
                }
            }
            // (value, gauge, kind): queues a step for sub 9.
            (4, 3) => {
                if let Some(b) = battle {
                    b.queue.push((c.int(2), c.int(1), c.int(0)));
                }
            }
            // (id, interval, count, delta, gauge): HP changes over time
            // (see [`super::panels::Drain`]).
            (6, 5) => {
                if let (Some(b), Some(i)) = (battle.as_mut(), gauge(c.int(4))) {
                    b.drains.push(super::panels::Drain {
                        id: c.int(0),
                        gauge: i,
                        delta: c.int(3),
                        count: c.int(2),
                        interval: c.int(1).max(1) as u64,
                        done: 0,
                        next: None,
                    });
                }
            }
            // (id, gauge): stops those changes.
            (7, 2) => {
                if let Some(b) = battle.as_mut() {
                    let (id, i) = (c.int(0), gauge(c.int(1)));
                    b.drains.retain(|d| !(d.id == id && Some(d.gauge) == i));
                }
            }
            // Waits for the gauges; not called by the scripts.
            (8, 0) => {}
            (9, 0) => {
                if let Some(b) = battle {
                    b.run();
                }
            }
            (10, 0) => *battle = None,
            // Queues the top gauge flickering, ending full.
            (11, 0) => {
                if let Some(b) = battle {
                    b.queue.push((2, 0, 100));
                }
            }
            _ => return None,
        }
        self.done()
    }

    /// `Game.995`, copies of one picture at fixed spots.
    fn swarm(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let swarm = &mut self.scene.panels.swarm;
        match (c.sub, c.args.len()) {
            // (alpha, size, count, file): new copies, hidden.
            (0, 4) => *swarm = Some(Swarm::new(c.str(3)?, c.int(2), c.int(1), c.int(0))),
            // (time, wave, to, from): copies fade in (1) or out (2) over
            // `time` frames while the script waits; with wave 1 the first
            // of them wavers as it goes.
            (1 | 2, 4) => {
                if let Some(s) = swarm {
                    let ms = s.change(c.int(3), c.int(2), c.sub == 1, c.int(0), c.int(1) == 1);
                    return self.live_wait(ms);
                }
            }
            (3, 0) => *swarm = None,
            // (copy): the first copy changes into `ch08_02_03.akb`.
            (4, 1) => {
                if let Some(s) = swarm {
                    s.first = Some("ch08_02_03.akb".into());
                }
            }
            // (copy): a copy jumps; 6 to 8 are other movements.
            (5, 1) | (6..=8, 0) => {}
            _ => return None,
        }
        self.done()
    }

    /// `Game.994`, `Game.996` and `Game.997`.
    fn panels(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let p = &mut self.scene.panels;
        match (c.command, c.sub, c.args.len()) {
            // The board: 0 sets it up empty, 1 drops it, 3 fades it out.
            (BOARD, 0 | 1 | 3, 0) => p.board = None,
            // (file): a picture on the board; 4 wipes to it.
            (BOARD, 2 | 4, 1) => p.board = c.str(0),
            // (x, y, file): a marker zooms onto (x, y) of the board, then
            // the board shows the picture.
            (BOARD, 5, 3) => p.board = c.str(2),
            // (kind): the notes panel; kind 0 is the only one.
            (EXPLAIN, 0, 1) => {}
            (EXPLAIN, 1 | 3, 0) => p.notes = None,
            // (page): `k01_0N.akb`.
            (EXPLAIN, 2, 1) => p.notes = Some(c.int(0)),
            // The explanation screen: 10 sets it up, 11 drops it.
            (EXPLAIN, 10, 0) => {}
            (EXPLAIN, 11, 0) => {
                p.explain = None;
                p.explain_fade = None;
                p.explain_over = None;
            }
            // (file) at once.
            (EXPLAIN, 12, 1) => p.explain = c.str(0),
            // (time, file) and (time, rule, file): the new screen fades in
            // over the old one in `time` frames while the script waits
            // (the rule is not used).
            (EXPLAIN, 13, 3) | (EXPLAIN, 14, 2) => {
                let file = c.str(c.args.len() - 1)?;
                let t = Tween::once(c.int(0), 0, 255);
                let ms = t.length_ms();
                p.explain_over = Some((file, t));
                return self.live_wait(ms);
            }
            // (file): the new screen blinks over the old one until sub 16:
            // after 42 frames it fades out over 8, and after 128 more in
            // over 8.
            (EXPLAIN, 15, 1) => {
                let keys = vec![(0, 1, 255, 255), (42, 8, 255, 0), (128, 8, 0, 255)];
                p.explain_over = Some((c.str(0)?, Tween::looping(keys)));
            }
            // (time, _, file): the new screen fades in over `time` frames
            // again and again until sub 16.
            (EXPLAIN, 19, 3) => {
                let keys = vec![(0, 1, 255, 255), (0, c.int(0), 0, 255)];
                p.explain_over = Some((c.str(2)?, Tween::looping(keys)));
            }
            // Stops the blinking where it is.
            (EXPLAIN, 16, 0) => {
                if let Some((_, t)) = &mut p.explain_over {
                    t.stop();
                }
            }
            // The screen fades out over 12 frames while the script waits.
            (EXPLAIN, 17, 0) => {
                if let Some((_, t)) = &mut p.explain_over {
                    t.stop();
                }
                if p.explain.is_some() {
                    let t = Tween::once(12, 255, 0);
                    let ms = t.length_ms();
                    p.explain_fade = Some(t);
                    return self.live_wait(ms);
                }
            }
            // (layer): the screen is drawn into the background (0) or the
            // picture layer (1) and stays there until the background
            // changes.
            (EXPLAIN, 18, 1) => {
                if let Some((_, t)) = &mut p.explain_over {
                    t.stop();
                }
                p.explain_kept = p.explain.take();
            }
            // (file): the item panel opens; 1 closes it. The script waits
            // for either.
            (ITEM, 0, 1) => {
                p.item = c.str(0);
                let a = ItemAnim::new(true);
                p.item_anim = Some(a);
                return self.live_wait(a.length_ms());
            }
            (ITEM, 1, 0) => {
                if p.item.is_some() {
                    let a = ItemAnim::new(false);
                    p.item_anim = Some(a);
                    return self.live_wait(a.length_ms());
                }
            }
            _ => return None,
        }
        self.done()
    }

    /// `Game.10`, the message window portrait, as far as it differs from
    /// the common set.
    fn apeiria_portrait(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let scene = &mut self.scene;
        match (c.sub, c.args.len()) {
            // Leave the scene skip; nothing to do with it off.
            (4 | 5, 0) => {}
            // (body): a new portrait; the name plate moves right.
            (6, 1) | (13, 1) => {
                scene.portrait = Some((c.str(0)?, None));
                scene.portrait_cheeks = None;
            }
            // (file): cheeks over the expression.
            (8, 1) => scene.portrait_cheeks = c.str(0),
            // Fade out (11) or drop (12) the portrait; 11 moves the plate
            // back.
            (11 | 12, 0) => {
                scene.portrait = None;
                scene.portrait_cheeks = None;
                scene.portrait_visible = false;
            }
            _ => return None,
        }
        self.done()
    }

    /// `Game.13`, standing characters, as far as they differ from the
    /// common set.
    fn apeiria_character(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let chars = &mut self.scene.characters;
        match (c.sub, c.args.len()) {
            // (slot, file): cheeks over the expression, until a new body.
            (6, 2) => {
                if let Some(ch) = chars.get_mut(&c.int(0)) {
                    ch.cheeks = c.str(1);
                }
            }
            // (dy, dx, slot): move at once.
            (8, 3) => {
                if let Some(ch) = chars.get_mut(&c.int(2)) {
                    ch.offset.0 += c.int(1);
                    ch.offset.1 += c.int(0);
                }
            }
            // (time, slot, body): another body in the same place, applied
            // with sub 2.
            (13, 3) => {
                self.character_time = self.character_time.max(c.int(0));
                if let Some(ch) = chars.get_mut(&c.int(1)) {
                    ch.body = c.str(2)?;
                    ch.face = None;
                    ch.cheeks = None;
                }
            }
            _ => return None,
        }
        self.done()
    }

    /// `Game.1013`. The original skips the pictures while the scene skip
    /// setting is on; this runtime has it off.
    fn holo(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let holo = self.scene.holo.get_or_insert_with(Holo::default);
        match (c.sub, c.args.len()) {
            // (time, size, place, body): a new picture; faces start again.
            (0, 4) => {
                holo.body = c.str(3)?;
                holo.scale = c.int(1);
                holo.place = c.int(2);
                holo.faces.clear();
                holo.offset = (0, 0);
            }
            // (file): an expression or cheeks drawn over the picture.
            (1, 1) => {
                let f = c.str(0)?;
                holo.faces.retain(|x| !x.eq_ignore_ascii_case(&f));
                holo.faces.push(f);
            }
            // (way): 0 wiping in among specks of light (fading in on the
            // bottom half), 1 at once, 2 fading in, over 12 frames while
            // the script waits. Up already, she comes again the same way;
            // while she is still coming, the script waits for that.
            (2, 1) => {
                holo.shown = !holo.body.is_empty();
                if !holo.shown {
                    return self.done();
                }
                if let Some(f) = holo.fade.filter(|f| !f.done()) {
                    let left = f.length_ms().saturating_sub(f.ms);
                    return self.live_wait(left);
                }
                holo.fade = None;
                holo.lights = None;
                match c.int(0) {
                    0 if holo.panel == 0 => {
                        holo.fade = Some(Fade::wipe(WIPE_RULE, FADE_IN_FRAMES));
                        let seed = self.scene.effects.rng ^ self.handled as u32;
                        holo.lights = Some(Lights::new(holo.middle(), seed));
                    }
                    0 | 2 => holo.fade = Some(Fade::new(0, 255, FADE_IN_FRAMES, false)),
                    _ => {}
                }
                if let Some(f) = holo.fade {
                    return self.live_wait(f.length_ms());
                }
            }
            // (time): she and the panel fade out over `time` frames while
            // the script waits, then she is gone.
            (3, 1) => {
                if !holo.shown {
                    holo.remove();
                    return self.done();
                }
                let fade = Fade::new(255, 0, c.int(0), true);
                holo.fade = Some(fade);
                return self.wait_for(crate::host::Pause::Transition(crate::host::Transition {
                    ms: fade.length_ms() as u32,
                    rule: None,
                    spin: None,
                    live: true,
                }));
            }
            (5, 0) => holo.remove(),
            // (dx, dy, time): move her picture.
            (4, 3) => holo.offset = (holo.offset.0 + c.int(0), holo.offset.1 + c.int(1)),
            // (half): panel picture.
            (6, 1) => holo.panel = c.int(0),
            _ => return None,
        }
        self.done()
    }

    fn mail(&mut self, vm: &mut Vm, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let mail = &mut self.scene.mail;
        match (c.sub, c.args.len()) {
            // (tray): set up the list of the inbox (0) or the sent
            // tray (1); its newest mail is byte 1200 or 1400.
            (0, 1) => {
                let tray = c.int(0);
                let newest = vm.var_b(if tray == 0 { 1200 } else { 1400 });
                *mail = Some(Mail {
                    tray,
                    newest,
                    ..Mail::default()
                });
            }
            // The panel opens, or closes and goes, while the script
            // waits.
            (1, 0) => {
                let seed = self.scene.effects.rng ^ self.handled as u32;
                let m = mail.get_or_insert_with(Mail::default);
                m.shown = true;
                let ms = m.animate(true, seed);
                return self.live_wait(ms);
            }
            (2, 0) => match mail {
                Some(m) if m.shown => {
                    m.notice = None;
                    m.body = None;
                    let ms = m.animate(false, 0);
                    return self.live_wait(ms);
                }
                _ => *mail = None,
            },
            (3, 1) => {
                let text = c.str(0)?.replace("\\n", "\n");
                mail.get_or_insert_with(Mail::default).body = Some(text);
            }
            (4, 0) => {
                if let Some(m) = mail {
                    m.body = None;
                }
            }
            // (count): "n new mails received".
            (5, 1) => mail.get_or_insert_with(Mail::default).notice = Some(c.int(0)),
            (6, 0) => {
                if let Some(m) = mail {
                    m.notice = None;
                }
            }
            // (row) picks a row of the list, 9 lets it go.
            (8, 1) => mail.get_or_insert_with(Mail::default).selected = Some(c.int(0)),
            (9, 0) => {
                if let Some(m) = mail {
                    m.selected = None;
                }
            }
            // (mode): the panel holds the Apeiria system window.
            (7, 1) => {
                mail.get_or_insert_with(Mail::default).system = Some(c.int(0));
            }
            // (page): scrolls the open mail's text.
            (10, 1) => mail.get_or_insert_with(Mail::default).body_page = c.int(0),
            // (mail): byte 1200, the newest mail of the inbox.
            (11, 1) => vm.set_var_b(1200, c.int(0) as u8),
            // (mail): the first free byte of 1400 to 1499 takes it.
            (14, 1) => {
                if let Some(i) = (1400..1500).find(|&i| vm.var_b(i) == 0) {
                    vm.set_var_b(i, c.int(0) as u8);
                }
            }
            _ => return None,
        }
        self.done()
    }
}

/// Engine calls this runtime has nothing to do for, checked on the
/// executable: the same set as ふゆから、くるる。 has, with this game's
/// numbers.
pub(crate) fn nothing_to_do(class: i32, sub: i32) -> bool {
    matches!(
        (class, sub),
        // Set up at the start: the message window page (MesWnd.1), font
        // sizes, the one-line message, menus, selection settings.
        (2, 1) | (10, 0 | 1) | (24, 0 | 2) | (26, 2..=4) | (1, 0)
            // The store of variables the original rebuilds its scene from
            // after a load (Save.2 reads, 4 writes, 5 and 6 delete); this
            // runtime saves the scene itself.
            | (27, 2 | 4 | 5 | 6)
            // Its own backlog: size, clear, voice marks of the entry.
            | (22, 0..=2)
    )
}
