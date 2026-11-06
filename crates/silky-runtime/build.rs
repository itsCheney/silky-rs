fn main() {
    println!("cargo:rerun-if-changed=vendor/pl_mpeg/shim.c");
    println!("cargo:rerun-if-changed=vendor/pl_mpeg/pl_mpeg.h");
    cc::Build::new()
        .file("vendor/pl_mpeg/shim.c")
        .warnings(false)
        .opt_level(2)
        .compile("silky_pl_mpeg");
}
