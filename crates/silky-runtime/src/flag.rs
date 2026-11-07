//! The `Flag` class (14) of ふゆから、くるる。, read from the executable:
//! the script variables written to and read back from flag files, which
//! keep them across games.
//!
//! A flag file holds the four variable areas of the VM: bytes, words, the
//! raw area and value bank G. The game only uses file 0, its system file:
//!
//! - 0 `(g, raw, words, bytes)`: sizes of the areas.
//! - 2 `(file)`: writes all variables to the file.
//! - 3 `(count, start, file)`: reads bytes `start..start + count` from it.
//! - 6 `(count, start, file)`: the same for bank G.
//! - 11 `(file)`: keeps the bytes from 1000 on in the file, each the larger
//!   of the file's and the game's value (this game's own version of the
//!   call; the engine's keeps all bytes).
//! - 12 `(file)`: reads all variables from the file.
//! - 13 `()`: clears all variables.
//! - 14 `(value, end, start)`: sets bytes `start..end` to `value`.
//!
//! The scripts read the system file at start up (bytes 3900..3999), merge
//! the bytes from 1000 on into it at each new game, then take all bytes and
//! three G values back from it and clear bytes 0..999; after a load they
//! take bytes 1000.. back again. A file that is not there reads as zeros.

use std::collections::BTreeMap;

use silky_vm::{Value, Vm};

/// First byte variable the game keeps in its system file.
const KEPT_BYTES: usize = 1000;

/// The variables of one flag file.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct FlagFile {
    pub bytes: Vec<u8>,
    pub words: Vec<u16>,
    pub raw: Vec<u8>,
    pub g: Vec<Value>,
}

impl FlagFile {
    fn of(vm: &Vm) -> Self {
        Self {
            bytes: vm.bytes.clone(),
            words: vm.words.clone(),
            raw: vm.raw.clone(),
            g: vm.g.clone(),
        }
    }

    /// Without trailing zeros, to keep the system data small.
    fn trimmed(mut self) -> Self {
        fn trim<T: PartialEq + Default>(v: &mut Vec<T>) {
            while v.last().is_some_and(|x| *x == T::default()) {
                v.pop();
            }
        }
        trim(&mut self.bytes);
        trim(&mut self.words);
        trim(&mut self.raw);
        trim(&mut self.g);
        self
    }
}

/// Copies `count` elements from `start` of `from` into `to`, within the
/// size of `to` (the executable stops at its end).
fn copy<T: Clone + Default>(to: &mut [T], from: &[T], start: usize, count: usize) {
    for i in start..(start + count).min(to.len()) {
        to[i] = from.get(i).cloned().unwrap_or_default();
    }
}

/// Carries out `Flag.sub` with `args` (in script order) on the VM and the
/// flag files. `None` for a call this does not cover; `Some(true)` when a
/// file changed.
pub fn call(
    files: &mut BTreeMap<i32, FlagFile>,
    vm: &mut Vm,
    sub: i32,
    args: &[Value],
) -> Option<bool> {
    let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
    let size = |i: usize| n(i).max(0) as usize;
    let empty = FlagFile::default();
    Some(match (sub, args.len()) {
        (0, 4) => {
            vm.bytes.resize(size(3), 0);
            vm.words.resize(size(2), 0);
            vm.raw.resize(size(1), 0);
            vm.g.resize(size(0), Value::default());
            false
        }
        (2, 1) => {
            files.insert(n(0), FlagFile::of(vm).trimmed());
            true
        }
        (3, 3) => {
            let file = files.get(&n(2)).unwrap_or(&empty);
            copy(&mut vm.bytes, &file.bytes, size(1), size(0));
            false
        }
        (6, 3) => {
            let file = files.get(&n(2)).unwrap_or(&empty);
            copy(&mut vm.g, &file.g, size(1), size(0));
            false
        }
        (11, 1) => {
            let old = files.get(&n(0));
            let mut file = old.cloned().unwrap_or_default();
            if file.bytes.len() < vm.bytes.len() {
                file.bytes.resize(vm.bytes.len(), 0);
            }
            for (i, &b) in vm.bytes.iter().enumerate().skip(KEPT_BYTES) {
                file.bytes[i] = file.bytes[i].max(b);
            }
            let file = file.trimmed();
            let changed = old != Some(&file);
            files.insert(n(0), file);
            changed
        }
        (12, 1) => {
            let file = files.get(&n(0)).unwrap_or(&empty);
            let (b, w, r, g) = (vm.bytes.len(), vm.words.len(), vm.raw.len(), vm.g.len());
            copy(&mut vm.bytes, &file.bytes, 0, b);
            copy(&mut vm.words, &file.words, 0, w);
            copy(&mut vm.raw, &file.raw, 0, r);
            copy(&mut vm.g, &file.g, 0, g);
            false
        }
        (13, 0) => {
            vm.bytes.fill(0);
            vm.words.fill(0);
            vm.raw.fill(0);
            vm.g.fill(Value::default());
            false
        }
        (14, 3) => {
            let (start, end) = (size(2), size(1).min(vm.bytes.len()));
            for b in vm.bytes.iter_mut().take(end).skip(start) {
                *b = n(0) as u8;
            }
            false
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use silky_format::mes::Profile;

    fn int(v: &[i32]) -> Vec<Value> {
        v.iter().map(|&i| Value::Int(i)).collect()
    }

    /// The new game sequence of `main.mes`.
    #[test]
    fn kept_across_games() {
        let mut files = BTreeMap::new();
        let mut vm = Vm::new(Profile::default());
        call(&mut files, &mut vm, 0, &int(&[100, 25000, 300, 4096]));
        vm.bytes[5] = 1;
        vm.bytes[3001] = 1;
        assert_eq!(call(&mut files, &mut vm, 11, &int(&[0])), Some(true));
        // A second game that did not reach 3001 keeps it in the file.
        vm.bytes[3001] = 0;
        assert_eq!(call(&mut files, &mut vm, 11, &int(&[0])), Some(false));
        call(&mut files, &mut vm, 3, &int(&[4096, 0, 0]));
        call(&mut files, &mut vm, 14, &int(&[0, 1000, 0]));
        assert_eq!((vm.bytes[5], vm.bytes[3001]), (0, 1));
        assert_eq!(vm.bytes.len(), 4096);
    }
}
