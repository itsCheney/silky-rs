//! The calendar `Game.25` writes into the raw area of the VM, read from
//! the executable: from raw byte 18000, 32 bytes a month, one byte a day
//! (0 past the end of the month). A day holds its weekday, 1 for Monday to
//! 7 for Sunday, plus 8 on a holiday. The scripts count a month's days
//! and take the weekdays from it.

/// First raw byte of January.
pub const START: usize = 18000;
const MONTH_BYTES: usize = 32;

/// Days of each month and the weekday of its first day.
const MONTHS: [(u8, u8); 12] = [
    (31, 4),
    (28, 7),
    (31, 7),
    (30, 2),
    (31, 4),
    (30, 7),
    (31, 2),
    (31, 5),
    (30, 1),
    (31, 3),
    (30, 6),
    (31, 1),
];

/// Holidays as (month, day); 11/24 is the Monday after one on a Sunday.
const HOLIDAYS: [(u8, u8); 15] = [
    (1, 1),
    (1, 15),
    (2, 11),
    (3, 21),
    (4, 29),
    (5, 3),
    (5, 4),
    (5, 5),
    (9, 15),
    (9, 23),
    (10, 10),
    (11, 3),
    (11, 23),
    (11, 24),
    (12, 23),
];

/// The 12 x 32 bytes of the calendar.
pub fn bytes() -> Vec<u8> {
    let mut out = vec![0u8; 12 * MONTH_BYTES];
    for (m, &(days, first)) in MONTHS.iter().enumerate() {
        for d in 0..days {
            let weekday = (first - 1 + d) % 7 + 1;
            let holiday = HOLIDAYS.contains(&(m as u8 + 1, d + 1));
            out[m * MONTH_BYTES + d as usize] = weekday + if holiday { 8 } else { 0 };
        }
    }
    out
}

/// Writes the calendar into the raw area.
pub fn write(raw: &mut Vec<u8>) {
    let bytes = bytes();
    if raw.len() < START + bytes.len() {
        raw.resize(START + bytes.len(), 0);
    }
    raw[START..START + bytes.len()].copy_from_slice(&bytes);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn like_the_executable() {
        let b = bytes();
        let month = |m: usize| &b[(m - 1) * MONTH_BYTES..m * MONTH_BYTES];
        assert_eq!(
            &month(5)[..8],
            &[4, 5, 14, 15, 9, 2, 3, 4],
            "May 1 is a Thursday, 3 to 5 are holidays"
        );
        assert_eq!(month(2)[10], 11);
        assert_eq!(month(2)[28], 0);
        assert_eq!(&month(8)[..3], &[5, 6, 7]);
        assert_eq!(month(11).iter().filter(|&&d| d != 0).count(), 30);
        assert_eq!(month(12)[22], 10);
        assert_eq!(&month(11)[22..24], &[15, 9]);
    }
}
