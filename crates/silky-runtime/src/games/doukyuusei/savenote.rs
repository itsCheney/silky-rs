//! The comment of a manual save in 同級生 Remake: typed in the save
//! dialog, shown on the card, and the fixed labels of auto and quick
//! saves. Widths count half-width characters as one column and all others
//! as two; a wide character is never split.
//!
//! Typing on the keyboard (English) takes `0-9`, `@`, `A-Z` and `a-z` up
//! to [`TYPED_COLUMNS`]; committed input method text (Chinese) and a
//! paste go up to [`LIMIT_UNITS`]. The caret is always at the end.

/// The most columns a comment takes (committed text and paste).
pub const LIMIT_UNITS: usize = 40;
/// The most columns typed on the keyboard (English) take.
pub const TYPED_COLUMNS: usize = 20;
/// Columns after which a comment wraps on a card.
pub const WRAP_UNITS: usize = 20;
/// The most columns the card's text takes before it is cut (a line break
/// counts one).
pub const CARD_COLUMNS: usize = 41;
/// Pixels a column takes in the field (size 20 text).
pub const UNIT_PX: i32 = 10;
/// Size of the text on a card, and so the height of a line.
pub const CARD_FONT: u32 = 26;
pub const LINE_PX: i32 = 26;
/// The comment area of a card, relative to the card: left, top, width,
/// height.
pub const CARD_AREA: (i32, i32, i32, i32) = (7, 267, 300, 71);
/// Where the text starts inside the comment area.
pub const CARD_TEXT: (i32, i32) = (20, 9);
/// The text field of the save dialog: left, top, width, height.
pub const FIELD: (i32, i32, u32, u32) = (717, 495, 460, 24);
/// The right end of what the field shows, from its left.
pub const FIELD_CLIP: i32 = 440;
/// The clear button of the dialog: left, top, size.
pub const CLEAR: (i32, i32, i32) = (1159, 483, 48);
/// Milliseconds the caret stays on or off.
pub const BLINK_MS: u64 = 300;
/// The caret is drawn only below this many columns.
pub const CARET_BELOW: usize = 22;
/// An input method's composition is drawn only below this many columns.
pub const COMPOSE_BELOW: usize = 39;

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

/// The start of `text` that fits `max` columns: a wide character that
/// would stand over the end is left out, and so is all after it.
pub fn truncate(text: &str, max: usize) -> String {
    let mut used = 0;
    let mut out = String::new();
    for c in text.chars() {
        used += units(c);
        if used > max {
            break;
        }
        out.push(c);
    }
    out
}

/// Whether the keyboard types `c` (English): digits, `@` and letters.
pub fn typable(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '@'
}

/// Types `input` on the keyboard (English): Backspace (`\u{8}`) takes off
/// the last character, a character of [`typable`] goes to the end if the
/// text stays within [`TYPED_COLUMNS`], others are ignored. Returns
/// whether `text` changed.
pub fn type_ascii(text: &mut String, input: &str) -> bool {
    let mut changed = false;
    for c in input.chars() {
        if c == '\u{8}' {
            changed |= text.pop().is_some();
        } else if typable(c) && width(text) < TYPED_COLUMNS {
            text.push(c);
            changed = true;
        }
    }
    changed
}

/// Adds committed input method text (Chinese) to the end, cut to
/// [`LIMIT_UNITS`]; Backspace takes off the last character and control
/// characters are ignored. Returns whether `text` changed.
pub fn commit(text: &mut String, input: &str) -> bool {
    let before = text.clone();
    for c in input.chars() {
        if c == '\u{8}' {
            text.pop();
        } else if !c.is_control() {
            text.push(c);
        }
    }
    *text = truncate(text, LIMIT_UNITS);
    *text != before
}

/// Ctrl+V: the clipboard's text replaces the whole comment, cut to
/// [`LIMIT_UNITS`] and not filtered. Returns whether `text` changed.
pub fn paste(text: &mut String, clip: &str) -> bool {
    let new = truncate(clip, LIMIT_UNITS);
    let changed = *text != new;
    *text = new;
    changed
}

/// Backspace: takes off the last character. Returns whether there was one.
pub fn backspace(text: &mut String) -> bool {
    text.pop().is_some()
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
/// save, else the comment wrapped, its first [`CARD_COLUMNS`] columns (the
/// line breaks counted), as many lines as fit under the text's top.
pub fn card_lines(slot: &str, comment: &str) -> Vec<String> {
    let mut lines = match label(slot) {
        Some(l) => vec![l],
        None => {
            let wrapped = wrap(comment).join("\n");
            truncate(&wrapped, CARD_COLUMNS)
                .split('\n')
                .map(str::to_string)
                .collect()
        }
    };
    lines.retain(|l| !l.is_empty());
    lines.truncate(((CARD_AREA.3 - CARD_TEXT.1) / LINE_PX) as usize);
    lines
}

/// Whether the caret shows `ms` after the dialog opened.
pub fn caret_on(ms: u64) -> bool {
    (ms / BLINK_MS) % 2 == 0
}

/// The arrow keys of the save dialog: the focus after `key` from `from`
/// (`-1` for none), or `None` when it stays. Ids of `SAVENOTE.AREA`: 0
/// confirm, 1 cancel, [`DONT_ASK`] and [`CLEAR_ID`]. The target must still
/// be in the area list to take the focus.
pub fn arrow(from: i32, key: super::keys::Arrow) -> Option<i32> {
    use super::keys::Arrow::*;
    match (from, key) {
        (-1, _) => Some(0),
        (0, Up) | (1, Up) => Some(CLEAR_ID),
        (0, Down) | (1, Down) => Some(DONT_ASK),
        (0, Right) => Some(1),
        (1, Left) => Some(0),
        (DONT_ASK, Up) => Some(1),
        (CLEAR_ID, Down) => Some(1),
        _ => None,
    }
}

/// The id the program's code expects for the check box and for the clear
/// button.
pub const DONT_ASK: i32 = 0x100;
pub const CLEAR_ID: i32 = 0x200;

#[cfg(test)]
mod tests {
    use super::super::keys::Arrow;
    use super::*;

    #[test]
    fn width_counts_units() {
        assert_eq!(width("abc"), 3);
        assert_eq!(width("あい"), 4);
        assert_eq!(width("aあ"), 3);
    }

    #[test]
    fn truncation_never_splits_a_wide_character() {
        assert_eq!(truncate("abc", 2), "ab");
        assert_eq!(truncate("aあい", 4), "aあ");
        assert_eq!(truncate("aあい", 3), "aあ");
        assert_eq!(truncate("aあb", 2), "a");
        assert_eq!(truncate("", 5), "");
    }

    #[test]
    fn the_keyboard_takes_digits_at_and_letters_to_twenty_columns() {
        let mut t = String::new();
        assert!(type_ascii(&mut t, "a@Z9 -_!あ"));
        assert_eq!(t, "a@Z9");
        assert!(type_ascii(&mut t, &"b".repeat(30)));
        assert_eq!(width(&t), TYPED_COLUMNS);
        assert!(!type_ascii(&mut t, "c"));
        assert!(type_ascii(&mut t, "\u{8}"));
        assert_eq!(width(&t), TYPED_COLUMNS - 1);
        let mut e = String::new();
        assert!(!type_ascii(&mut e, "\u{8}"));
    }

    #[test]
    fn committed_text_goes_to_forty_columns() {
        let mut t = String::new();
        assert!(commit(&mut t, &"あ".repeat(25)));
        assert_eq!(t.chars().count(), 20);
        assert_eq!(width(&t), 40);
        assert!(!commit(&mut t, "a"));
        let mut t = "a".repeat(39);
        commit(&mut t, "あb");
        assert_eq!(t, "a".repeat(39));
        let mut t = "ab".to_string();
        assert!(commit(&mut t, "\n\u{8}c"));
        assert_eq!(t, "ac");
    }

    #[test]
    fn paste_replaces_everything_and_does_not_filter() {
        let mut t = String::from("old comment");
        assert!(paste(&mut t, "a_b\nc"));
        assert_eq!(t, "a_b\nc");
        assert!(!paste(&mut t, "a_b\nc"));
        assert!(paste(&mut t, &"a".repeat(60)));
        assert_eq!(t.len(), 40);
        assert!(paste(&mut t, &"あ".repeat(30)));
        assert_eq!(width(&t), 40);
        assert!(paste(&mut t, ""));
        assert_eq!(t, "");
    }

    #[test]
    fn backspace_takes_the_last_character() {
        let mut t = String::from("aあ");
        assert!(backspace(&mut t));
        assert_eq!(t, "a");
        assert!(backspace(&mut t));
        assert!(!backspace(&mut t));
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
    }

    #[test]
    fn a_card_shows_two_lines_of_the_first_forty_one_columns() {
        // Two lines fit under the top of the text (9 + 2 * 26 < 71).
        let lines = card_lines("save001", &"a".repeat(100));
        assert_eq!(lines, vec!["a".repeat(20), "a".repeat(20)]);
        // The break counts a column: 20 + 1 + 20 = 41.
        let lines = card_lines("save001", &"a".repeat(41));
        assert_eq!(lines, vec!["a".repeat(20), "a".repeat(20)]);
        // A wide character that starts at column 19 makes a line of 21.
        let text = format!("{}あb", "a".repeat(19));
        let lines = card_lines("save001", &text);
        assert_eq!(lines[0], format!("{}あ", "a".repeat(19)));
        assert_eq!(lines[1], "b");
    }

    #[test]
    fn the_caret_blinks_every_three_hundred_ms() {
        assert!(caret_on(0));
        assert!(caret_on(299));
        assert!(!caret_on(300));
        assert!(caret_on(600));
    }

    #[test]
    fn arrows_move_the_focus_without_wrapping() {
        use Arrow::*;
        for k in [Up, Down, Left, Right] {
            assert_eq!(arrow(-1, k), Some(0));
        }
        assert_eq!(arrow(0, Up), Some(CLEAR_ID));
        assert_eq!(arrow(0, Down), Some(DONT_ASK));
        assert_eq!(arrow(0, Left), None);
        assert_eq!(arrow(0, Right), Some(1));
        assert_eq!(arrow(1, Up), Some(CLEAR_ID));
        assert_eq!(arrow(1, Down), Some(DONT_ASK));
        assert_eq!(arrow(1, Left), Some(0));
        assert_eq!(arrow(1, Right), None);
        assert_eq!(arrow(DONT_ASK, Up), Some(1));
        assert_eq!(arrow(DONT_ASK, Down), None);
        assert_eq!(arrow(DONT_ASK, Left), None);
        assert_eq!(arrow(CLEAR_ID, Down), Some(1));
        assert_eq!(arrow(CLEAR_ID, Up), None);
        assert_eq!(arrow(CLEAR_ID, Right), None);
    }
}
