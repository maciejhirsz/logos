use rustc_version::{version_meta, Version};

fn main() {
    let version_meta = version_meta().expect("Could not get Rust version");

    let rustc_version = version_meta.semver;
    let trimmed_rustc_version = Version::new(
        rustc_version.major,
        rustc_version.minor,
        rustc_version.patch,
    );

    println!("cargo:rustc-check-cfg=cfg(rust_1_82)");
    if trimmed_rustc_version >= Version::new(1, 82, 0) {
        println!("cargo:rustc-cfg=rust_1_82");
    }
}
