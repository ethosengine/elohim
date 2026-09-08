---
title: "EPR-app deliverability through the doorway — the served shell converges from storage, is judged before it is served, and is proven on the household mesh before a push"
id: epr-app-deliverability-through-doorway
status: Active
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
boundary: "Governs how a doorway decides WHICH shell to serve for a bundled EPR app and HOW it says so. It preserves the existing Content notary and byte-route reach gate, adds a shared local/CI packaging operation, and carries the server identity through the canonical Content snapshot. It retires the cell-local-signal dependence of the doorway's declared app head and the silent stale-shell failure."
cites:
  - "substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
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
`BundleHeadsReconciler` (`render/bundle_heads.rs`; targets = the EPR-router mounts ∪ configured SSR slugs): for every app slug it reads
`GET {storage}/db/content/{slug}` (a) every `BUNDLE_HEADS_TICK_SECS` (30) and (b) on every
`content.created|updated` event for that slug, and writes `blobHash` + `serverBlobHash`
through to the projected entry (`projected_entries`) and the in-memory slug index on every successful read, even when the head is unchanged. This repairs failed writes and late stale projections. Tick and event passes are serialized so an older read cannot finish after a newer write. When a head
moves it evicts the warm shell for the slug. The existing SSR adoption pass reads the server
head from this reconciler instead of its own `resolve_declared`. Falsifier: a doorway whose
conductor subscription is dead, or that booted while storage was down, converges within one tick
after storage answers.

**D2 — Deliverability is judged through the doorway before a shell is served, and the verdict is
readable in the headers the test already sees.** A shell classifies `AtHead` only if (i) it was
fetched by that head (existing `head_bound`) AND (ii) the head is coherent through this doorway:
primary probe `HEAD /apps/{head}/_capability` reading storage's `X-Deliverability`/`-Reason`
(the `/apps/{id}/{file}` route is GET-only, so a HEAD on the entry script cannot be the probe);
fallback `GET /apps/{head}/{entry}` with `Range: bytes=0-0`. Memo keyed by head (the entry is
derived from that head's bytes): a confirmation is PERMANENT — an unreachable peer can never
un-prove a content-addressed head it already proved, which is what keeps a storage blip from
flipping a proven shell to `behind` (load-bearing, not an optimisation); a negative verdict is
re-checked after 30 s; no probe is made at an upstream already judged unavailable. On a head move
the warm shell is evicted and the declaration written through; the archive is NOT purged (its
docs are keyed by head, and the last-reconciled bytes are exactly what the `behind` path must
serve). The bytes in hand serve with `x-elohim-bundle: behind;<reason>` (`missing-asset:<file>`,
`head-unknown`, `storage-unreachable`, `stale-projection`) and `x-elohim-freshness: amber`;
**503** with a one-line converging page and `Retry-After: 20` only when the doorway holds no
shell at all — never a blank 200. `last-reconciled` (coherent, unconfirmed this request) and
`slug-resolved` keep their meaning; the vocabulary is additive. A typed admin view is NOT in this
slice — it is added only if the integration test's failure output proves to need more than the
headers carry. (Implemented 2026-09-08: `render/bundle_heads.rs`, `render/coherence.rs`;
doorway lib 1200/0.)

**D3 — One notarized app declaration carries both bundle identities.** `serverBlobHash`
selects executable code and must not be elected by an unauthenticated sync document. The
existing `Content.metadata_json` carries this app attribute through `content_store::update_content`;
a server-head PATCH therefore uses the conductor just like a browser-head PATCH. Accepted
canonical content projections derive the SQL server pointer from that metadata. Sync carries
hints and bytes, but cannot overwrite a server pointer from an unverified document. A deployment
with an old, unattested server pointer must publish it through this path before peer convergence
can be claimed.

Classification: the server identity is an attribute of the existing notarized Content (A; linked
attribute in concept, encoded in its existing metadata snapshot), and SQL/cache copies are
rebuildable projections (C). This reuses `content_store_integrity::Content` and the existing
`content_store::update_content` output's ActionHash; no new entry type, route, or persistent head
key is introduced, and the DNA hash does not move. Cost is one Content revision per server-bundle
declaration (for N apps at D deployments/year, N×D revisions/year), never one per visitor. The
existing canonical-head resolver chooses the app revision; blob identity remains content-derived,
and byte transfer uses the existing transport. Passive recovery must not promote an old unverified
SQL/server hint into the notarized declaration.

**CI phase diagnostics.** Publication, peer declaration, renderer adoption, and an actual
render are separate observations. `verify-projected-head.sh` first requires this doorway’s
storage route to expose the intended canonical `serverBlobHash` within 90 seconds. Only that
observation admits the 400-second renderer-adoption window. It rechecks the declaration after
adoption and requires an actual SSR response; a healthy registry alone cannot pass. Missing
metadata names the authoring/peer-propagation leg instead of spending the renderer window
waiting for a head the peer has not declared. Changes to `elohim-render` also invalidate the
household receipt, because runtime compatibility is part of delivery. Lamad is checked at its
actual `/lamad/path/elohim-protocol` route and must return the learning path’s rendered h1,
not only serialized state. Its old `/concept` probe path was not an app route. Angular
packaging rejects browser/server base-path drift; finite SDK reads contribute to Angular
SSR stability through the Angular adapter, and async view changes notify its scheduler.

**SSR responsibilities and the two trips.** Peers retain and exchange the app's declared bundle
identities and content-addressed bytes. Doorways fetch, materialize, render, and cache locally;
web visitor traffic must not require rendering once per request on household peers. Rendering
inside a doorway stays in place. Both bundle identities are carried by the existing Content declaration; this slice adds no
new authoritative entity. Agreement on a server pointer, availability of its bytes, adoption
by a doorway, and the HTML a visitor receives are four different assertions.

Act I publishes a valid server fixture once, verifies peer-to-peer pointer and byte delivery,
then checks both doorways' adopted server head and actual rendered HTML through a public mount. It publishes the next browser and server versions back-to-back under the same app without restarting either renderer, checks immutable browser bytes and declaration within 75 seconds, and waits up to 330 seconds (the normal 300-second SSR replacement cadence plus slack). Public SSR HTML and browser bootstrap are checked after adoption; the 75-second immutable browser delivery bound is not a bound on periodic renderer replacement.
After warming that page, it makes the storage peers unavailable and requires the doorway to
continue serving the rendered version locally. Act II reads the deployed declarations,
compares materialized server heads on both doorways, and boots the served browser app. A
browser fallback alone cannot certify SSR. Cache identity must include the rendering inputs
that affect the response (such as route and data/version context); a bundle head alone is not
a claim that arbitrary personalized rendering is a pure function of bundle bytes.

**D4 — The gate is an integration test that boots the app through the doorway, and it blocks.**
`served-shell-boots.feature` keeps its static clause (every script/stylesheet the page names
resolves through the SAME doorway; entry script = declared browser head) and gains the dynamic
clause: a headless browser loads `/` through each doorway and the app BOOTS — no `pageerror`, no
failed same-origin request, the app root visibly rendered with `data-app-ready="true"` set only after browser bootstrap succeeds, and the served `/version.json` stamp equal
to the declared head's stamp. Runs in three places, all blocking: (a) **household mesh, Act I**
(`@act:i` twin scenarios under `@concern:doorway-failover @requires:multi-node`): bundle N+1
published via doorway A only → both doorways boot within 75 seconds in the browser-only station; SSR upgrades use the separate adoption bound above;
`serverBlobHash` equal on all peers; doorway B restarted while its storage is down converges after
storage returns; a deliberately broken bundle (entry script missing) is refused — page never
blank, header says `behind;missing-asset`. Pre-push: the T2-receipt leg is `strict` for
the doorway and storage serving implementation paths, including caches, HTTP dispatch, and sync projection. It requires all current deliverability stations to pass in the household report against matching source identities; unrelated passes, skipped stations, and report timestamps alone are insufficient. (b) **App pipeline, Act II**: immediately after
`authorHeadOnce`, run the feature against BOTH doorways; a red FAILS the app build (`error()`, not
`unstable`). (c) **Edge Dataplane Validation**: the same feature is a hard failure of the edge
build. The rule: an EPR app that cannot boot through a doorway does not ship, and the build that
tried says which doorway, which head, and which asset.

**D5 — Packaging is available before publication through an extensible SDK adapter.**
The shared layer archives and hashes files. The first built-in adapter owns Angular build,
layout and browser/optional-SSR checks. A local `--adapter` module (or `EPR_APP_ADAPTER` through
`just dev package`) can supply build, layout, validation and runtime-check hooks today. Missing
hooks are refused; a custom adapter does not silently inherit Angular assumptions. Runtime
verification is a separate operation against produced artifacts and a compatible execution
host, never implied by successful packaging. A non-browser fixture adapter exercises this
extension point; production native desktop/Wasm adapters remain future work.
 `just dev package
<app-directory>` builds an Angular app and produces browser/server archives using
the same packer called by CI staging. It does not upload or author a head. The SDK API also checks existing builds without rebuilding. Both outputs receive the same version stamp only after a successful build; a missing or mismatched stamp is refused at packaging. The browser
entry, referenced scripts/styles, version stamp, and SSR entry contract fail locally with an
asset or entry named in the error. Packaging leaves source build outputs untouched and hashes
the exact archive that staging uploads. The Act I fixtures use that same staging path.

The local descriptor and archives are reproducible build artifacts (C), not a second release
authority. Published identity remains the existing Content declaration described in D3;
packaging creates no notary entries, routes, or additional heads. Package checks certify file
structure and declared build context. Peer propagation, renderer adoption, warm-cache service
and browser initialization require the separate Act I/Act II evidence; successful compilation
or packaging cannot certify those environmental properties.

## Not in scope
Framework build tools themselves; the byte-route reach gate; a new bundle entity/type;
or replacement of doorway-resident rendering. The first packaging adapter covers the current
Angular EPR-app runtime. Other clients need a compatible packaging and launch adapter, not an Angular renderer.
SSR is optional web behavior, never an EPR-app-wide requirement. Native desktop and non-browser
Wasm clients may retrieve artifacts through peers and validate their own host capabilities and
readiness signals; those adapters are future work. Browser bootstrap and SSR assertions in D4
apply to the web adapter covered here.

## Delivery evidence — 2026-09-08

The spec remains **Active** while fleet proof is outstanding. Household run
`20260908T192233Z-0e89181b` passed the root-address browser station (1 scenario,
23 steps) and restored both borrowed root projections. The station now scans
the filtered `project-epr` commitment relation page by page, with a fixed upper
bound and a fail-closed result if the API cannot prove exhaustion. Running the
real root exposed a route collision that nested fixtures missed: runtime
`/version` ownership also captured the app artifact `/version.json`; the route
classifier now owns only the exact runtime endpoint. Separately, an actual
rebuilt Lamad browser rendered Learning Paths with zero page errors and zero
WASM requests after host cache capability moved into `ELOHIM_ENV`. These checks
prove the local corrections. The complete committed-source five-scenario
household receipt and a fleet build carrying the commit still own the remaining
graduation evidence.
