//! Render-level golden tests for the Node-builtin shim (crypto) + the loud
//! error path for unshimmed builtins. These load real ESM fixtures through the
//! same `NodeShimLoader` + `node_crypto_ext` the production `with_shims` /
//! `with_full_shims` runtimes use, so they exercise resolution AND evaluation
//! end-to-end in a V8 isolate.

use elohim_render::runtime::JsRuntime;

/// sha256("hello") -- the vector produced by the injected crypto shim.
const SHA256_HELLO: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

#[tokio::test]
async fn crypto_createhash_renders_via_default_and_named_imports() {
    let mut rt = JsRuntime::with_shims();
    let html = rt
        .render_via_module(&fixture("crypto-consumer.mjs"), "/x")
        .await
        .expect("crypto-consumer renders");
    // Both the default import (`crypto.createHash`) and the `node:crypto` named
    // import (`createHash`) resolve to the shim and produce the real digest.
    assert_eq!(
        html.matches(SHA256_HELLO).count(),
        2,
        "default + node:crypto named import both produced the real sha256 digest: {html}"
    );
}

#[tokio::test]
async fn unimplemented_crypto_member_throws_named_error() {
    let mut rt = JsRuntime::with_shims();
    let err = rt
        .render_via_module(&fixture("crypto-unimplemented.mjs"), "/x")
        .await
        .expect_err("randomUUID stub must throw when called");
    let msg = format!("{err}");
    assert!(msg.contains("randomUUID"), "error names the member: {msg}");
    assert!(
        msg.contains("not implemented"),
        "error is a clear not-implemented message: {msg}"
    );
}

#[tokio::test]
async fn unknown_node_builtin_is_loud_named_error_not_panic() {
    let mut rt = JsRuntime::with_shims();
    let err = rt
        .render_via_module(&fixture("node-builtin-unknown.mjs"), "/x")
        .await
        .expect_err("importing an unshimmed builtin must fail");
    let msg = format!("{err}");
    assert!(
        msg.contains("zlib"),
        "error names the unshimmed builtin: {msg}"
    );
    assert!(
        !msg.contains("not prefixed with"),
        "must be the loud shim error, NOT the raw relative-import TypeError: {msg}"
    );
}
