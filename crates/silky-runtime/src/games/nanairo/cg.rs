//! The CG registry of なないろリンカネーション (`Game.14`, `Game.22`).
//!
//! A script names the picture (`Game.14.10`) and its layers in slot order
//! (`Game.14.1`), and the engine registers the combination when the scene
//! is committed: if it is a row of [`ENTRIES`], byte flag 3200 + row is
//! set, which the CG mode reads.

use silky_vm::Vm;

pub use super::cg_table::ENTRIES;

/// First byte flag of the table.
pub const FIRST_FLAG: usize = 3200;
/// Bytes the engine also sets when a combination is registered.
const ANY_FLAGS: [usize; 2] = [3001, 3002];

/// The picture and layers named so far.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Combo {
    base: String,
    parts: Vec<String>,
}

fn norm(name: &str) -> String {
    let s = name.to_ascii_lowercase();
    s.strip_suffix(".akb").map(str::to_string).unwrap_or(s)
}

impl Combo {
    /// Starts a new combination. Only names that begin with `H` are kept.
    pub fn start(&mut self, name: &str) {
        self.parts.clear();
        self.base = if name.starts_with(['H', 'h']) {
            norm(name)
        } else {
            String::new()
        };
    }

    /// Layer `slot` of the combination.
    pub fn part(&mut self, slot: usize, name: &str) {
        if slot < 8 {
            if self.parts.len() <= slot {
                self.parts.resize(slot + 1, String::new());
            }
            self.parts[slot] = norm(name);
        }
    }

    /// The row of the table this combination is, if any.
    pub fn entry(&self) -> Option<usize> {
        if self.base.is_empty() {
            return None;
        }
        let names: Vec<&str> = std::iter::once(self.base.as_str())
            .chain(
                self.parts
                    .iter()
                    .map(String::as_str)
                    .filter(|p| !p.is_empty()),
            )
            .collect();
        ENTRIES.iter().position(|row| **row == *names)
    }

    /// Registers the combination: sets the row's flag.
    pub fn register(&self, vm: &mut Vm) {
        if let Some(i) = self.entry() {
            vm.set_var_b(FIRST_FLAG + i, 1);
            for f in ANY_FLAGS {
                vm.set_var_b(f, 1);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_rows() {
        assert_eq!(ENTRIES.len(), 493);
        let mut c = Combo::default();
        c.start("HG080001_h01.akb");
        c.part(0, "HG080001_h02.akb");
        assert_eq!(c.entry(), Some(5));
        c.start("bg01.akb");
        assert_eq!(c.entry(), None);
    }
}
