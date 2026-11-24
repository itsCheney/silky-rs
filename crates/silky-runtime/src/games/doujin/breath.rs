//! The puff of breath of the event pictures (`Game.1006`): a soft white
//! glow that grows and fades while it drifts, again and again, until
//! told to stop.
//!
//! A `.brt` file is 276 bytes: where the glow starts (x, y), the point it
//! grows about (x, y) and a 260-byte picture name (always the glow).

use crate::games::apeiria::anim::{Key, Look, Sprite, Step};
use crate::games::apeiria::effects::User;
use crate::scene::Scene;

/// Owner number of the sprites on the effect stage.
const OWNER: u32 = 1006;

fn int(bytes: &[u8], at: usize) -> i32 {
    bytes
        .get(at..at + 4)
        .and_then(|b| b.try_into().ok())
        .map(i32::from_le_bytes)
        .unwrap_or(0)
}

/// The settings of a `.brt` file.
pub struct Brt {
    pub at: (i32, i32),
    pub pivot: (i32, i32),
    pub picture: String,
}

impl Brt {
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let raw = bytes.get(16..16 + 260)?;
        let end = raw.iter().position(|&b| b == 0).unwrap_or(raw.len());
        let picture = String::from_utf8_lossy(&raw[..end]).into_owned();
        (!picture.is_empty()).then(|| Self {
            at: (int(bytes, 0), int(bytes, 4)),
            pivot: (int(bytes, 8), int(bytes, 12)),
            picture,
        })
    }
}

/// Starts a puff: it takes `frames` frames to drift by `d`, grow and
/// fade, then waits `rest` frames and begins again. A puff already there
/// goes.
pub fn start(scene: &mut Scene, brt: &Brt, d: (i32, i32), frames: i32, rest: i32) {
    let st = &mut scene.effects;
    st.clear(OWNER);
    let (id, group, user) = (st.id(), st.id(), st.id());
    st.users.insert(
        user,
        User::Breath {
            owner: OWNER,
            sprite: id,
        },
    );
    let (x, y) = brt.at;
    let keys = vec![
        Key::new(frames.max(1))
            .show(true)
            .moving((x, y), (x + d.0, y + d.1))
            .scaling(
                (0.2, 0.2),
                (1.0, 1.0),
                (brt.pivot.0 as f32, brt.pivot.1 as f32),
            )
            .user(user),
        Key::new(1).wait(rest.max(0)),
        Key::jump(0),
    ];
    st.play(vec![
        Step::Add(Sprite::new(id, OWNER, Look::picture(&brt.picture), 0).at(x as f32, y as f32)),
        Step::Keys(id, keys),
        Step::Start(group, vec![id]),
    ]);
}

/// Lets the puff finish the round it is in and stop.
pub fn finish(scene: &mut Scene) {
    for s in scene
        .effects
        .sprites
        .iter_mut()
        .filter(|s| s.owner == OWNER)
    {
        s.keys.truncate(1);
    }
}

/// Takes the puff away.
pub fn clear(scene: &mut Scene) {
    scene.effects.clear(OWNER);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn puff_fades_in_and_out() {
        let mut bytes = Vec::new();
        for v in [10i32, 20, 95, 106] {
            bytes.extend(v.to_le_bytes());
        }
        let mut name = b"breath.akb".to_vec();
        name.resize(260, 0);
        bytes.extend(name);
        let brt = Brt::parse(&bytes).unwrap();
        assert_eq!((brt.at, brt.pivot), ((10, 20), (95, 106)));
        let mut scene = Scene::default();
        start(&mut scene, &brt, (40, 10), 120, 40);
        let mut top = 0;
        for _ in 0..80 {
            scene.effects.advance(16);
            top = top.max(scene.effects.sprites[0].alpha);
        }
        assert!(top > 150);
        finish(&mut scene);
        for _ in 0..200 {
            scene.effects.advance(16);
        }
        assert_eq!(scene.effects.sprites[0].alpha, 0);
        clear(&mut scene);
        assert!(scene.effects.sprites.is_empty());
    }
}
