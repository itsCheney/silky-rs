//! The comment of a manual save in 同級生 Remake: typed in the save
//! dialog, shown on the card, and the fixed labels of auto and quick
//! saves. Widths count half-width characters as one unit and all others as
//! two.

/// The most width units a comment takes.
pub const LIMIT_UNITS: usize = 40;
/// The most UTF-16 units a stored comment takes (one is kept for the
/// terminator).
pub const LIMIT_UTF16: usize = 39;
/// Width units after which a comment wraps on a card.
pub const WRAP_UNITS: usize = 20;
/// Pixels a width unit takes in the field and on cards (size 20 text).
pub const UNIT_PX: i32 = 10;
/// Height of a line of the comment on a card (a guess; the original's
/// spacing is not settled).
pub const LINE_PX: i32 = 23;
/// The comment area of a card, relative to the card: left, top, width,
/// height.
pub const CARD_AREA: (i32, i32, i32, i32) = (7, 267, 300, 71);
/// The text field of the save dialog: left, top, width, height.
pub const FIELD: (i32, i32, u32, u32) = (717, 495, 460, 24);
/// The clear button of the dialog: left, top, size.
pub const CLEAR: (i32, i32, i32) = (1159, 483, 48);
/// Milliseconds the caret stays on or off.
pub const BLINK_MS: u64 = 300;

/// Width units of a character.
pub fn units(c: char) -> usize {
    if c.is_ascii() {
        1
    } else {
        2
    }
}

/// Width units of a text.
pub fn width(text: &str) -> usize {
    text.chars().map(units).sum()
}

fn utf16(text: &str) -> usize {
    text.chars().map(char::len_utf16).sum()
}

/// Types `input` at the end of `text`: Backspace (`\u{8}`) deletes the
/// last character, other control characters are ignored, and a character
/// that would take the width over [`LIMIT_UNITS`] (or the stored length
/// over [`LIMIT_UTF16`]) is dropped. Returns whether `text` changed.
pub fn type_text(text: &mut String, input: &str) -> bool {
    let mut changed = false;
    for c in input.chars() {
        if c == '\u{8}' {
            changed |= text.pop().is_some();
        } else if !c.is_control()
            && width(text) + units(c) <= LIMIT_UNITS
            && utf16(text) + c.len_utf16() <= LIMIT_UTF16
        {
            text.push(c);
            changed = true;
        }
    }
    changed
}

/// Breaks a comment into the lines of a card: a line ends once its width
/// reaches [`WRAP_UNITS`]; `\n` also ends one.
pub fn wrap(text: &str) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    let mut count = 0;
    for c in text.chars() {
        if c == '\n' {
            lines.push(std::mem::take(&mut line));
            count = 0;
            continue;
        }
        line.push(c);
        count += units(c);
        if count >= WRAP_UNITS {
            lines.push(std::mem::take(&mut line));
            count = 0;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

/// The label of an auto or quick save card (`auto03`, `quick`), or `None`
/// for other slots.
pub fn label(slot: &str) -> Option<String> {
    if let Some(n) = slot.strip_prefix("auto") {
        return Some(format!("AUTO SAVE {:02}", n.parse::<u32>().ok()?));
    }
    (slot == crate::save::QUICK).then(|| "QUICK SAVE 01".to_string())
}

/// The lines drawn in a card's comment area: the label of an auto or quick
/// save, else the comment wrapped, as many lines as fit.
pub fn card_lines(slot: &str, comment: &str) -> Vec<String> {
    let mut lines = match label(slot) {
        Some(l) => vec![l],
        None => wrap(comment),
    };
    lines.truncate((CARD_AREA.3 / LINE_PX) as usize);
    lines
}

/// Whether the caret shows `ms` after the dialog opened.
pub fn caret_on(ms: u64) -> bool {
    (ms / BLINK_MS) % 2 == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn width_counts_units() {
        assert_eq!(width("abc"), 3);
        assert_eq!(width("あい"), 4);
        assert_eq!(width("aあ"), 3);
    }

    #[test]
    fn typing_stops_at_forty_units() {
        let mut t = String::new();
        assert!(type_text(&mut t, &"あ".repeat(25)));
        assert_eq!(t.chars().count(), 20);
        assert_eq!(width(&t), 40);
        assert!(!type_text(&mut t, "a"));
        let mut t = String::new();
        type_text(&mut t, &"a".repeat(60));
        assert_eq!(t.len(), 39);
    }

    #[test]
    fn a_wide_character_that_does_not_fit_is_refused_but_a_narrow_one_is_not() {
        let mut t = "a".repeat(39);
        type_text(&mut t, "あb");
        assert_eq!(t.len(), 39);
        let mut t = "a".repeat(37);
        type_text(&mut t, "あb");
        assert_eq!(width(&t), 40);
    }

    #[test]
    fn backspace_and_control_characters() {
        let mut t = String::from("ab");
        assert!(type_text(&mut t, "\u{8}c\n\t"));
        assert_eq!(t, "ac");
        assert!(type_text(&mut t, "\u{8}\u{8}"));
        assert!(!type_text(&mut t, "\u{8}"));
        assert_eq!(t, "");
    }

    #[test]
    fn wrapping_at_twenty_units() {
        let text = "a".repeat(45);
        let lines = wrap(&text);
        assert_eq!(lines, vec!["a".repeat(20), "a".repeat(20), "a".repeat(5)]);
        let lines = wrap(&"あ".repeat(12));
        assert_eq!(lines, vec!["あ".repeat(10), "あ".repeat(2)]);
        assert_eq!(wrap("a\nb"), vec!["a", "b"]);
        assert!(wrap("").is_empty());
    }

    #[test]
    fn labels_of_auto_and_quick_saves() {
        assert_eq!(label("auto01").as_deref(), Some("AUTO SAVE 01"));
        assert_eq!(label("auto20").as_deref(), Some("AUTO SAVE 20"));
        assert_eq!(label("quick").as_deref(), Some("QUICK SAVE 01"));
        assert_eq!(label("save001"), None);
        assert_eq!(card_lines("auto07", "x"), vec!["AUTO SAVE 07"]);
        assert_eq!(card_lines("save001", "hi"), vec!["hi"]);
        assert_eq!(card_lines("save001", &"a".repeat(100)).len(), 3);
    }

    #[test]
    fn the_caret_blinks_every_three_hundred_ms() {
        assert!(caret_on(0));
        assert!(caret_on(299));
        assert!(!caret_on(300));
        assert!(caret_on(600));
    }
}
