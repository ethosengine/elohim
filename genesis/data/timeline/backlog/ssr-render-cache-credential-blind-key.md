---
id: "backlog-ssr-render-cache-credential-blind-key"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway SSR: the render cache key is blind to the user credential, so a credentialed render is served to other principals"
slug: "ssr-render-cache-credential-blind-key"
written: "2026-07-30"
author: "rust-architect"
status: "wip"
priority: "high"
relatedNodeIds: []
tags: [doorway, ssr, security, cache, trust-boundary, reach]
shift_objective: |
  Stop doorway's SSR render cache from serving one principal's rendered HTML to
  another. `render_cache_key(&url, &[], "v1")` (http.rs:3596) omits the user
  credential entirely, and http.rs:3726 writes every successful render into it
  with a 5-minute TTL — including renders performed with a per-user credential
  attached (build_ssr_user_credential -> ResolverFetcher). Pick one:
  (a) do not cache when the render's fetcher declares FetcherTrust::Principal
      (simplest, and the trust declaration to gate on already exists), or
  (b) include a credential fingerprint in the cache key (keeps the cache warm
      for repeat visits by the same principal, at the cost of key cardinality).
  (a) is the recommended first move; it is a few lines and removes the exposure
  outright. Add a regression test asserting an anonymous request never receives
  a cache entry produced by a credentialed request for the same URL.
cites:
  - doorway/doorway-service/src/server/http.rs
  - doorway/doorway-service/src/ssr.rs
  - doorway/doorway-service/src/cache/store.rs
  - elohim/elohim-render/src/data_fetcher.rs
---

## What

Doorway's SSR path caches rendered HTML under a key that does not include **who the render
was performed for**, then serves that cache entry to everyone.

- The key: `let cache_key = crate::ssr::render_cache_key(&url, &[], "v1");`
  (`http.rs:3596`). Inputs are the URL, an empty `fetched_hashes` slice, and a spec version.
  **No principal, no credential, no auth posture.**
- The write: `state.cache.put_rendered(&cache_key, html.clone(), Duration::from_secs(5 * 60))`
  (`http.rs:3726`), on every successful non-empty render — unconditionally.
- The read: `state.cache.get_rendered(&cache_key)` (`http.rs:3597`), before any credential is
  even constructed, and returns the cached HTML directly.

## Why it is a disclosure

The render that populates that entry may have been performed under an end-user's credential.
`build_ssr_user_credential(&req)` (`http.rs:998`) lifts the request's `Authorization` header,
or a `Cookie` carrying `doorway_session=` / `steward_attestation=`, into a `UserCredential`;
`ResolverFetcher` attaches it to every outbound storage fetch (`ssr.rs:150-152`). That is
deliberate and correct on its own — it is the reach-aware SSR contract, so an authenticated
visitor's render resolves reach-gated content instead of falling back to public.

The defect is that the *result* of that reach-aware render is then stored under the URL alone.
For the next 5 minutes, any request for the same URL — including an anonymous one, including a
different user's — gets a cache HIT and is served HTML rendered with someone else's reach.

This is materially worse than the sibling
[isolate-reuse residue channel](epr:elohim-render-isolate-reuse-trust-boundary): that one
requires the Angular bundle to actually retain user data at module scope for anything to leak,
whereas this one hands over the rendered document directly. It is also much cheaper to fix.

## Why it survived review

The two halves are correct in isolation and were plausibly written at different times. The
cache predates the reach-aware credential threading; its comment still describes the MVP
intent (*"MVP cache key: (url, spec_version). TTL invalidation (5-minute default).
`fetched_inputs` is captured in the audit trail (RenderOutput) but not in the lookup key."*).
Adding a per-user credential to the fetcher made a previously-safe cache key unsafe without
touching the cache code, so nothing flagged it.

Note the comment's own hint: `fetched_inputs` is *deliberately* excluded from the key. That
choice is fine for content-hash invalidation but it is exactly what makes the key blind to the
one thing that now varies per principal.

## Mitigating factors (why this is high, not critical)

- `render_capability.auth_modes` gates which auth postures reach the render path at all
  (`http.rs:3484-3509`). An operator publishing a claim of `["anonymous"]` never renders a
  credentialed request, so nothing credentialed enters the cache. **But the default is
  `["anonymous", "doorway-hosted"]`** (`render/capability.rs:179-188`, asserted at
  `capability.rs:425-426`), and the gate is skipped entirely when no claim is published
  (`if let Some(claim)`).
- The TTL is short (5 minutes), bounding the window per URL.
- What leaks is reach-gated *content* the peer chose to render, not credentials themselves.

## The fix

The trust declaration needed to gate on already exists as of 2026-07-30:
`DataFetcher::trust_scope() -> FetcherTrust` (`elohim/elohim-render/src/data_fetcher.rs`), and
`ResolverFetcher` returns `FetcherTrust::Principal` exactly when a `UserCredential` is
attached. So the minimal correct fix is a guard at the `put_rendered` site — do not cache a
render whose fetcher declared `Principal` — plus, for symmetry, skipping the cache *read* for
credentialed requests so an authenticated visitor is not served a stale anonymous render of
reach-gated content.

Option (b), fingerprinting the credential into the key, keeps the cache warm for repeat visits
by the same principal and is a reasonable follow-up, but it introduces key cardinality and a
token-hashing surface; option (a) removes the exposure with far less to get wrong.

## Acceptance

1. A credentialed render's HTML is never returned to a request that did not present that same
   credential. Regression test: render URL `/x` with a credential, then request `/x`
   anonymously, and assert the response is not the credentialed HTML (cache MISS or fallback).
2. An anonymous render's cache entry is not served to a credentialed request for reach-gated
   content (the converse staleness direction).
3. The `render_cache_key` doc comment states the principal-scoping rule explicitly, so the
   next person to extend the key does not re-open this.

---

# FIXED (2026-07-30, rust-architect) — option (a), skip the cache for Principal renders

Session-architect decision: **do not key the cache by credential.** Caching reach-gated HTML at
all is a liability, and per-principal keys would multiply cardinality for a 5-minute TTL win.
`Principal` renders skip the cache **in both directions**.

## The gating rule lives in the shared render layer, not in doorway

`FetcherTrust::is_cache_shareable()` — `elohim/elohim-render/src/data_fetcher.rs`. It returns
true only for `FetcherTrust::Ambient`.

It is deliberately NOT a doorway function. Rendering is a capability of the peer runtime
(`elohim-storage`'s `ssr` feature); **doorway is one optional web2 projection of it, never a
required component of the render path.** Every render host must make the same cache decision,
so the decision sits beside the trust vocabulary in the shared layer and each host's cache
adapter only calls it. A host re-deriving the rule locally is how two hosts diverge.

Doorway keeps only the thin cache *adapter*, because `ContentCache` is doorway-local:
`cached_render_for_trust` / `cache_render_for_trust` in `doorway/doorway-service/src/ssr.rs`.
These move when render serving consolidates (see below); the predicate does not.

## Wiring

In `doorway/doorway-service/src/server/http.rs::serve_ssr_route`, the per-request
`ResolverFetcher` is now constructed **before** the cache lookup (it previously came after), so
`render_trust = fetcher.trust_scope()` is available to gate both directions:

- **Read** — `crate::ssr::cached_render_for_trust(&state.cache, &cache_key, render_trust)`
  replaces the bare `state.cache.get_rendered(&cache_key)`.
- **Write** — `crate::ssr::cache_render_for_trust(..., render_trust)` replaces the bare
  `state.cache.put_rendered(...)`.

`maybe_inject_stall_fault` is still applied *after* the cache lookup, so its test-only warning
still marks an actual render rather than a cache hit; the decorator delegates `trust_scope`, so
the hoisted `render_trust` still describes the final fetcher.

## elohim-storage's SSR path: nothing to gate yet

Checked `elohim/elohim-storage/src/ssr.rs` for a symmetric cache — **there is none** (zero
occurrences of `cache` in the file; `render_url` renders unconditionally). No fix needed there
today. **When storage-ssr grows a render cache it MUST call the same
`FetcherTrust::is_cache_shareable()` predicate** rather than re-deriving the rule. Its
`LocalFetcher` is `Ambient` (the peer renders its own content under its own authority), so a
storage-side cache would be shareable today — but that changes the instant a requesting user's
identity is threaded through, which is exactly why the predicate is shared.

## Evidence

`cargo test --lib --bins` (doorway) — **exit 0**, 869 passed / 0 failed / 2 ignored, including:

| Test | Asserts |
|---|---|
| `ssr::tests::ambient_render_round_trips_through_the_cache` | the anonymous path still caches and serves — the SSR cache is preserved where it is safe |
| `ssr::tests::principal_render_is_never_written_to_the_cache` | a credentialed render is absent from the store, checked on the anonymous read path AND directly via `get_rendered` |
| `ssr::tests::principal_render_is_never_served_from_the_cache` | a credentialed request does not inherit a legitimately-cached anonymous entry |
| `ssr::tests::only_ambient_is_cache_shareable` | pins doorway's adapter to the shared predicate |

## Related decision landed in the same pass

`DEFAULT_AUTH_MODES` flipped `["anonymous", "doorway-hosted"]` → `["anonymous"]`
(`doorway/doorway-service/src/render/capability.rs`). Authenticated requests no longer reach the
reused V8 isolate unless an operator opts in. This shrinks the *population* of `Principal`
renders to zero by default, which is defence in depth on top of the cache gate — not a
substitute for it, since an opted-in operator still needs the cache to be trust-scoped.

## Consolidation note (sequenced work, not this fix)

Render-path logic that exists only in doorway — this cache, capability derivation, the auth-mode
gate — is a **misplacement to be migrated, not extended**. The consolidation of render serving
out of doorway into the shared render-host layer is sequenced per
`genesis/docs/superpowers/specs/2026-07-30-render-delivery-manifest-adapter-design.md` §3e/§6:
doorway and storage-ssr become symmetric thin hosts over one engine and one contract. The
operational test is **a zero-doorway mesh with full SSR delivery**. This fix was deliberately
kept minimal and landed where the cache lives today, but shaped to move: the decision is already
in the shared layer, so migration relocates the adapter only.

**Owner:** rust-architect.
