pub(crate) mod breath;
mod cgdata;
pub(crate) mod comic;
pub(crate) mod commands;
pub mod omake;
pub(crate) mod title;
pub(crate) mod vo;
pub(crate) mod window;

/// The game's movies are plain `do-jin*.dat` files beside the program, not
/// in an archive. Only read when the movie decoder for them is built in.
pub(crate) fn movie_file(root: &std::path::Path, name: &str) -> Option<Vec<u8>> {
    if !cfg!(feature = "ffmpeg") {
        return None;
    }
    let exact = root.join(name);
    let path = if exact.is_file() {
        exact
    } else {
        std::fs::read_dir(root)
            .ok()?
            .flatten()
            .map(|e| e.path())
            .find(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.eq_ignore_ascii_case(name))
            })?
    };
    std::fs::read(path).ok()
}
