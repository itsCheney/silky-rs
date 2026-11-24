//! Each girl's records against what the original program builds for the
//! same story state.

use super::cases::CASES;
use crate::games::doukyuusei::flaglist::{records, Env};

fn pairs(s: &str) -> impl Iterator<Item = (usize, u32)> + '_ {
    s.split_whitespace().map(|kv| {
        let (i, v) = kv.split_once(':').unwrap();
        (i.parse().unwrap(), v.parse().unwrap())
    })
}

#[test]
fn records_match_the_original() {
    for (state, want) in CASES {
        let p: Vec<&str> = state.split('|').collect();
        let head: Vec<i32> = p[0]
            .split_whitespace()
            .map(|x| x.parse().unwrap())
            .collect();
        let mut b = vec![0u8; 0x2400];
        let mut h = vec![0u16; 0x800];
        for (i, v) in pairs(p[1]) {
            b[i] = v as u8;
        }
        for (i, v) in pairs(p[2]) {
            h[i] = v as u16;
        }
        let env = Env {
            bytes: &b,
            words: &h,
            date: head[1],
            time: head[2],
            cash: head[3],
        };
        let got: Vec<String> = records(head[0] as usize, &env)
            .iter()
            .map(|r| {
                format!(
                    "{} {} {} {} {} {} {} {} {} {} {} {}",
                    r.script,
                    r.place,
                    r.d0,
                    r.d1,
                    r.t0,
                    r.t1,
                    r.stage,
                    r.sub,
                    r.cost,
                    r.f0,
                    r.state,
                    r.reason
                )
            })
            .collect();
        assert_eq!(got.join(";"), *want, "state {state}");
    }
}
