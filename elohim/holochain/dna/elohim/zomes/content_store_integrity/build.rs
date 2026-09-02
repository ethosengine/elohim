// Zome cdylibs import their host functions (`__hc__*`, declared as plain
// `extern "C"` by holochain_wasmer_guest::host_externs!) from the conductor at
// instantiation. Newer rust-lld (rustc 1.98, 2026-08) refuses undefined
// symbols in a wasm shared object unless told they are imports; older
// toolchains (the holonix pin CI builds under) imported them by default. The
// flag is the documented wasm-ld switch for exactly this and is a no-op where
// the default already imports. Guarded to wasm so a native `cargo test` of the
// workspace never hands the flag to a non-wasm linker. A `.cargo/config.toml`
// rustflags entry would NOT work here: the dev container exports RUSTFLAGS
// (getrandom backend), and an env RUSTFLAGS overrides every config rustflags.
fn main() {
    if std::env::var("CARGO_CFG_TARGET_ARCH").as_deref() == Ok("wasm32") {
        println!("cargo:rustc-link-arg=--import-undefined");
    }
    println!("cargo:rerun-if-changed=build.rs");
}
