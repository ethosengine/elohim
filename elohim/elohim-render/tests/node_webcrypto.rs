//! Render-level golden tests for the WebCrypto global shim (`globalThis.crypto`)
//! installed at isolate init by `node_webcrypto.js` + backed by the
//! `op_elohim_get_random_values` op. These load real ESM fixtures through the same
//! `NodeShimLoader` + `node_crypto_ext` the production `with_shims` /
//! `with_full_shims` runtimes use, so `getRandomValues` / `randomUUID` and the
//! one-source-of-truth contract are exercised end-to-end in a V8 isolate.
//!
//! The Rust-level entropy properties (non-constant bytes, length, quota) are unit
//! tested in `src/shim/node_crypto.rs`; these tests prove the JS surface and the
//! op wiring on top of that core.

use elohim_render::runtime::JsRuntime;

fn fixture(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(name)
}

/// `globalThis.crypto.getRandomValues` fills in place (returning the same array),
/// preserves length, yields non-constant / non-zero bytes across draws and across
/// element widths, no-ops on a zero-length array, and `randomUUID()` is a
/// well-formed, unique v4 UUID.
#[tokio::test]
async fn getrandomvalues_and_randomuuid_behave() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .render_via_module(&fixture("webcrypto-consumer.mjs"), "/x")
        .await
        .expect("webcrypto-consumer renders");

    for flag in [
        "inPlaceSameRef:true",
        "lenPreserved:true",
        "differ:true",
        "notAllZero:true",
        "u32NonZero:true",
        "emptyOk:true",
        "uuidFormatOk:true",
        "uuidUnique:true",
    ] {
        assert!(
            out.contains(flag),
            "expected `{flag}` in render output: {out}"
        );
    }
}

/// One source of truth: `import { webcrypto } from "node:crypto"` and the default
/// export's `.webcrypto` are the SAME object as `globalThis.crypto`.
#[tokio::test]
async fn node_crypto_webcrypto_is_the_same_object_as_global_crypto() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .render_via_module(&fixture("webcrypto-source-of-truth.mjs"), "/x")
        .await
        .expect("webcrypto-source-of-truth renders");
    assert!(
        out.contains("namedSame:true"),
        "node:crypto `webcrypto` export must === globalThis.crypto: {out}"
    );
    assert!(
        out.contains("defaultSame:true"),
        "the crypto default export's .webcrypto must === globalThis.crypto: {out}"
    );
    assert!(
        out.contains("uuidViaModule:true"),
        "the real randomUUID must flow through the module surface too: {out}"
    );
}

/// `crypto.subtle.digest` is a loud, named stub: calling it on the render path
/// throws an error naming the operation, not a silent wrong result.
#[tokio::test]
async fn subtle_digest_is_a_loud_named_stub() {
    let mut rt = JsRuntime::with_shims();
    let err = rt
        .render_via_module(&fixture("webcrypto-subtle-stub.mjs"), "/x")
        .await
        .expect_err("subtle.digest stub must throw when called");
    let msg = format!("{err}");
    assert!(
        msg.contains("subtle.digest"),
        "error names the subtle operation: {msg}"
    );
    assert!(
        msg.contains("not implemented"),
        "error is a clear not-implemented message: {msg}"
    );
}
