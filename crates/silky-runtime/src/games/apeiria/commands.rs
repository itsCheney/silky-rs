//! `Game` commands of 景の海のアペイリア, read from its executable.

use silky_vm::{Flow, Vm};

use super::holo::Holo;
use super::mail::Mail;
use super::panels::{Battle, Swarm};
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
/// Lip sync of an event picture, from a `.vo` file.
const EVENT_LIPS: i32 = 1007;
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
            // (file) / (): the event picture's mouth moves with the voice;
            // not done, the picture stays as it is.
            EVENT_LIPS if matches!(c.sub, 0 | 1) => self.done(),
            // (a, b, file) returns a timer, 1 stops one, 10 restarts one,
            // 11 drops them all; only the library script uses
            // them, around screens this runtime draws itself.
            TIMERS => {
                if c.sub == 0 {
                    vm.push(0);
                }
                self.done()
            }
            // Effects that leave nothing behind once over, not done: the
            // screen shake (993), the attack and spell effects (998, 1001,
            // 1002), the white flash of a zoomed screen (1005), a sprite
            // animation (1000), and set up calls of the library
            // script (991, 1003).
            991 | 993 | 998 | 1000 | 1001 | 1002 | 1003 | 1005 => self.done(),
            // (): the list of conclusions drawn into the background layer
            // from bytes 0x514 to 0x51a of the save data (once, at the end
            // of s0110); not done.
            992 => self.done(),
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
        use super::panels::MONITOR_SCREEN;
        let scene = &mut self.scene;
        match (c.sub, c.args.len()) {
            // Its screen goes into the background layer, the monitor over
            // it.
            (0, 0) => {
                scene.background = Some(MONITOR_SCREEN.into());
                scene.panels.monitor = true;
            }
            // The system window opens (1) and closes (2); 3 takes the
            // monitor away too.
            (1, 0) => scene.panels.system = true,
            (2, 0) => scene.panels.system = false,
            (3, 0) => {
                scene.panels.system = false;
                scene.panels.monitor = false;
            }
            // A strand of DNA scrolling in the window (4) and its progress
            // bar (5).
            (4 | 5, 0) => {}
            _ => return None,
        }
        self.done()
    }

    /// `Game.1015`, the HP gauges. Their slides and the
    /// draining over time (subs 6 and 7) are not done.
    fn battle(&mut self, c: &Call) -> Option<silky_vm::Result<Flow>> {
        let battle = &mut self.scene.panels.battle;
        let gauge = |i: i32| usize::try_from(i).ok().filter(|i| *i < 5);
        match (c.sub, c.args.len()) {
            // Sets the gauges up, out of view.
            (0, 0) => *battle = Some(Battle::default()),
            // (gauge): slides it in (1) or out (2).
            (1 | 2, 1) => {
                if let (Some(b), Some(i)) = (battle.as_mut(), gauge(c.int(0))) {
                    b.shown[i] = c.sub == 1;
                }
            }
            // (hp, gauge): moves (3) or sets (5) its HP.
            (3 | 5, 2) => {
                if let (Some(b), Some(i)) = (battle.as_mut(), gauge(c.int(1))) {
                    b.hp[i] = c.int(0);
                }
            }
            // (value, gauge, kind): queues a step for sub 9.
            (4, 3) => {
                if let Some(b) = battle {
                    b.queue.push((c.int(2), c.int(1), c.int(0)));
                }
            }
            // (id, start, step, time, gauge): drains HP over time; 7
            // (id, gauge) stops it; 8 waits for the gauges.
            (6, 5) | (7, 2) | (8, 0) => {}
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
            // (time, wave, to, from): fade copies in; with wave 1 the first
            // of them comes in rippling.
            (1, 4) | (2, 4) => {
                if let Some(s) = swarm {
                    s.set(c.int(3), c.int(2), c.sub == 1);
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
            (EXPLAIN, 11, 0) => p.explain = None,
            // (file) at once (12) or with a wipe (15); (time, file) fading
            // (14); (time, rule, file) through a rule (13, 19).
            (EXPLAIN, 12 | 15, 1) => p.explain = c.str(0),
            (EXPLAIN, 14, 2) => p.explain = c.str(1),
            (EXPLAIN, 13 | 19, 3) => p.explain = c.str(2),
            // 16 stops the transition, 17 fades the screen in, 18 keeps it
            // in the picture layer.
            (EXPLAIN, 16 | 17, 0) | (EXPLAIN, 18, 1) => {}
            // (file): the item panel slides in; 1 takes it away.
            (ITEM, 0, 1) => p.item = c.str(0),
            (ITEM, 1, 0) => p.item = None,
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
            // (how): 0 with light particles, 1 at once, 2 fading in.
            (2, 1) => holo.shown = !holo.body.is_empty(),
            // (time): fade out, then 5: gone.
            (3, 1) | (5, 0) => {
                holo.shown = false;
                holo.body.clear();
                holo.faces.clear();
            }
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
            (1, 0) => mail.get_or_insert_with(Mail::default).shown = true,
            (2, 0) => *mail = None,
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
            // (mode) before the screen opens, (mail) opens a mail at once
            // (10), (flag) (14): their screens
            // are not done.
            (7 | 10 | 14, 1) => {}
            // (mail): byte 1200, the newest mail of the inbox.
            (11, 1) => vm.set_var_b(1200, c.int(0) as u8),
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
            // Settings read from the XML configuration (scene skip, auto
            // read, ...), read as 0 (off).
            | (25, 0 | 1)
    )
}
