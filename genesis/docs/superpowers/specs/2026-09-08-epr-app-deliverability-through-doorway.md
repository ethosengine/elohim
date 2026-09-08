---
title: "EPR-app deliverability through the doorway — the served shell converges from storage, is judged before it is served, says so in a typed view, and is proven on the household mesh before a push"
id: epr-app-deliverability-through-doorway
status: Draft
class: protocol-canonical
context-tier: disclosed
steward: rust-architect
serves:
  - doorway-failover
  - dataplane-convergence
graduation-trigger: "Draft→Active when D1–D4 below are implemented on sprint/epr-app-shell-convergence, the doorway lib + storage lib gates are green, and the new Act I scenarios pass on the household mesh with a receipt under genesis/a2o/reports/; Active→Canonical when served-shell-boots.feature is green on a fleet build carrying the commit and the doorway-failover habit carries the DELTA"
created: 2026-09-08
domain: D4
topic: [epr-app, doorway, warm-shell, deliverability, serverBlobHash, projection, convergence, pre-push]
boundary: "Governs how a doorway decides WHICH shell to serve for a bundled EPR app and HOW it says so. It does not change how bundles are built, how heads are notarized, or the byte route's reach gate. It retires the cell-local-signal dependence of the doorway's declared app head and the silent stale-shell failure."
cites:
  - "doorway-failover-habit | doorway-failover | path: doorway/doorway-service/.epr-meta/doorway-failover.habit.md"
---

# EPR-app deliverability through the doorway

## Why this exists (measured 2026-09-04 and again 2026-09-08)

Both apex names served `/` with a shell from a previous bundle era while storage held the new
head: the entry script 404'd and every visitor got a blank page. The 2026-09-04 rails bound the
warm shell to the head the doorway's own projection declares — but that projection is refreshed
only by the boot-time bulk pull and by conductor `post_commit` signals, which are cell-local (only
the doorway subscribed to the AUTHORING conductor can ever receive one), and the storage
`content.updated` bridge deliberately clears only the app-file slug index, not the projected
entry. So the declared head never moved, the Behind-class shell re-checked upstream against its
own stale declaration, and the amber marker sat for 15 hours. Separately, `serverBlobHash` is a
deploy-projection field written diesel-direct on the authoring peer only; it never crossed to
adam, so `elohim.host`'s SSR adoption looped on "declared head still absent". Edge Dataplane
Validation ran the served-shell scenario and went UNSTABLE, which the orchestrator reads as
success. Nothing blocked the push; the operator found it by looking at the page.

## Sealed decisions

**D1 — Storage-authoritative bundle heads, reconciled on a tick.** The doorway owns one
`BundleHeadsReconciler` (`render/bundle_heads.rs`): for every configured app slug it reads
`GET {storage}/db/content/{slug}` (a) every `BUNDLE_HEADS_TICK_SECS` (30) and (b) on every
`content.created|updated` event for that slug, and writes `blobHash` + `serverBlobHash`
through to the projected entry (`projected_entries`) and the in-memory slug index. When a head
moves it evicts the warm shell for the slug. The existing SSR adoption pass reads the server
head from this reconciler instead of its own `resolve_declared`. Falsifier: a doorway whose
conductor subscription is dead, or that booted while storage was down, converges within one tick
after storage answers.

**D2 — Deliverability is judged through the doorway before a shell is served, and the verdict
is a typed view.** A shell is classified `AtHead` only when (i) it was fetched by that head
(existing `head_bound`) AND (ii) the head's entry script resolves through this doorway
(`HEAD /apps/{head}/{entry-script}` → 200, or storage's `X-Deliverability: boots` for the head,
memoised per head). An incoherent head is never served as current; the last coherent shell
serves with `x-elohim-bundle: <head12>;behind;<reason>`; if no coherent shell exists at all the
doorway answers **503 with a one-line converging page** and `Retry-After`, never a blank 200.
New view `elohim/sdk/schemas/v1/views/bundle-deliverability-view.schema.json`
(Category C: doorway-local operational, reconstructed per request) served at
`GET /admin/bundles` and `GET /admin/bundles/{slug}` — fields per slug: `slug`,
`storageBrowserHead`, `storageServerHead`, `projectedBrowserHead`, `projectedServerHead`,
`servedBrowserHead`, `servedShellProvenance` (`at-head|behind|none`), `deliverability`
(`boots|broken|unjudged`), `deliverabilityReason`, `lastReconcileAt`, `lastReconcileError`,
`ssrAdopted` (bool), `doorwayId`. Wire header `x-elohim-bundle` gains the same vocabulary. The
app's own `/version.json` build stamp must equal the served head's stamp (the 2026-09-06 finding);
the view reports `versionStamp` and the scenario asserts it.

**D3 — `serverBlobHash` converges peer to peer.** The diesel-direct server-head write in
`content_service::patch_content` emits `content.updated` on the storage event bus and bumps the
content sync document exactly as the browser-head path does. Falsifier: after one PATCH on peer A,
every peer's `/db/content/{slug}` carries the same `serverBlobHash` within one sync round.

**D4 — Proven before push, and blocking.** New Act I feature
`genesis/a2o/features/dataplane/epr-app-deliverability.feature` under
`@concern:doorway-failover @requires:multi-node`, stations: (1) bundle N+1 byte-seeded to all
peers, head PATCHed via doorway A only → within 2 ticks BOTH doorways serve a shell whose every
named script/stylesheet resolves through that doorway and whose `version.json` stamp matches the
declared head; (2) `serverBlobHash` equal on all three peers; (3) doorway B restarted while its
storage is down → after storage returns, B converges within 2 ticks with no operator action;
(4) a deliberately broken bundle (entry script missing) is refused: `/admin/bundles/{slug}`
reports `broken`, the served shell stays at the previous coherent head with `behind`, the page is
never blank. Pre-push: the T2-receipt leg becomes `strict` for `doorway-service/src/render/**`,
`doorway-service/src/routes/apps.rs`, `doorway-service/src/projection/**`,
`elohim-storage/src/services/content_service.rs` — a change there with no newer household
receipt for `@concern:doorway-failover` refuses the push. Fleet: the edge Dataplane Validation
served-shell probe becomes a hard FAILURE of the edge build (not UNSTABLE), and
`scripts/ci/verify-projected-head.sh` asserts BOTH doorways' `/admin/bundles/{slug}` read
`boots` + `at-head` after the app deploy.

## Not in scope
Bundle build content (`version.json` presence is the app job's gate, filed 2026-09-06); the
byte-route reach gate; SSR renderer materialisation beyond consuming D1's server head.
