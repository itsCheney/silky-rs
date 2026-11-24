//! きまぐれテンプテーション's E-mote calls: the companion model (`Game.10`),
//! the standing models (`Game.40`) and the CG models (`Game.48`).
//!
//! The models and their timelines are those of the sequel (`_h0007` with
//! faces 0 to 7, `_h0813` with 8 to 13; the same key opens them), so the
//! calls that take the same arguments go through the sequel's handlers. What
//! differs is below: the places, the calls the first game has of its own, and
//! the CG models.
//!
//! The first game hands the driver its positions the other way round to the
//! sequel (the companion is at -542, 400 where the sequel has 500, -930): a
//! positive x moves the model right and a positive y down. [`Place`] keeps
//! the sequel's signs, so the numbers are turned around when they are made.

use silky_vm::{Value, Vm};

use crate::host::Runtime;
use crate::scene::{Emote, Place};

/// The slot the CG model (`Game.48`) is kept in, behind the standing ones.
const CG_SLOT: i32 = 100;

/// Standing models' x by slot, in the first game's signs.
const SLOT_X: [i32; 4] = [0, -400, 400, 400];

/// The screen y of the origin of a model placed at y 0. The numbers the
/// game gives are relative to it; it was measured on a Wine screenshot of a
/// standing model at 50 percent (to within about 20 pixels), as nothing in
/// the executable tells where the driver puts its zero.
const ORIGIN_Y: i32 = 350;

/// The top of the screen as [`Place::origin`] counts it.
const PLACE_TOP: i32 = 258;

/// A place from the game's numbers: the model's origin lands at
/// (640 + x, [`ORIGIN_Y`] + y), `scale` in percent.
fn place(x: i32, y: i32, scale: i32) -> Place {
    Place {
        scale,
        x: -x,
        y: -PLACE_TOP - ORIGIN_Y - y,
        scene: false,
    }
}

/// The companion beside the protagonist (`Game.10`): 35 percent at
/// (-542, 400).
fn companion() -> Place {
    place(-542, 400, 35)
}

/// A standing model in `slot` at `scale` percent. Its y keeps the head at
/// about the same line whatever the size: `scale * 400 + (1 - scale) * -50`.
fn standing(slot: i32, scale: i32) -> Place {
    let x = usize::try_from(slot)
        .ok()
        .and_then(|i| SLOT_X.get(i))
        .copied()
        .unwrap_or(0);
    let s = scale as f32 / 100.0;
    let y = (s * 400.0 + (1.0 - s) * -50.0) as i32;
    place(x, y, scale)
}

/// The place of a CG model: full size, at the origin its file gives. The
/// game takes the position from the file; here it was found by laying the
/// model's first picture over the still picture of the same CG
/// (`HG010200_00.akb` and others), which agree to within 2 pixels for
/// four models.
fn cg_place() -> Place {
    Place {
        scale: 100,
        x: 0,
        y: -680,
        scene: false,
    }
}

impl Runtime {
    /// `Game.10`, the companion model beside the message window. False for
    /// the calls that are not handled here.
    ///
    /// Subs: 4 the scene skip point and 5 the reset of that setting (not
    /// offered, so nothing happens); 6 loads the two files; 7 the face and
    /// the extra timeline, 8 the extra timeline alone; 9 shows (fading in);
    /// 11 fades out and removes, 12 removes at once; 13 the main timeline,
    /// 16 the `a` timeline, 17 the main timeline to follow. 10, 14, 15 and
    /// 18 act on the message window and are not handled here.
    pub(crate) fn kimaten1_emote(&mut self, args: &[Value], sub: i32) -> bool {
        let a = &args[..args.len().saturating_sub(1)];
        let n = |i: usize| a.get(i).map(Value::int).unwrap_or(0);
        match (sub, a.len()) {
            (4 | 5, 0) => {}
            (6, 2) => {
                if !self.kimaten2_emote(args, 6) {
                    return false;
                }
                if let Some(e) = &mut self.scene.emote {
                    e.place = Some(companion());
                }
            }
            (7, 2) => return self.kimaten2_emote(args, 7),
            (8, 1) => {
                if let Some(e) = &mut self.scene.emote {
                    let face = e.face.map_or(0, |f| f.0);
                    e.face = Some((face, n(0)));
                }
            }
            (9, 0) => return self.kimaten2_emote(args, 9),
            (11 | 12, 0) => self.scene.emote = None,
            (13, 1) => {
                if let Some(e) = &mut self.scene.emote {
                    e.pose = Some(n(0));
                    // A new main timeline drops the one queued after the
                    // last.
                    e.next_pose = None;
                }
            }
            (16, 1) => return self.kimaten2_emote(args, 16),
            (17, 1) => {
                if let Some(e) = &mut self.scene.emote {
                    e.next_pose = Some(n(0));
                }
            }
            _ => return false,
        }
        true
    }

    /// `Game.40`, the standing models by slot (the slot is the last value
    /// but for sub 1 and 17, which take it first).
    ///
    /// Subs: 0 (12 with one more value) loads at a size in percent, 1 the
    /// face (slot, extra, face), 2 applies, 3 removes a slot and 4 all,
    /// 8 moves by (dy, dx, slot), 13 the main timeline to follow, 14 the
    /// `a` timeline, 15 the main timeline, 16 the mouth track of the voice,
    /// 17 the extra timeline alone (slot, extra), 18 pauses the driver, 21
    /// stops the mouth, 31 clears the mouth tracks' cache. 100 (a white
    /// flash of a slot) is not handled. False for the calls that are not.
    pub(crate) fn kimaten1_emotes(&mut self, args: &[Value], sub: i32, vm: &Vm) -> bool {
        let a = &args[..args.len().saturating_sub(1)];
        let n = |i: usize| a.get(i).map(Value::int).unwrap_or(0);
        let last = n(a.len().saturating_sub(1));
        match (sub, a.len()) {
            (0, 5) | (12, 6) => {
                if !self.kimaten2_emotes(args, sub, vm) {
                    return false;
                }
                let scale = n(a.len() - 4);
                if let Some(e) = self.scene.emotes.get_mut(&last) {
                    e.place = Some(standing(last, scale));
                    e.outfit = false;
                }
            }
            (1, 3) | (14, 2) | (3, 2) | (4, 1) | (16, 1) => {
                return self.kimaten2_emotes(args, sub, vm);
            }
            (13, 2) => {
                if let Some(e) = self.scene.emotes.get_mut(&last) {
                    e.next_pose = Some(n(0));
                }
            }
            (15, 2) => {
                if let Some(e) = self.scene.emotes.get_mut(&last) {
                    e.pose = Some(n(0));
                    // A new main timeline drops the one queued after the
                    // last.
                    e.next_pose = None;
                }
            }
            // (dy, dx, slot): the model's place moves at once.
            (8, 3) => {
                if let Some(p) = self
                    .scene
                    .emotes
                    .get_mut(&last)
                    .and_then(|e| e.place.as_mut())
                {
                    p.x -= n(1);
                    p.y -= n(0);
                }
            }
            (17, 2) => {
                if let Some(e) = self.scene.emotes.get_mut(&n(0)) {
                    let face = e.face.map_or(0, |f| f.0);
                    e.face = Some((face, n(1)));
                }
            }
            (18, 1) | (31, 0) => {}
            (21, 0) => {
                self.lip = None;
                self.lip_track = None;
            }
            _ => return false,
        }
        true
    }

    /// `Game.48`, the E-mote CG model: one file, shown over the background.
    /// 0 loads it by name (`hg010200_0` is `hg010200_0.psb`, face 0), 1 sets
    /// the face (a difference timeline), 15 the main timeline, 2 removes it
    /// and 3 fades it out over some frames and removes it. The model fades
    /// in and out in the original; here it appears and goes at once.
    pub(crate) fn kimaten1_cg(&mut self, args: &[Value], sub: i32) -> bool {
        let a = &args[..args.len().saturating_sub(1)];
        let n = |i: usize| a.get(i).map(Value::int).unwrap_or(0);
        match (sub, a.len()) {
            (0, 1) => {
                let Some(name) = a[0].as_str() else {
                    return false;
                };
                self.scene.emotes.insert(
                    CG_SLOT,
                    Emote {
                        files: [format!("{}.psb", name.to_ascii_lowercase()), String::new()],
                        face: Some((0, 0)),
                        visible: true,
                        place: Some(cg_place()),
                        cg: true,
                        ..Default::default()
                    },
                );
            }
            (1, 1) => {
                if let Some(e) = self.scene.emotes.get_mut(&CG_SLOT) {
                    e.face = Some((n(0), 0));
                }
            }
            (15, 1) => {
                if let Some(e) = self.scene.emotes.get_mut(&CG_SLOT) {
                    e.pose = Some(n(0));
                }
            }
            (2, 0) | (3, 1) => {
                self.scene.emotes.remove(&CG_SLOT);
            }
            _ => return false,
        }
        true
    }
}
