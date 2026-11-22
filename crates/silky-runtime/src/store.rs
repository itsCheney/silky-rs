//! The `Save` class (27) of the engine: a store of variables the scripts
//! use to remember what the screen shows (which pictures are up) so the
//! scene can be rebuilt after a load.
//!
//! The store has several tables, picked by the first argument of `2`, `4`
//! and `5`; each maps a key to a number:
//!
//! - 2 `(table, key, default)`: reads a value, the default if the key is
//!   not there. Returns it.
//! - 4 `(table, key, value, 0)`: writes a value.
//! - 5 `(table, key)`: deletes a key from one table.
//! - 6 `(key)`: deletes a key from every table.
//!
//! It is part of the scene, so saves keep it.

use std::collections::BTreeMap;

use silky_vm::Value;

/// The tables of the store.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Store {
    tables: BTreeMap<i32, BTreeMap<i32, i32>>,
}

impl Store {
    /// One `Save` call: `None` if it is not understood, else the value it
    /// returns, if it returns one.
    pub fn call(&mut self, sub: i32, args: &[Value]) -> Option<Option<i32>> {
        let n = |i: usize| args.get(i).map(Value::int).unwrap_or(0);
        match (sub, args.len()) {
            (2, 3) => {
                let v = self
                    .tables
                    .get(&n(0))
                    .and_then(|t| t.get(&n(1)))
                    .copied()
                    .unwrap_or(n(2));
                Some(Some(v))
            }
            (4, 4) => {
                self.tables.entry(n(0)).or_default().insert(n(1), n(2));
                Some(None)
            }
            (5, 2) => {
                if let Some(t) = self.tables.get_mut(&n(0)) {
                    t.remove(&n(1));
                }
                Some(None)
            }
            (6, 1) => {
                for t in self.tables.values_mut() {
                    t.remove(&n(0));
                }
                Some(None)
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ints(v: &[i32]) -> Vec<Value> {
        v.iter().map(|&i| Value::Int(i)).collect()
    }

    #[test]
    fn write_read_delete() {
        let mut s = Store::default();
        assert_eq!(s.call(2, &ints(&[0, 5, 0])), Some(Some(0)));
        assert_eq!(s.call(2, &ints(&[0, 5, 9])), Some(Some(9)));
        s.call(4, &ints(&[0, 5, 3, 0]));
        s.call(4, &ints(&[2, 5, 7, 0]));
        assert_eq!(s.call(2, &ints(&[0, 5, 0])), Some(Some(3)));
        assert_eq!(s.call(2, &ints(&[2, 5, 0])), Some(Some(7)));
        s.call(5, &ints(&[0, 5]));
        assert_eq!(s.call(2, &ints(&[0, 5, 0])), Some(Some(0)));
        assert_eq!(s.call(2, &ints(&[2, 5, 0])), Some(Some(7)));
        s.call(4, &ints(&[0, 5, 1, 0]));
        s.call(6, &ints(&[5]));
        assert_eq!(s.call(2, &ints(&[0, 5, 0])), Some(Some(0)));
        assert_eq!(s.call(2, &ints(&[2, 5, 0])), Some(Some(0)));
    }

    #[test]
    fn survives_serde() {
        let mut s = Store::default();
        s.call(4, &ints(&[1, 4, 2, 0]));
        let j = serde_json::to_string(&s).unwrap();
        let mut t: Store = serde_json::from_str(&j).unwrap();
        assert_eq!(t, s);
        assert_eq!(t.call(2, &ints(&[1, 4, 0])), Some(Some(2)));
    }
}
