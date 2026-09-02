// hc-rna's `[lib]` is `crate-type = ["cdylib", "rlib"]` (2025-12) because the
// content_store / content_store_integrity zomes consume it as a path
// dependency for its rlib, but cargo compiles both crate-types of a `[lib]`
// target in one rustc invocation — so the cdylib's link step gates the rlib
// the zomes actually need.
//
// hc-rna links against `hdk` (host-call capable, unlike a pure `hdi` helper),
// so its wasm32 cdylib pulls in the same `__hc__*` host-import externs
// (`holochain_wasmer_guest::host_externs!`, plain `extern "C"`, no body) the
// zome cdylibs do. Newer rust-lld (rustc 1.98, 2026-08) refuses undefined
// symbols in a wasm shared object unless told they are imports; older
// toolchains (the holonix pin CI builds under) imported them by default.
// `--import-undefined` is the documented wasm-ld switch for exactly this and
// is a no-op where the default already imports — the same fix applied to every
// zome crate's build.rs; mirrored here so hc-rna's cdylib link doesn't fail
// before either zome gets a chance to compile.
//
// The crate-type is deliberately NOT narrowed to rlib-only: the integrity
// zomes link hc-rna's rlib, and a crate-type change is not provably
// byte-neutral for the integrity wasm CI builds under holonix — a DNA-hash
// move on the fleet. A link flag that is a no-op there is.
//
// Guarded to wasm32 so a native `cargo test`/`cargo build` of hc-rna's CLI
// binaries (the `cli` feature, native target) never hands the flag to a
// non-wasm linker.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=--import-undefined");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
