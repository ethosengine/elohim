//! Isolate-reuse trust boundary: the contract tests for `JsRuntime`'s
//! cross-render state channel.
//!
//! `elohim_render::JsRuntime` reuses one V8 isolate across sequential renders
//! and swaps only the `DataFetcher` between them. These tests exist to make
//! that fact *load-bearing* rather than assumed:
//!
//! 1. [`js_global_state_survives_a_fetcher_swap`] asserts the bleed channel is
//!    REAL — state written under one fetcher is readable under the next. It is
//!    written deliberately as a positive assertion of the leak, not as a
//!    hypothesis, because the leak is the current behaviour and pretending
//!    otherwise is how the trust assumption rots. **The day realm-per-render,
//!    per-principal isolates, or snapshot-backed recycling lands, this test
//!    MUST flip to asserting isolation** — and flipping it is the signal that
//!    the trust contract in `JsRuntime`'s and `shim/mod.rs`'s docs has changed.
//! 2. The `trust_scope` tests pin the declaration mechanism itself: a
//!    credentialed fetcher marks the isolate, decorators delegate instead of
//!    laundering, and the mark is sticky.
//!
//! Background and the deno_core-realm verdict:
//! `genesis/data/timeline/backlog/elohim-render-isolate-reuse-trust-boundary.md`.

use async_trait::async_trait;
use elohim_render::runtime::JsRuntime;
use elohim_render::{
    DataFetcher, FetchRequest, FetchResponse, FetcherTrust, Result, TracingFetcher,
};
use std::collections::HashMap;
use std::sync::Arc;

/// A fetcher whose declared trust scope is a constructor parameter, so a test
/// can stage an ambient render followed by a credentialed one (and back).
struct ScopedFetcher(FetcherTrust);

#[async_trait]
impl DataFetcher for ScopedFetcher {
    fn trust_scope(&self) -> FetcherTrust {
        self.0
    }

    async fn fetch(&self, _request: FetchRequest) -> Result<FetchResponse> {
        Ok(FetchResponse {
            status: 200,
            headers: HashMap::new(),
            body: b"{}".to_vec(),
            content_hash: None,
        })
    }
}

fn ambient() -> Arc<dyn DataFetcher> {
    Arc::new(ScopedFetcher(FetcherTrust::Ambient))
}

fn principal() -> Arc<dyn DataFetcher> {
    Arc::new(ScopedFetcher(FetcherTrust::Principal))
}

/// THE test this backlog item exists for.
///
/// Swapping the fetcher between renders changes what the next render is
/// *allowed* to fetch. It does not change what the previous render *left
/// behind*. This asserts the second half of that sentence concretely: a value
/// written to `globalThis` while fetcher A is installed is still readable after
/// fetcher B replaces it.
///
/// `globalThis` here stands in for the much larger real surface — the Angular
/// server bundle's module-scope caches and DI singletons — which is not
/// exercisable without a bundle fixture but leaks by exactly this mechanism.
#[tokio::test]
async fn js_global_state_survives_a_fetcher_swap() {
    let mut rt = JsRuntime::with_full_shims(ambient());

    // "Render 1", under the first fetcher: stash something on the global object.
    rt.set_fetcher(ambient());
    rt.eval_string("globalThis.__renderOneSecret = 'principal-a-data'; 'ok'")
        .await
        .expect("eval under fetcher A");

    // "Render 2", under a DIFFERENT fetcher — a different principal in the
    // shape doorway already ships (build_ssr_user_credential -> ResolverFetcher).
    rt.set_fetcher(principal());
    let observed = rt
        .eval_string("String(globalThis.__renderOneSecret)")
        .await
        .expect("eval under fetcher B");

    assert_eq!(
        observed, "principal-a-data",
        "EXPECTED FAILURE MODE, pinned deliberately: the fetcher swap does not \
         reset the isolate, so render 1's global state is live in render 2. If \
         this assertion has started failing, isolation has been implemented — \
         flip this test to assert `undefined` and update the trust contract in \
         src/runtime.rs, src/shim/mod.rs, and the backlog record."
    );
}

/// A freshly booted isolate has hosted nothing, so it starts uncontaminated.
#[tokio::test]
async fn a_fresh_isolate_is_not_marked_contaminated() {
    let rt = JsRuntime::with_full_shims(ambient());
    assert!(!rt.isolate_hosted_principal_fetcher());
}

/// Ambient renders never mark the isolate — the common doorway case (an
/// anonymous visitor) must not raise a false trust-boundary signal.
#[tokio::test]
async fn ambient_fetchers_do_not_mark_the_isolate() {
    let mut rt = JsRuntime::with_full_shims(ambient());
    rt.set_fetcher(ambient());
    rt.set_fetcher(ambient());
    assert!(
        !rt.isolate_hosted_principal_fetcher(),
        "a run of anonymous renders must not be reported as a trust crossing"
    );
}

/// A credentialed fetcher marks the isolate, and the mark is STICKY: an ambient
/// render that follows a principal render is still running on that principal's
/// residue, so the isolate must not "clean itself" by serving an anonymous
/// request.
#[tokio::test]
async fn a_principal_fetcher_marks_the_isolate_permanently() {
    let mut rt = JsRuntime::with_full_shims(ambient());

    rt.set_fetcher(principal());
    assert!(
        rt.isolate_hosted_principal_fetcher(),
        "a credentialed fetcher must mark the isolate"
    );

    rt.set_fetcher(ambient());
    assert!(
        rt.isolate_hosted_principal_fetcher(),
        "the mark must be sticky — nothing in JsRuntime resets the JS heap, so a \
         subsequent anonymous render still sits on the prior principal's residue"
    );
}

/// `TracingFetcher` wraps EVERY fetcher on the live render path
/// (`AngularRenderer` re-wraps per render). If it answered `trust_scope` for
/// itself instead of delegating, every credentialed fetcher in the system would
/// be laundered into an ambient one and the isolate bookkeeping would be
/// permanently blind. This is the regression guard for that.
#[tokio::test]
async fn the_tracing_decorator_delegates_trust_instead_of_laundering_it() {
    let traced_principal = TracingFetcher::new(principal());
    assert_eq!(
        traced_principal.trust_scope(),
        FetcherTrust::Principal,
        "TracingFetcher must report its INNER fetcher's trust scope"
    );

    let traced_ambient = TracingFetcher::new(ambient());
    assert_eq!(traced_ambient.trust_scope(), FetcherTrust::Ambient);
}

/// The wrapped-fetcher path end to end: a decorated credentialed fetcher handed
/// to `set_fetcher` must still mark the isolate. This is the exact shape the
/// render worker uses (`TracingFetcher::with_soft_budget(ctx.data_fetcher, _)`).
#[tokio::test]
async fn a_decorated_principal_fetcher_still_marks_the_isolate() {
    let mut rt = JsRuntime::with_full_shims(ambient());
    rt.set_fetcher(Arc::new(TracingFetcher::new(principal())));
    assert!(
        rt.isolate_hosted_principal_fetcher(),
        "the render worker always wraps in TracingFetcher; wrapping must not \
         hide the credential from the isolate's trust bookkeeping"
    );
}
