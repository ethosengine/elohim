//! Names the target triple this crate is compiled for (`EPR_CLI_TARGET`), so the sibling draw
//! reads its C baseline by the real triple. A triple put together from `cfg` predicates is not
//! the target: GNU x32 satisfies x86_64, linux and gnu alike (row 29.2 review, blocker 3).
fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    let target = std::env::var("TARGET").expect("cargo sets TARGET for every build script");
    println!("cargo:rustc-env=EPR_CLI_TARGET={target}");
}
