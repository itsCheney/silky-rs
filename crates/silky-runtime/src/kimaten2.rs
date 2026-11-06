//! きまぐれテンプテーション2: what its interface needs beyond the game data.
//!
//! The speaker's name comes as text (`Game.11("悠久")`); the engine looks it
//! up in a list in its executable and draws that row of `name_p.akb`. The
//! list was read from the executable once and is kept here. A fan
//! translation renames the speakers in its scripts and redraws
//! `name_p.akb` in the same row order, so translated names are matched to
//! rows by comparing its scripts with the original ones.

use std::collections::HashMap;

use silky_format::game::{Game, Kind};
use silky_format::mes::{Instr, Operand, Profile, Script};

/// Speakers in the row order of `name_p.akb` (rows 42 pixels high).
pub const NAMES: [&str; 20] = [
    "美代",
    "儚げな女性",
    "悠久",
    "？？？",
    "安藤",
    "コハル",
    "猫",
    "晶",
    "アンネリーゼ",
    "千代",
    "千代と思われる女性",
    "キルト",
    "みりあ",
    "花音",
    "すすり泣く女の霊",
    "血濡れの花嫁",
    "ベルベット",
    "メリーさん",
    "悠久・アンネリーゼ",
    "千代・美代",
];

/// Row of `name_p.akb` for each speaker name, including the names of the
/// translation in use (see [`translated_names`]).
pub fn speaker_rows(game: &Game) -> HashMap<String, usize> {
    let mut rows: HashMap<String, usize> = NAMES
        .iter()
        .enumerate()
        .map(|(i, n)| (n.to_string(), i))
        .collect();
    for (translated, original) in translated_names(game) {
        if let Some(&row) = rows.get(&original) {
            rows.entry(translated).or_insert(row);
        }
    }
    rows
}

/// Pairs (translated name, original name) of speakers, from the scripts the
/// translation in use replaces: the n-th speaker call of a translated
/// script names the same speaker as the n-th of the original.
pub fn translated_names(game: &Game) -> Vec<(String, String)> {
    let Some(t) = game.translation() else {
        return Vec::new();
    };
    let mut pairs = Vec::new();
    for name in game.set(Kind::Script).names() {
        let Some(path) = t.find(&name) else {
            continue;
        };
        let (Ok(original), Ok(data)) = (game.set(Kind::Script).read(&name), std::fs::read(path))
        else {
            continue;
        };
        let (Ok(a), Ok(b)) = (
            Script::parse(&original),
            Script::parse_with(&data, t.encoding),
        ) else {
            continue;
        };
        let (sa, sb) = (speakers(&a), speakers(&b));
        if sa.len() != sb.len() {
            continue;
        }
        for (o, n) in sa.into_iter().zip(sb) {
            if o != n && !pairs.iter().any(|(x, _)| *x == n) {
                pairs.push((n, o));
            }
        }
    }
    pairs
}

/// Names passed to `Game.11` in a script, in order: the byte code
/// `push.s name; push 10; push 1; add; push 29; native`.
fn speakers(script: &Script) -> Vec<String> {
    let Ok(code) = script.disassemble(Profile::default()) else {
        return Vec::new();
    };
    let int = |i: &Instr, v: i32| i.opcode == 0x32 && i.operand == Operand::Int(v);
    code.windows(6)
        .filter(|w| {
            matches!(w[0].opcode, 0x0a | 0x0b | 0x33)
                && int(&w[1], 10)
                && int(&w[2], 1)
                && w[3].opcode == 0x34
                && int(&w[4], 29)
                && w[5].opcode == 0x18
        })
        .filter_map(|w| match &w[0].operand {
            Operand::Str(b) => Some(script.decode(b)),
            _ => None,
        })
        .collect()
}

/// Message window, read from the engine's window code. The window can be
/// dragged up; this is its default (lowest) position.
pub mod window {
    /// Top of the 1280 x 300 window sprite.
    pub const TOP: i32 = 468;
    /// Name plate: 288 x 42 cells of `name_p.akb`, at (178, TOP + 47).
    pub const NAME: (i32, i32) = (178, TOP + 47);
    pub const NAME_SIZE: (u32, u32) = (288, 42);
    /// First text line, from screenshots of the game.
    pub const TEXT_LEFT: f32 = 180.0;
    pub const TEXT_BASELINE: f32 = (TOP + 115) as f32;
}

/// The map (`Game.43(voice, back, map)`), read from the engine's map
/// screen.
///
/// Byte variables describe the places: for place `i`, `b[i]` is 0 when it
/// cannot be chosen, else nonzero, plus 100 once it is done (a 探索終了
/// stamp); `b[20 + i] == 1` hides it. The first map has four places; the
/// second has seven, of which places 4 and 5 open a panel of three more
/// destinations each, described by `b[10 + n]`.
pub mod map {
    /// Places per map.
    pub const PLACES: [usize; 2] = [4, 7];
    /// Result of each place (`None` opens a panel).
    const RESULTS: [[Option<i32>; 7]; 2] = [
        [Some(1), Some(2), Some(3), Some(4), None, None, None],
        [Some(1), Some(2), Some(3), Some(4), None, None, Some(5)],
    ];
    /// Panels: for each of their three destinations, the byte variable
    /// (after 10) and the result; then the variable that shows the
    /// panel's back button.
    const PANELS: [([(usize, i32); 3], usize); 2] = [
        ([(1, 11), (2, 12), (0, 10)], 3),
        ([(4, 13), (5, 14), (6, 15)], 7),
    ];
    /// Voices played when the map opens: three per set, one at random.
    pub const VOICES: [[&str; 3]; 2] = [
        ["k2a0344.ogg", "k2a0345.ogg", "k2a0346.ogg"],
        ["k2a1079.ogg", "k2a1080.ogg", "k2a1081.ogg"],
    ];
    /// Result of the back button.
    pub const BACK: i32 = 255;

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct Spot {
        pub shown: bool,
        pub enabled: bool,
        pub done: bool,
    }

    fn spot(v: u8) -> Spot {
        Spot {
            shown: true,
            enabled: v % 100 != 0,
            done: v >= 100,
        }
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct Map {
        /// The second map (`map2.akb`).
        pub second: bool,
        /// Whether there is a back button.
        pub back: bool,
        /// Byte variables 0..30 at the call.
        pub bytes: Vec<u8>,
        /// Panel open on the second map.
        pub panel: Option<usize>,
    }

    /// What a click does.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Click {
        Result(i32),
        OpenPanel(usize),
        ClosePanel,
    }

    impl Map {
        fn b(&self, i: usize) -> u8 {
            self.bytes.get(i).copied().unwrap_or(0)
        }

        pub fn places(&self) -> usize {
            PLACES[self.second as usize]
        }

        pub fn place(&self, i: usize) -> Spot {
            Spot {
                shown: self.b(20 + i) != 1,
                ..spot(self.b(i))
            }
        }

        /// Destinations of a panel, by their `map03.area` index (0..3 or
        /// 4..7), and whether it has a back button (index 3 or 7).
        pub fn panel_spots(&self, panel: usize) -> ([(usize, Spot); 3], bool) {
            let (dest, back) = PANELS[panel];
            let k0 = panel * 4;
            (
                [0, 1, 2].map(|j| (k0 + j, spot(self.b(10 + dest[j].0)))),
                self.b(10 + back) != 0,
            )
        }

        pub fn click_place(&self, i: usize) -> Option<Click> {
            if !self.place(i).enabled || !self.place(i).shown {
                return None;
            }
            match RESULTS[self.second as usize].get(i).copied().flatten() {
                Some(r) => Some(Click::Result(r)),
                None if self.second && i == 4 => Some(Click::OpenPanel(0)),
                None if self.second && i == 5 => Some(Click::OpenPanel(1)),
                None => None,
            }
        }

        /// A panel button by its `map03.area` index.
        pub fn click_panel(&self, k: usize) -> Option<Click> {
            let panel = k / 4;
            if self.panel != Some(panel) {
                return None;
            }
            if k % 4 == 3 {
                return Some(Click::ClosePanel);
            }
            let (dest, _) = PANELS[panel];
            let (n, result) = dest[k % 4];
            spot(self.b(10 + n))
                .enabled
                .then_some(Click::Result(result))
        }
    }
}
