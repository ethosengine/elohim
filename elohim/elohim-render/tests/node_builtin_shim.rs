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
        .expect_err("randomBytes stub must throw when called");
    let msg = format!("{err}");
    assert!(msg.contains("randomBytes"), "error names the member: {msg}");
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
        msg.contains("readline"),
        "error names the unshimmed builtin: {msg}"
    );
    assert!(
        !msg.contains("not prefixed with"),
        "must be the loud shim error, NOT the raw relative-import TypeError: {msg}"
    );
}

/// The link-safety keystone. Every synthetic builtin shim must carry the exact
/// named exports the bundle links against, so a NAMED import
/// (`import { createRequire } from "node:module"`) LINKS rather than dying at
/// link time with "does not provide an export named 'createRequire'". The
/// fixture named-imports the full surface across ~10 builtins; reaching its
/// `render()` at all proves linking, and the returned string proves each
/// member's behaviour (real vs loud).
#[tokio::test]
async fn named_imports_across_builtins_link_and_behave() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .render_via_module(&fixture("node-builtins-link.mjs"), "/x")
        .await
        .expect("the named-import surface must LINK (reaching render proves it)");

    // module.createRequire is real and returns a require function...
    assert!(
        out.contains("createRequire-fn:true"),
        "createRequire must return a function: {out}"
    );
    assert!(
        out.contains("createRequireBare-fn:true"),
        "bare `module` specifier links the same as `node:module`: {out}"
    );
    // ...whose require is loud for NON-builtin ids (npm addons ws try/catches).
    assert!(
        out.contains("require-loud:true"),
        "require of a non-builtin must throw a named error: {out}"
    );
    // ...but RESOLVES Node builtins to CJS exports objects (the ws interop rung).
    assert!(
        out.contains("require-events-ctor:true"),
        "require('events') must return the constructible EventEmitter: {out}"
    );
    assert!(
        out.contains("require-events-self:true"),
        "require('events').EventEmitter must self-reference: {out}"
    );
    assert!(
        out.contains("require-events-extends:true"),
        "`class X extends require('events')` must be a legal definition: {out}"
    );
    assert!(
        out.contains("require-node-prefix:true"),
        "require('node:events') must resolve the same builtin: {out}"
    );
    assert!(
        out.contains("require-net-shape:true"),
        "require('net') must be a namespace object carrying its members: {out}"
    );
    // require('crypto') is the SAME object as the ESM crypto default (single
    // source across the ESM and CJS surfaces), with the real createHash.
    assert!(
        out.contains("require-crypto-same:true"),
        "require('crypto') must be the same object as the ESM crypto default: {out}"
    );
    assert!(
        out.contains("require-crypto-createHash:true"),
        "require('crypto').createHash must be the real impl: {out}"
    );
    // util.promisify and events.setMaxListeners are real.
    assert!(
        out.contains("promisify-fn:true"),
        "promisify must return a function: {out}"
    );
    assert!(
        out.contains("setMaxListeners-noop:true"),
        "setMaxListeners must be a real no-op returning undefined: {out}"
    );
    // A loud stub throws a NAMED error (names the builtin + the member).
    assert!(
        out.contains("createServer-loud:true"),
        "net.createServer must throw a loud, named error when called: {out}"
    );
    // The whole named surface links (all bindings defined).
    assert!(
        out.contains("all-present:true"),
        "every named import must resolve to a defined binding: {out}"
    );
}

/// The `node:module` createRequire path in isolation — the exact link failure
/// the deployed bundle hit at `polyfills.server.mjs:1` (`import { createRequire }
/// from 'node:module'`). This is the minimal reproduction: it must LINK, the
/// returned require must resolve Node builtins to CJS exports (the `ws` interop
/// rung), and stay a loud, named throw for non-builtin ids.
#[tokio::test]
async fn node_module_createrequire_links_and_resolves_cjs_builtins() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .render_via_module(&fixture("node-builtins-link.mjs"), "/x")
        .await
        .expect("node:module createRequire named import must link");
    assert!(out.contains("createRequire-fn:true"), "{out}");
    assert!(out.contains("require-loud:true"), "{out}");
    assert!(out.contains("require-events-ctor:true"), "{out}");
    assert!(out.contains("require-net-shape:true"), "{out}");
}

/// Local-only harness: render the REAL deployed Angular server bundle end-to-end
/// through the production `AngularRenderer` path, to prove no Node-builtin link
/// or call failure remains on the boot path. Ignored by default — it needs the
/// deployed bundle unpacked on disk, which is NOT in the repo.
///
/// # Fetch recipe (read-only against the deployed alpha)
///
/// ```sh
/// mkdir -p /tmp/claude-ssr-bundle && cd /tmp/claude-ssr-bundle
/// # 1. resolve the server bundle blob hash from the landing content doc:
/// HASH=$(curl -s https://doorway-alpha.elohim.host/db/content/elohim-host-landing \
///   | python3 -c 'import sys,json;print(json.load(sys.stdin)["serverBlobHash"])')
/// # 2. download + unzip the ~4.1MB server bundle (main.server.mjs + chunks):
/// curl -s "https://doorway-alpha.elohim.host/blob/$HASH" -o server-bundle.zip
/// mkdir -p bundle && cd bundle && unzip -oq ../server-bundle.zip
/// ```
///
/// Then run:
/// ```sh
/// cd elohim/elohim-render && RUSTFLAGS="" CARGO_TARGET_DIR=/tmp/claude-render-target \
///   cargo test --test node_builtin_shim renders_deployed_bundle -- --ignored --nocapture
/// ```
///
/// Override the bundle location with `ELOHIM_RENDER_DEPLOYED_BUNDLE=/path/to/main.server.mjs`.
#[tokio::test]
#[ignore = "requires the deployed SSR bundle unpacked on disk (see fetch recipe in the doc comment)"]
async fn renders_deployed_bundle_without_node_builtin_wall() {
    use async_trait::async_trait;
    use elohim_render::{
        AngularRenderer, DataFetcher, FetchRequest, FetchResponse, RenderContext, RenderLimits,
        RenderSpec, Renderer,
    };
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::Arc;

    // A stub fetcher: every data fetch returns an empty-ish JSON object. A
    // truthful-empty render still proves module linking + bootstrap.
    struct StubFetcher;
    #[async_trait]
    impl DataFetcher for StubFetcher {
        async fn fetch(&self, _req: FetchRequest) -> elohim_render::Result<FetchResponse> {
            Ok(FetchResponse {
                status: 200,
                headers: HashMap::new(),
                body: b"{}".to_vec(),
                content_hash: None,
            })
        }
    }

    let bundle: PathBuf = std::env::var("ELOHIM_RENDER_DEPLOYED_BUNDLE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/claude-ssr-bundle/bundle/main.server.mjs"));

    if !bundle.exists() {
        eprintln!(
            "deployed bundle not found at {} — skipping (see fetch recipe in the doc comment)",
            bundle.display()
        );
        return;
    }

    let renderer = AngularRenderer::new(bundle, Arc::new(StubFetcher))
        .expect("renderer init on deployed bundle");
    let ctx = RenderContext {
        spec: RenderSpec::AngularSsr,
        url: "/".into(),
        data_fetcher: Arc::new(StubFetcher),
        limits: RenderLimits {
            wall_time_ms: 120_000,
            memory_mb: 1024,
            max_fetches: 256,
            max_output_bytes: 8 * 1024 * 1024,
        },
    };

    match renderer.render(ctx).await {
        Ok(out) => {
            eprintln!(
                "deployed bundle rendered: status={} html_len={}",
                out.status,
                out.html.len()
            );
            eprintln!("html head: {}", &out.html[..out.html.len().min(600)]);
            // Opt-in full-HTML dump for inspecting the first real render (inert by
            // default). `ELOHIM_RENDER_DUMP_HTML=/path cargo test ... -- --ignored`.
            if let Ok(path) = std::env::var("ELOHIM_RENDER_DUMP_HTML") {
                std::fs::write(&path, &out.html).expect("dump html");
                eprintln!("wrote rendered HTML to {path}");
            }
        }
        Err(e) => {
            let msg = format!("{e}");
            // The success bar for this harness: whatever wall we hit, it must NOT
            // be a Node-builtin link or "not implemented" failure. A data-shape or
            // DOM-global wall AFTER linking is out of scope for the builtin shim.
            assert!(
                !msg.contains("does not provide an export named"),
                "a Node-builtin NAMED import still fails to link: {msg}"
            );
            assert!(
                !msg.contains("shim:") || !msg.contains("is not implemented"),
                "a Node-builtin shim member was called on the boot path but is a loud \
                 stub — implement it for real: {msg}"
            );
            assert!(
                !msg.contains("not prefixed with"),
                "a bare builtin still reaches FsModuleLoader (resolve gap): {msg}"
            );
            eprintln!("render hit a NON-builtin wall (acceptable for this harness): {msg}");
        }
    }
}
