---
id: "backlog-elohim-render-isolate-reuse-trust-boundary"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-render: V8 isolate reuse across sequential renders is a cross-request bleed channel"
slug: "elohim-render-isolate-reuse-trust-boundary"
written: "2026-07-30"
author: "angular22-campaign"
status: "wip"
priority: "medium"
relatedNodeIds: []
tags: [elohim-render, ssr, security, isolate, trust-boundary, deno_core]
shift_objective: |
  Size the deno_core startup-snapshot spike (JsRuntimeForSnapshot +
  create_snapshot at 0.339) — the ONLY remaining path to closing the V8 isolate
  residue channel, since deno_core 0.339 has no public realm API and a fresh
  isolate per render is a ~51MB bundle reparse on a capacity-1 worker. The spike
  must answer: can the Angular server-bundle module graph be snapshotted when it
  is dynamically import()ed from a file:// URL rather than declared in the
  extension set; can our ops (FetcherHandle et al.) register the external
  references V8 snapshot serialization requires; and is Angular's post-bootstrap
  state provably render-neutral at snapshot time.
  The two cheaper mitigations are DONE (2026-07-30): the render cache is
  trust-scoped, and auth_modes defaults to ["anonymous"]. Verdict, evidence, and
  the landed trust-declaration mechanism are in the body.
cites:
  - elohim/elohim-render/src/runtime.rs
  - elohim/elohim-render/src/data_fetcher.rs
  - elohim/elohim-render/src/traced_fetcher.rs
  - elohim/elohim-render/src/shim/mod.rs
  - elohim/elohim-render/src/shim/web_api.js
  - elohim/elohim-render/tests/isolate_trust.rs
  - doorway/doorway-service/src/ssr.rs
  - doorway/doorway-service/src/server/http.rs
---

## What

`elohim_render::JsRuntime` reuses the same `deno_core` V8 isolate across sequential renders,
swapping only the `DataFetcher` per request. The JS-side globals (`fetch`, `Headers`,
`Response`, module-scope state a bundle may stash on `globalThis`, any per-app singleton the
Angular bundle constructs at bootstrap) persist across that swap unless something explicitly
clears them.

## Why this is a trust-boundary concern, not just a perf detail

If a future caller ever runs isolate reuse across renders for **different users** (a
per-user/per-tenant credentialed fetcher swapped in per request — the natural next step once
the render path carries auth context), the isolate becomes a channel for cross-request bleed:

- **JS global state** written by user A's render (module-level caches, memoized computed
  values, anything the Angular bundle or a `@defer` block stashes outside component state) is
  still live in `globalThis` when user B's render runs in the same isolate.
- **The web_api.js shims are install-guarded** (`if (typeof globalThis[name] === "undefined")`
  — see the file's `--- Install ---` section), which prevents re-installation from clobbering
  live state across renders, but that guard is exactly the mechanism that lets stale state
  survive: nothing resets `Headers`/`Response`/`ReadableStream` instances or app-bundle module
  state between requests.
- **The app bundle itself** is the larger surface: Angular DI singletons, RxJS `BehaviorSubject`
  state held in a provided-in-root service, anything memoized at module scope in the built
  `main.server.mjs` — none of that is isolate-reset between calls to `renderApplication()`.

## Pre-existing, not introduced by the v22 campaign

This is architecture that predates the Angular 22 migration; the v22 work touched web_api.js's
install guards but did not create the reuse pattern. Filed now because the review pass that
produced the web_api.js hardening (see the sibling Job-3 commit) surfaced it as an adjacent
concern while reading `src/shim/mod.rs`'s isolate lifecycle notes.

---

# VERDICT (2026-07-30, rust-architect)

**Option (2) realm-per-render is not purchasable at our deno_core version. Option (3)
recycling is not a security answer. Option (1) is implemented — rigorously, in code, not
as a comment — and the item is re-scoped to the one thing that would actually close the
channel: making cold start cheap.**

The premise this item was filed under has also been **falsified**: see "Correction" below.
A credentialed per-user fetcher is not a future risk. It shipped.

## 1. Correction: the "mitigating factor" was already false when filed

The original section below ("Every render today uses the SAME trust level: doorway's own
service-to-service fetch, never a per-user credentialed fetcher") does not describe the
live code. In `doorway/doorway-service/src/server/http.rs`, the SSR path does:

- `build_ssr_user_credential(&req)` (`http.rs:998`) lifts the request's `Authorization`
  header, or a `Cookie` containing `doorway_session=` / `steward_attestation=`, into a
  `UserCredential`;
- that credential is attached to the per-request `ResolverFetcher` via
  `maybe_with_user_credential`, and `ResolverFetcher::fetch` sets it as a header on **every**
  outbound storage fetch (`ssr.rs:150-152`);
- that fetcher is handed to the render as `ctx.data_fetcher` and swapped into the **reused**
  isolate by `AngularRenderer`'s worker loop (`angular.rs`, `runtime.set_fetcher(traced)`).

The one real gate is `render_capability.auth_modes` (`http.rs:3484-3509`): a request whose
auth posture is not in the published claim falls back before reaching V8. But
`DEFAULT_AUTH_MODES` is `["anonymous", "doorway-hosted"]` (`render/capability.rs:179-188`,
asserted at `capability.rs:425-426`), so **by default an authenticated request does reach the
V8 render path with a credentialed fetcher**, and the gate is absent entirely when no claim is
published (`if let Some(claim)`).

So the exposure is live-by-default configuration, not hypothetical. Priority raised
`low` → `medium` accordingly. It is not `high` only because the residue channel is indirect
(it requires the Angular server bundle to actually retain per-render user data at module
scope, which is plausible but unproven) — unlike the directly-exploitable sibling finding
below.

## 2. Option (2) realm-per-render: NOT AVAILABLE at deno_core 0.339.0

Verified against the vendored registry source, not from memory:
`/opt/rust/cargo/registry/src/index.crates.io-1949cf8c6b5b557f/deno_core-0.339.0`
(version pinned in `elohim/Cargo.toml:77` and `elohim/Cargo.lock`, checksum
`e849c86341672e56f1a01473be43b04a14404a146f8a6976ee501768fd153df9`).

| Evidence | Finding |
|---|---|
| `grep -rn 'create_realm'` across the whole crate | **One hit**, and it is a *doc comment* at `runtime/jsrealm.rs:118` referring to `[JsRuntime::create_realm]`. The method does not exist. The doc link is a vestige of removed upstream multi-realm support. |
| `runtime/mod.rs:24` | `pub(crate) use jsrealm::JsRealm;` — `JsRealm` is **crate-private**. `grep -n 'JsRealm' lib.rs` returns nothing: it is not exported. |
| `grep -rn 'CreateRealmOptions'` | **Three hits total**: the definition (`runtime/jsruntime.rs:697`) and two re-exports (`runtime/mod.rs:28`, `lib.rs:149`). **No consumer.** It is a publicly-exported dead struct with no constructor path. |
| `runtime/jsruntime.rs` | Everything is hardcoded to `self.inner.main_realm` — `main_realm()` is itself `pub(crate)` (line 1218), and the event-loop tick still carries a stale `// Get the pending state from the main realm, or all realms` comment (line ~1980) from when "all realms" was a real case. |

Reaching a secondary context anyway would mean raw `v8::Context::new` plus hand-wiring
deno_core's op bindings into it — but ops are bound through the main realm's `ContextState`
and `FunctionTemplateData`, both `pub(crate)`. That is a deno_core fork, not a feature we
can adopt. **Option (2) is closed until deno_core restores a public realm API.**

## 3. Fresh `JsRuntime` per render: not bounded

Rejected on measured-in-repo cost, not on principle. Doorway's own comment at
`http.rs:3646-3648` records the operating envelope: *"60s wall: cold-start parses a ~51MB
bundle (171 .mjs files) + walks Angular bootstrap (which fetches); warm renders settle to
tens of ms once deno_core's module loader caches imports + V8 JIT settles."*

The render driver `await import({bundle_lit})`s the bundle every render; on a reused isolate
that is a module-map cache hit. A fresh isolate per render converts **every** render into
that cold start — the wall-time budget is 60s and the warm case is tens of ms. That is a
three-to-four-order-of-magnitude regression on the hot path, into a **capacity-1 sequential
worker** that sheds to CSR on overflow (`RenderError::Busy`). It would not degrade SSR; it
would effectively disable it. Not bounded, not cheap.

## 4. Option (3) recycling cadence: rejected as a security answer

Recycling every N renders / M minutes does not close the channel — it only shortens the
window. Users A and B inside the same window still share `globalThis`. Given a credentialed
fetcher already ships, a mitigation whose guarantee is "fewer users bleed into each other"
is not a fix; it is a smaller number attached to the same defect. It also imports the §3
cold-start cost as a *periodic latency cliff*: every Nth request pays the 51MB reparse on a
capacity-1 worker, which under load converts into an `ssr_render_busy_total` shed storm.
Worse guarantee, new failure mode. Rejected.

## 5. What was implemented instead (option 1, made load-bearing)

A doc comment alone would have been worthless here — the *existing* doc comments are what
went stale and produced this item's false premise. So the trust assumption is now encoded
where it cannot rot silently:

**A required trait method — the compile-time tripwire.**
`DataFetcher::trust_scope() -> FetcherTrust` (`data_fetcher.rs`) has **no default
implementation**, deliberately. `FetcherTrust` is `Ambient` (fetches under the peer's own
service-to-service identity) or `Principal` (fetches under an end-user's credential). Because
the method is required, **a new `DataFetcher` impl cannot compile without consciously
answering "whose authority is this?"** — that compile error is the tripwire this item asked
for. A defaulted `Ambient` would have silently mis-declared precisely the fetcher that
matters, reproducing the exact bug. All 15 impls across `elohim-render`, `doorway-service`,
and `elohim-storage` were swept.

**Isolate-level bookkeeping at the swap seam.**
`JsRuntime::set_fetcher` now reads the incoming trust scope and records it. The rule is
deliberately conservative and **sticky**: once any `Principal` fetcher has rendered on an
isolate, `isolate_hosted_principal_fetcher()` stays true forever, because nothing in the
runtime resets the JS heap — an *ambient* render following a principal render is still
sitting on that principal's residue. The first render that follows a principal render emits
one `WARN` on target `elohim_render::trust` (warn-once per isolate, so an authenticated
doorway logs one line per isolate rather than one per render). The crossing is now visible in
Loki instead of assumed away in a comment.

**Decorator delegation, guarded.**
`TracingFetcher` wraps *every* fetcher on the live render path (`AngularRenderer` re-wraps per
render), and doorway's `StallFaultFetcher` wraps it too. Both delegate `trust_scope` to their
inner fetcher. A decorator that answered for itself would launder every credentialed fetcher
in the system into an ambient one and render the bookkeeping permanently blind — so both
delegations have explicit regression tests.

**Tests that pin reality rather than the wish.**
`elohim/elohim-render/tests/isolate_trust.rs` (6 tests). The load-bearing one is
`js_global_state_survives_a_fetcher_swap`, which asserts the leak is **REAL**: a value written
to `globalThis` under fetcher A is read back verbatim under fetcher B. It is written as a
positive assertion of the leak, with a message instructing the next engineer to **flip it to
assert isolation** when a fix lands. That inverts the original acceptance sketch (item 2)
to match the world as it is — and makes the doc contract and the test move together.
Doorway gets three sibling contract tests pinning credentialed→`Principal`,
anonymous→`Ambient`, and decorator delegation.

**Docs corrected at all three stale sites**: `JsRuntime` (type-level), `set_fetcher`
("this swap authorizes; it does not isolate"), `AngularRenderer::with_soft_budget` (which
had claimed the per-request swap as an unqualified win), and `shim/mod.rs`'s
isolate-lifecycle note (which now states the contract and warns against adding new
`globalThis`-cached request state).

## 6. Residual risk — what this does NOT fix

**The channel is still open.** Nothing above stops the bleed; it makes the bleed declared,
observable, and impossible to re-forget. A credentialed render still leaves residue that the
next render can reach. Accepting that is a deliberate, now-documented trade against SSR being
functional at all.

**The real fix is to make cold start cheap**, which is the only path that makes
per-principal isolates affordable. The candidate is a `deno_core` **startup snapshot**
(`JsRuntimeForSnapshot` + `create_snapshot`, both exported at 0.339) capturing the evaluated
bundle module graph, so a fresh isolate boots warm. It is a genuine spike, not a fix: ESM
snapshotting wants modules in the extension set rather than dynamically `import()`ed from a
`file://` URL; V8 snapshot serialization needs external references registered for our ops
(`FetcherHandle` et al.); and Angular's bootstrap state at snapshot time must be provably
render-neutral. Sizing that spike is the next action on this item, and it should be sized
against the sibling cache finding below, which is cheaper and more urgent.

## 7. Sibling finding surfaced during this work (filed separately)

The SSR **render cache is keyed blind to the credential** — `render_cache_key(&url, &[], "v1")`
(`http.rs:3596`), written at `http.rs:3726` with a 5-minute TTL. A credentialed render's HTML
is stored under the URL alone and served to subsequent different-principal (including
anonymous) requests. That is a *direct* cross-user disclosure of rendered content, strictly
worse and far more concrete than the isolate residue channel, and it is cheap to fix (include
a credential fingerprint in the key, or simply do not cache `Principal`-scoped renders).
Filed as `ssr-render-cache-credential-blind-key.md`. **Do that one first.**

## 8. Decisions landed 2026-07-30 (session architect)

Both §7 and acceptance item 6 below were decided and implemented in the same pass.

**(a) The SSR render cache is now trust-scoped.** `Principal` renders skip the cache in both
directions; the credential is deliberately NOT added to the key. The gating predicate
`FetcherTrust::is_cache_shareable()` lives in `elohim/elohim-render/src/data_fetcher.rs` — the
**shared** render layer, not doorway — because doorway is one optional web2 projection of a peer
runtime capability, never a required component of the render path, and every render host must
make the same decision. Doorway keeps only the `ContentCache`-bound adapter. `elohim-storage`'s
SSR path has no cache today (verified), so nothing to gate symmetrically yet; when it grows one
it calls the same predicate. Full write-up: `ssr-render-cache-credential-blind-key.md`.

**(b) `DEFAULT_AUTH_MODES` flipped to `["anonymous"]`** (`doorway/doorway-service/src/render/
capability.rs`), from `["anonymous", "doorway-hosted"]`. Rationale, recorded at the constant:
until per-principal isolates land (§6's snapshot spike), authenticated SSR in a reused isolate
is **unsafe by construction**. The cost is nil — an unsupported posture already falls back to
CSR with `x-ssr-skipped: auth-mode-not-supported`, a modelled graceful degradation where the app
hydrates and fetches with the user's credential client-side. So this is secure-by-default with
explicit operator opt-in and no user-facing breakage.

The `anonymous`-always-present invariant and the reduce-never-inflate override semantics are
**unchanged**: `override_restricting_auth_modes_keeps_anonymous_required` and
`override_restricting_to_anonymous_only_works` pass untouched. Only the test asserting the
*derived default* moved (`derives_full_profile_when_disk_and_manifest_align`), plus two new a2o
scenarios in `genesis/a2o/features/content/ssr_capability.feature` pinning the secure default and
the explicit opt-in. `tests/capability_publish.rs` builds its profile as a literal fixture, so it
tests round-trip serialization rather than the default — correctly left alone.

Together these shrink the `Principal`-render population to zero by default AND make the cache
safe for operators who opt in — defence in depth, not one substituting for the other. **The
isolate residue channel itself remains open** (§6); these close the two paths by which it was
reachable in a default deployment.

## 9. Consolidation: render serving belongs in the shared host layer

Render-path logic that exists only in doorway — the render cache, capability derivation, the
auth-mode gate — is a **misplacement to be migrated, not extended.** The renderer is an optional
feature of the peer runtime (`elohim-storage`'s `ssr` feature); doorway consumes the identical
engine and contract solely to project rendered HTML to web2.

That consolidation is sequenced work per
`genesis/docs/superpowers/specs/2026-07-30-render-delivery-manifest-adapter-design.md` §3e/§6:
doorway and storage-ssr converge into **symmetric thin hosts** over one engine and one contract.
The operational test is **a zero-doorway mesh with full SSR delivery**.

The §8 fixes were kept minimal and landed where the code lives today, but shaped to move — the
decisions sit in the shared layer, so migration relocates adapters only. Anything added to
doorway's render path before that consolidation should follow the same discipline: decision in
`elohim-render`, host-local binding in the host.

## Acceptance sketch (remaining)

1. ~~Decide (1)/(2)/(3)~~ — done, see verdict.
2. ~~A regression test for cross-render state~~ — done, inverted:
   `tests/isolate_trust.rs::js_global_state_survives_a_fetcher_swap` pins the leak and must be
   flipped to assert isolation when a fix lands.
3. ~~Update `src/shim/mod.rs`'s isolate-lifecycle doc comment~~ — done.
4. ~~Fix the sibling render-cache key finding (§7)~~ — done, §8(a).
5. ~~Default `render_capability.auth_modes` to `["anonymous"]`~~ — done, §8(b).
6. **Open:** size the deno_core startup-snapshot spike (§6). If it lands, per-principal
   isolates become affordable and this item closes properly. This is now the ONLY remaining
   path to closing the residue channel itself.
7. **Open (sequenced elsewhere):** consolidate render serving out of doorway into the shared
   render-host layer (§9) — tracked by the render-delivery manifest-adapter design, not by this
   item. Listed here so the §8 adapters are migrated rather than entrenched.

**Owner:** rust-architect.
