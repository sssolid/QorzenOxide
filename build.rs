// build.rs
fn main() {
    println!(
        "cargo:rustc-env=RUSTC_VERSION={}",
        rustc_version::version().unwrap()
    );
    println!(
        "cargo:rustc-env=TARGET={}",
        std::env::var("TARGET").unwrap()
    );
}
