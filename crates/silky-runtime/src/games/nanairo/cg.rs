//! The CG registry of なないろリンカネーション (`Game.14`, `Game.22`).
//!
//! A script names the picture (`Game.14.10`) and its layers in slot order
//! (`Game.14.1`), and the engine registers the combination when the scene
//! is committed: if it is a row of [`ROWS`], the row's byte flag is set, which
//! the CG mode reads.

use silky_vm::Vm;

/// The script variable (H bank) that is set while the extras replay a scene.
const REPLAY_VAR: usize = 69;

pub use super::cg_table::ROWS;

/// Byte flag that opens the CG tab of the extras screens; any registered
/// picture sets it.
const CG_TAB_FLAG: usize = 3002;
/// Byte flag the engine also sets when a table picture is registered.
const SEEN_FLAG: usize = 3001;
/// Layers the engine compares with a table row.
const LAYERS: usize = 6;

/// The picture and layers named so far.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Combo {
    base: String,
    parts: Vec<String>,
    /// The two tables of pictures the scripts lay over the background:
    /// five places, then three. An empty name is a free place.
    #[serde(default)]
    tables: [Vec<String>; 2],
    /// What the last composition put on the screen, in drawing order.
    #[serde(default)]
    on_screen: Vec<String>,
}

/// Places of the two picture tables.
const TABLE_SIZES: [usize; 2] = [5, 3];

fn norm(name: &str) -> String {
    let s = name.to_ascii_lowercase();
    s.strip_suffix(".akb").map(str::to_string).unwrap_or(s)
}

impl Combo {
    /// Starts a new combination. Only names that begin with `H` are kept.
    pub fn start(&mut self, name: &str) {
        self.parts.clear();
        self.base = if name.starts_with('H') {
            norm(name)
        } else {
            String::new()
        };
    }

    /// Layer named for `slot`. It goes to the first free place up to that
    /// slot; with none free it replaces the one at the slot.
    pub fn part(&mut self, slot: usize, name: &str) {
        if slot >= LAYERS + 2 {
            return;
        }
        if self.parts.len() <= slot {
            self.parts.resize(slot + 1, String::new());
        }
        let at = (0..=slot)
            .find(|&i| self.parts[i].is_empty())
            .unwrap_or(slot);
        self.parts[at] = norm(name);
    }

    /// Puts `name` in a place of a picture table (0 or 1); it is drawn at
    /// the next composition.
    pub fn set_picture(&mut self, table: usize, slot: i32, name: &str) {
        if let Some(at) = usize::try_from(slot)
            .ok()
            .filter(|&i| i < TABLE_SIZES[table])
        {
            let t = &mut self.tables[table];
            t.resize(TABLE_SIZES[table], String::new());
            t[at] = name.to_string();
        }
    }

    /// Frees a place of a picture table.
    pub fn clear_picture(&mut self, table: usize, slot: i32) {
        if let Ok(at) = usize::try_from(slot) {
            if let Some(p) = self.tables[table].get_mut(at) {
                p.clear();
            }
        }
    }

    /// Copies the background onto the screen and draws the pictures of both
    /// tables over it, the first table first.
    pub fn compose(&mut self) {
        self.on_screen = self
            .tables
            .iter()
            .flatten()
            .filter(|n| !n.is_empty())
            .cloned()
            .collect();
    }

    /// The screen shows the background alone again.
    pub fn clear_screen(&mut self) {
        self.on_screen.clear();
    }

    /// Pictures laid over the background on the screen, bottom first.
    pub fn on_screen(&self) -> &[String] {
        &self.on_screen
    }

    /// The row of the table this combination is, if any: the base and the
    /// first six layers must match the row place by place.
    pub fn entry(&self) -> Option<usize> {
        if self.base.is_empty() {
            return None;
        }
        let part = |i: usize| self.parts.get(i).map_or("", String::as_str);
        ROWS.iter().position(|row| {
            row.layers[0] == self.base
                && (0..LAYERS).all(|i| row.layers.get(i + 1).copied().unwrap_or("") == part(i))
        })
    }

    /// Registers the combination: any picture with a name opens the CG
    /// tab, and a row of the table also sets its flag. Nothing is
    /// registered while the extras flag (H69, which the script sets for a
    /// scene replay) is set.
    pub fn register(&self, vm: &mut Vm) {
        if self.base.is_empty() || vm.var_h(REPLAY_VAR).int() != 0 {
            return;
        }
        vm.set_var_b(CG_TAB_FLAG, 1);
        if let Some(i) = self.entry() {
            vm.set_var_b(ROWS[i].flag as usize, 1);
            vm.set_var_b(SEEN_FLAG, 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn combo(base: &str, parts: &[(usize, &str)]) -> Combo {
        let mut c = Combo::default();
        c.start(base);
        for (slot, name) in parts {
            c.part(*slot, name);
        }
        c
    }

    #[test]
    fn table_rows() {
        assert_eq!(ROWS.len(), 644);
        let flags = |i: usize| ROWS[i].flag;
        assert_eq!((flags(0), ROWS[0].layers), (3200, &["hg090001"][..]));
        assert_eq!(flags(171), 3371);
        assert_eq!(
            ROWS[171].layers,
            ["hg090401_h01", "hg090401_h05", "hg090401_h02"]
        );
        assert_eq!(flags(172), 3372);
        assert_eq!(
            ROWS[172].layers,
            [
                "hg090401_h01",
                "hg090401_h05",
                "hg090401_h02",
                "hg090401_h08"
            ]
        );
        assert_eq!((flags(492), flags(493)), (3692, 3694));
        assert_eq!(ROWS[493].layers, ["hg070502"]);
        assert_eq!((flags(643), ROWS[643].layers), (3845, &["staff04"][..]));
    }

    #[test]
    fn pictures_show_at_composition() {
        let mut c = Combo::default();
        c.set_picture(0, 1, "HG090401_h02.akb");
        c.set_picture(0, 0, "HG090401_h05.akb");
        c.set_picture(1, 2, "HG090401_h09.akb");
        c.set_picture(0, 5, "ignored.akb");
        assert!(c.on_screen().is_empty());
        c.compose();
        assert_eq!(
            c.on_screen(),
            ["HG090401_h05.akb", "HG090401_h02.akb", "HG090401_h09.akb"]
        );
        // A cleared place stays on the screen until the next composition.
        c.clear_picture(0, 1);
        assert_eq!(c.on_screen().len(), 3);
        c.compose();
        assert_eq!(c.on_screen(), ["HG090401_h05.akb", "HG090401_h09.akb"]);
        c.clear_screen();
        assert!(c.on_screen().is_empty());
    }

    #[test]
    fn matching() {
        let c = combo("HG080001_h01.akb", &[(0, "HG080001_h02.akb")]);
        assert_eq!(c.entry(), Some(5));
        assert_eq!(combo("bg01.akb", &[]).entry(), None);
        // Lower case names are not kept as the picture.
        assert_eq!(combo("hg080001_h01.akb", &[]).entry(), None);
        // A layer goes to the first free place up to its slot.
        let c = combo(
            "HG090401_h01.akb",
            &[(2, "HG090401_h05.akb"), (1, "HG090401_h02.akb")],
        );
        assert_eq!(c.entry(), Some(171));
        let c = combo(
            "HG090401_h01.akb",
            &[
                (0, "HG090401_h05.akb"),
                (1, "HG090401_h02.akb"),
                (3, "HG090401_h08.akb"),
            ],
        );
        assert_eq!(c.entry(), Some(172));
        // A longer combination than the row is not the row.
        let c = combo("HG080001_h01.akb", &[(0, "HG080001_h02.akb"), (1, "x")]);
        assert_eq!(c.entry(), None);
    }

    #[test]
    fn a_replay_registers_nothing() {
        let mut vm = Vm::new(silky_format::mes::Profile::default());
        let mut c = combo("HG080001_h01.akb", &[(0, "HG080001_h02.akb")]);
        vm.set_h(REPLAY_VAR, 1);
        c.register(&mut vm);
        assert_eq!((vm.var_b(CG_TAB_FLAG), vm.var_b(SEEN_FLAG)), (0, 0));
        vm.set_h(REPLAY_VAR, 0);
        c.register(&mut vm);
        assert_eq!((vm.var_b(CG_TAB_FLAG), vm.var_b(SEEN_FLAG)), (1, 1));
    }
}
