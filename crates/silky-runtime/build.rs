use std::path::{Path, PathBuf};

fn main() {
    println!("cargo:rerun-if-changed=vendor/pl_mpeg/shim.c");
    println!("cargo:rerun-if-changed=vendor/pl_mpeg/pl_mpeg.h");
    cc::Build::new()
        .file("vendor/pl_mpeg/shim.c")
        .warnings(false)
        .opt_level(2)
        .compile("silky_pl_mpeg");

    println!("cargo:rerun-if-env-changed=SILKY_FFMPEG_DIR");
    if std::env::var_os("CARGO_FEATURE_FFMPEG").is_some() {
        ffmpeg();
    }
}

/// The `ffmpeg` feature: the shim over FFmpeg's libavformat and libavcodec,
/// linked statically from the prefix `tools/ffmpeg/build.sh` installs into.
fn ffmpeg() {
    println!("cargo:rerun-if-changed=ffmpeg/shim.c");
    let dir = match std::env::var_os("SILKY_FFMPEG_DIR") {
        Some(d) => PathBuf::from(d),
        None => Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/ffmpeg"),
    };
    if !dir.join("lib/libavformat.a").exists() || !dir.join("include/libavformat").exists() {
        panic!(
            "the `ffmpeg` feature needs FFmpeg's static libraries in {}: \
             build them with tools/ffmpeg/build.sh, or point SILKY_FFMPEG_DIR \
             at the prefix they were installed into",
            dir.display()
        );
    }
    cc::Build::new()
        .file("ffmpeg/shim.c")
        .include(dir.join("include"))
        .warnings(false)
        .opt_level(2)
        .compile("silky_ffmpeg_shim");
    println!(
        "cargo:rustc-link-search=native={}",
        dir.join("lib").display()
    );
    for lib in ["avformat", "avcodec", "avutil"] {
        println!("cargo:rustc-link-lib=static={lib}");
    }
    if std::env::var("CARGO_CFG_TARGET_FAMILY").as_deref() == Ok("unix") {
        println!("cargo:rustc-link-lib=m");
        println!("cargo:rustc-link-lib=pthread");
    }
}
