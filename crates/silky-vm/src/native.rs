//! Engine function classes called by opcode 0x18 (`native`).
//!
//! The script pushes the arguments, then a sub-command number, then the
//! class number, and executes `native`. The interpreter pops the class; the
//! class pops its sub-command and arguments. Numbers and class names come
//! from the engine's native dispatcher and its run-time type information.
//! Class 29 is game specific: the engine hands it to an object the game
//! installs, which pops a command number, usually a sub-command, then
//! arguments.

/// Class numbers and names. Numbers 4 to 9 are not used.
pub const CLASSES: &[(i32, &str)] = &[
    (1, "Select"),
    (2, "MesWnd"),
    (3, "Shake"),
    (10, "Font"),
    (11, "Value"),
    (12, "Mouse"),
    (13, "Anime"),
    (14, "Flag"),
    (15, "Music"),
    (16, "Effect"),
    (17, "HEffect"),
    (18, "Voice"),
    (19, "Sprite"),
    (20, "Screen"),
    (21, "Script"),
    (22, "BackLog"),
    (23, "Movie"),
    (24, "OneMes"),
    (25, "Xml"),
    (26, "Ui"),
    (27, "Save"),
    (28, "Wait"),
    (29, "Game"),
];

pub use silky_format::game::Title;

pub const SELECT: i32 = 1;
pub const SCRIPT: i32 = 21;
pub const GAME: i32 = 29;

/// `Script` sub-commands, each with one string argument (the file name).
pub mod script {
    /// Replace the running script.
    pub const JUMP: i32 = 0;
    /// Run a script, returning here when it executes `ret`.
    pub const CALL: i32 = 1;
    /// Run a function library (its `fdef`s), returning here.
    pub const LIBRARY: i32 = 2;
}

/// Name of a class number.
pub fn class_name(class: i32) -> Option<&'static str> {
    CLASSES.iter().find(|c| c.0 == class).map(|c| c.1)
}

/// Whether a native pushes a result. `sub` is the sub-command, or for class
/// [`GAME`] the command number. The engine classes are shared; class
/// [`GAME`] is written for each game. Natives not listed push nothing.
///
/// ふゆから、くるる。: from how its scripts use the results.
/// きまぐれテンプテーション2: Game commands whose handler in the exe pushes
/// onto the script stack.
pub fn returns_value(title: Title, class: i32, sub: i32) -> bool {
    if class == GAME {
        return match title {
            Title::Fuyukuru => matches!(sub, 20 | 30 | 31 | 32 | 110 | 223 | 1006 | 1009),
            // The common commands that return a value.
            Title::Unknown => matches!(sub, 20 | 30 | 31 | 32 | 110 | 223),
            // 景の海のアペイリア: provisional, from how its scripts use the
            // results.
            Title::Apeiria => matches!(sub, 30..=32 | 110 | 960 | 990 | 1009 | 1016),
            Title::Kimaten2 => matches!(
                sub,
                20 | 24
                    | 30
                    | 31
                    | 32
                    | 34
                    | 42
                    | 43
                    | 44
                    | 51
                    | 110
                    | 959
                    | 990
                    | 1006
                    | 1009
                    | 1010
            ),
        };
    }
    // 景の海のアペイリア: Movie.2 closes the movie; Movie.3, BackLog.4 to
    // 7, OneMes.5 and Save.3 and 8 push a result (read from the
    // executable).
    if title == Title::Apeiria {
        match (class, sub) {
            (23, 2) => return false,
            (23, 3) | (22, 4..=7) | (24, 5) | (27, 3 | 8) => return true,
            _ => {}
        }
    }
    matches!(
        (class, sub),
        (1, 2)
            | (2, 3)
            | (12, 4)
            | (16..=18, 7)
            | (19, 25)
            | (20, 4)
            | (23, 2)
            | (25, 0)
            | (26, 1)
            | (27, 2)
    )
}
