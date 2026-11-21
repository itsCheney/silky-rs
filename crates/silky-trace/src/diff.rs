//! Where two traces part.
//!
//! Both traces go through the same [`Projection`] (calls kept, fields
//! compared), then are walked side by side. At a difference the diff
//! aligns the next calls of both sides (longest common subsequence over a
//! window) and reports the calls up to the next one they share: calls one
//! side lacks, or calls that differ.
//! Calls are matched by order only: ticks of two runs, or of this runtime
//! and the original, need not line up, so compare them only when they do.

use std::fmt;

use crate::{show, Arg, Call};

/// Fields of a call left out of the comparison. The name is always
/// compared.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Fields {
    pub tick: bool,
    pub script: bool,
    pub pc: bool,
    pub args: bool,
    pub result: bool,
}

impl Fields {
    /// From a comma separated list: `tick,pc`.
    pub fn parse(list: &str) -> Result<Self, String> {
        let mut f = Fields::default();
        for name in list.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            match name {
                "tick" => f.tick = true,
                "script" => f.script = true,
                "pc" => f.pc = true,
                "args" => f.args = true,
                "result" => f.result = true,
                _ => {
                    return Err(format!(
                        "unknown field {name} (tick, script, pc, args, result)"
                    ))
                }
            }
        }
        Ok(f)
    }
}

/// Which calls take part and how they are compared.
#[derive(Debug, Clone, Default)]
pub struct Projection {
    pub ignore: Fields,
    /// Keep only these calls: a class (`Sprite`), a call (`Game.24`) or a
    /// kind (`message`). Empty keeps all.
    pub only: Vec<String>,
    /// Leave these out (same patterns as `only`).
    pub skip: Vec<String>,
    /// Start each side at its first call that matches: `message`, or with
    /// the first argument, `message:120` or `Script.0:FK001.MES`.
    pub from: Option<String>,
}

fn matches(name: &str, pattern: &str) -> bool {
    name == pattern
        || name
            .strip_prefix(pattern)
            .is_some_and(|rest| rest.starts_with('.'))
}

fn arg_text(a: &Arg) -> String {
    match a {
        Arg::Int(v) => v.to_string(),
        Arg::Str(s) => s.clone(),
    }
}

fn starts_here(call: &Call, from: &str) -> bool {
    match from.split_once(':') {
        Some((name, arg)) => {
            matches(&call.call, name) && call.args.first().is_some_and(|a| arg_text(a) == arg)
        }
        None => matches(&call.call, from),
    }
}

impl Projection {
    fn keeps(&self, call: &Call) -> bool {
        (self.only.is_empty() || self.only.iter().any(|p| matches(&call.call, p)))
            && !self.skip.iter().any(|p| matches(&call.call, p))
    }

    /// The compared form of a call.
    fn key(&self, call: &Call) -> Call {
        let i = &self.ignore;
        Call {
            tick: if i.tick { None } else { call.tick },
            script: if i.script { None } else { call.script.clone() },
            pc: if i.pc { None } else { call.pc },
            call: call.call.clone(),
            args: if i.args {
                Vec::new()
            } else {
                call.args.clone()
            },
            result: if i.result { None } else { call.result.clone() },
        }
    }

    /// The calls taking part: (index in the trace, compared form).
    fn apply(&self, calls: &[Call]) -> Vec<(usize, Call)> {
        let start = match &self.from {
            Some(from) => calls
                .iter()
                .position(|c| starts_here(c, from))
                .unwrap_or(calls.len()),
            None => 0,
        };
        calls
            .iter()
            .enumerate()
            .skip(start)
            .filter(|(_, c)| self.keeps(c))
            .map(|(i, c)| (i, self.key(c)))
            .collect()
    }
}

/// One place where the traces part.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Divergence {
    /// Position in each projected list where they part.
    pub at: (usize, usize),
    /// Index of that call in each trace, if the side has one there.
    pub call_index: (Option<usize>, Option<usize>),
    /// Calls before it that both sides share (compared form, expected's
    /// index in its trace).
    pub before: Vec<(usize, Call)>,
    /// The differing calls: expected's and actual's, up to the point where
    /// they agree again (or a few when they never do).
    pub expected: Vec<(usize, Call)>,
    pub actual: Vec<(usize, Call)>,
    /// Whether the two agree again after `expected` and `actual`.
    pub rejoined: bool,
}

/// The result of a diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Calls taking part on each side after the projection.
    pub compared: (usize, usize),
    /// Calls the two share before the first divergence.
    pub same_prefix: usize,
    pub divergences: Vec<Divergence>,
    /// Whether the search stopped at `max` divergences with more to go.
    pub stopped: bool,
}

impl Report {
    pub fn same(&self) -> bool {
        self.divergences.is_empty()
    }
}

/// Options of the walk.
#[derive(Debug, Clone, Copy)]
pub struct Walk {
    /// Calls shown before a divergence.
    pub context: usize,
    /// Divergences reported before stopping.
    pub max: usize,
    /// Calls on each side aligned at a divergence to find where the two
    /// agree again.
    pub window: usize,
}

impl Default for Walk {
    fn default() -> Self {
        Self {
            context: 3,
            max: 1,
            window: 200,
        }
    }
}

/// Where two lists that differ at their first item agree again: the
/// offsets of the first shared item on the best alignment of the first
/// `window` items of each, or of both ends when only those are left.
fn rejoin(e: &[(usize, Call)], a: &[(usize, Call)], window: usize) -> Option<(usize, usize)> {
    let (n, m) = (e.len().min(window), a.len().min(window));
    // lcs[x][y]: longest common subsequence of e[x..n] and a[y..m].
    let w = m + 1;
    let mut lcs = vec![0u16; (n + 1) * w];
    for x in (0..n).rev() {
        for y in (0..m).rev() {
            lcs[x * w + y] = if e[x].1 == a[y].1 {
                lcs[(x + 1) * w + y + 1] + 1
            } else {
                lcs[(x + 1) * w + y].max(lcs[x * w + y + 1])
            };
        }
    }
    let (mut x, mut y) = (0, 0);
    while x < n && y < m {
        if e[x].1 == a[y].1 {
            return Some((x, y));
        }
        if lcs[(x + 1) * w + y] >= lcs[x * w + y + 1] {
            x += 1;
        } else {
            y += 1;
        }
    }
    // Nothing shared in the window: they agree again only if what is left
    // on both sides ends here.
    (n == e.len() && m == a.len()).then_some((n, m))
}

/// Compares `actual` against `expected`.
pub fn diff(expected: &[Call], actual: &[Call], projection: &Projection, walk: Walk) -> Report {
    let e = projection.apply(expected);
    let a = projection.apply(actual);
    let same = |i: usize, j: usize| e[i].1 == a[j].1;
    let (mut i, mut j) = (0, 0);
    let mut divergences = Vec::new();
    let mut same_prefix = None;
    let mut stopped = false;
    loop {
        while i < e.len() && j < a.len() && same(i, j) {
            i += 1;
            j += 1;
        }
        if i == e.len() && j == a.len() {
            break;
        }
        if divergences.len() == walk.max {
            stopped = true;
            break;
        }
        same_prefix.get_or_insert(i.min(j));
        let rejoin = rejoin(&e[i..], &a[j..], walk.window).map(|(x, y)| (i + x, j + y));
        let shown = |list: &[(usize, Call)], from: usize, to: usize| list[from..to].to_vec();
        let (ti, tj) = rejoin.unwrap_or(((i + 5).min(e.len()), (j + 5).min(a.len())));
        divergences.push(Divergence {
            at: (i, j),
            call_index: (e.get(i).map(|c| c.0), a.get(j).map(|c| c.0)),
            before: shown(&e, i.saturating_sub(walk.context), i),
            expected: shown(&e, i, ti),
            actual: shown(&a, j, tj),
            rejoined: rejoin.is_some(),
        });
        match rejoin {
            Some((ni, nj)) => (i, j) = (ni, nj),
            None => break,
        }
    }
    Report {
        compared: (e.len(), a.len()),
        same_prefix: same_prefix.unwrap_or(e.len().min(a.len())),
        divergences,
        stopped,
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (ne, na) = self.compared;
        if self.same() {
            return writeln!(f, "same: {ne} calls compared");
        }
        writeln!(
            f,
            "compared {ne} expected and {na} actual calls; the first {} agree",
            self.same_prefix
        )?;
        let line = |f: &mut fmt::Formatter<'_>, mark: &str, (i, c): &(usize, Call)| {
            writeln!(f, "  {mark} #{i} {}", show(c))
        };
        for (n, d) in self.divergences.iter().enumerate() {
            writeln!(
                f,
                "divergence {} at compared call {} / {}:",
                n + 1,
                d.at.0,
                d.at.1
            )?;
            for c in &d.before {
                line(f, " ", c)?;
            }
            for c in &d.expected {
                line(f, "-", c)?;
            }
            for c in &d.actual {
                line(f, "+", c)?;
            }
            if d.rejoined {
                writeln!(
                    f,
                    "  then they agree again ({} expected, {} actual calls differ)",
                    d.expected.len(),
                    d.actual.len()
                )?;
            } else {
                writeln!(f, "  no agreement within the search window")?;
            }
        }
        if self.stopped {
            writeln!(f, "more divergences follow (raise --max to see them)")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(name: &str, arg: i32) -> Call {
        Call {
            tick: None,
            script: Some("A.MES".into()),
            pc: Some(arg as u32),
            call: name.into(),
            args: vec![Arg::Int(arg)],
            result: None,
        }
    }

    fn run(e: &[Call], a: &[Call], max: usize) -> Report {
        let walk = Walk {
            max,
            ..Walk::default()
        };
        assert_eq!(walk.window, 200);
        diff(e, a, &Projection::default(), walk)
    }

    #[test]
    fn same_traces() {
        let t = [c("Sprite.1", 1), c("message", 2)];
        let r = run(&t, &t, 1);
        assert!(r.same());
        assert_eq!(r.to_string(), "same: 2 calls compared\n");
    }

    #[test]
    fn extra_and_changed_calls_rejoin() {
        let e = [
            c("A.0", 1),
            c("A.0", 2),
            c("A.0", 3),
            c("A.0", 4),
            c("A.0", 5),
            c("A.0", 6),
        ];
        // One extra call after 1, and 5 changed.
        let a = [
            c("A.0", 1),
            c("B.0", 9),
            c("A.0", 2),
            c("A.0", 3),
            c("A.0", 4),
            c("A.0", 7),
            c("A.0", 6),
        ];
        let r = run(&e, &a, 5);
        assert_eq!(r.same_prefix, 1);
        assert_eq!(r.divergences.len(), 2);
        let d = &r.divergences[0];
        assert_eq!(d.at, (1, 1));
        assert!(d.expected.is_empty());
        assert_eq!(d.actual, [(1, c("B.0", 9))]);
        assert!(d.rejoined);
        let d = &r.divergences[1];
        assert_eq!(d.expected, [(4, c("A.0", 5))]);
        assert_eq!(d.actual, [(5, c("A.0", 7))]);
        let text = r.to_string();
        assert!(
            text.contains("  - #4 A.MES:5 A.0(5)\n  + #5 A.MES:7 A.0(7)\n"),
            "{text}"
        );

        // Stops at --max.
        let r = run(&e, &a, 1);
        assert_eq!(r.divergences.len(), 1);
        assert!(r.stopped);
    }

    #[test]
    fn missing_call_then_changed_call_are_two_divergences() {
        let e = [
            c("S.1", 1),
            c("M.3", 2),
            c("m", 3),
            c("G.20", 4),
            c("X.0", 5),
        ];
        let a = [c("S.1", 1), c("m", 3), c("G.20", 6), c("X.0", 5)];
        let r = run(&e, &a, 5);
        assert_eq!(r.divergences.len(), 2, "{r}");
        assert_eq!(r.divergences[0].expected, [(1, c("M.3", 2))]);
        assert!(r.divergences[0].actual.is_empty());
        assert_eq!(r.divergences[1].expected, [(3, c("G.20", 4))]);
        assert_eq!(r.divergences[1].actual, [(2, c("G.20", 6))]);
    }

    #[test]
    fn no_agreement_in_the_window() {
        let e: Vec<Call> = (0..10).map(|i| c("A.0", i)).collect();
        let a: Vec<Call> = (0..10).map(|i| c("B.0", i)).collect();
        let walk = Walk {
            window: 4,
            ..Walk::default()
        };
        let r = diff(&e, &a, &Projection::default(), walk);
        assert_eq!(r.divergences.len(), 1);
        assert!(!r.divergences[0].rejoined);
        assert_eq!(r.divergences[0].expected.len(), 5);
    }

    #[test]
    fn shorter_side_and_projection() {
        let e = [
            c("Sprite.1", 1),
            c("MesWnd.3", 2),
            c("message", 3),
            c("Sprite.2", 4),
        ];
        let a = [c("Sprite.1", 1), c("message", 3)];
        let r = run(&e, &a, 3);
        assert_eq!(r.divergences.len(), 2, "{r}");
        assert_eq!(r.divergences[0].expected, [(1, c("MesWnd.3", 2))]);
        assert_eq!(r.divergences[1].expected, [(3, c("Sprite.2", 4))]);
        assert!(r.divergences[1].actual.is_empty() && r.divergences[1].rejoined);

        // Leaving out MesWnd and starting at the message, then only the
        // Sprite class and with pc and args ignored.
        let p = Projection {
            skip: vec!["MesWnd".into(), "Sprite".into()],
            ..Projection::default()
        };
        assert!(diff(&e[..3], &a, &p, Walk::default()).same());
        let p = Projection {
            from: Some("message:3".into()),
            only: vec!["message".into()],
            ..Projection::default()
        };
        assert!(diff(&e, &a, &p, Walk::default()).same());
        let p = Projection {
            only: vec!["Sprite".into()],
            ignore: Fields::parse("pc,args").unwrap(),
            ..Projection::default()
        };
        let a2 = [c("Sprite.1", 8), c("Sprite.2", 9)];
        assert!(diff(&e, &a2, &p, Walk::default()).same());
        assert!(Fields::parse("pc,nope").is_err());
        // `Sprite` does not match `SpriteX.1`.
        assert!(!matches("SpriteX.1", "Sprite"));
    }
}
