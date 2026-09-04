---
title: "EPR-app delivery verdict and layered fallback: the landing tells the truth on arrival"
id: epr-app-delivery-verdict-layered-fallback-design
status: Draft
class: design
domain: doorway projection (T4) × epr-app delivery × deploy definition-of-done
sprint: delivery-verdict
requires_env: [household-nodes]
context-tier: disclosed
steward: orchestrator
graduation-trigger: decompose-complete OR superseded-by-implementation
cites:
  - "doorway-catching-up-page | Doorway catching-up shed page | sha256:2dbde4d56b074a5e | path: genesis/docs/superpowers/specs/2026-07-19-doorway-catching-up-page-design.md"
  - "trust-legibility-atlas | Trust-Legibility Atlas | sha256:17685eb252d53116 | path: genesis/docs/superpowers/specs/2026-07-18-trust-legibility-atlas-design.md"
  - genesis/docs/superpowers/specs/2026-05-25-stagespablob-substrate-correct-deploy.md
  - "doorway-access-tier-patterns | Doorway Access-Tier Patterns | sha256:f862d55525b442c3 | path: genesis/docs/content/elohim-protocol/architecture/2026-05-23-doorway-access-tier-patterns.md"
  - genesis/a2o/features/dataplane/served-shell-boots.feature
  - genesis/a2o/features/dataplane/served-projected-head.feature
  - genesis/a2o/features/dataplane/doorway-failover.feature
  - genesis/a2o/features/trust/trust-legibility-atlas.feature
  - genesis/data/timeline/backlog/2026-09-04-doorway-projection-never-carries-app-row-heads.md
  - genesis/data/timeline/backlog/task-runtime-passport-endpoint.md
  - doorway/doorway-service/.epr-meta/doorway-failover.habit.md
  - scripts/ci/verify-projected-head.sh
  - crates/seam-contracts/src/freshness.rs
  - crates/seam-contracts/src/answer.rs
  - elohim/epr/src/witness.rs
---

# EPR-app delivery verdict and layered fallback

## 0. What this is

On 2026-09-04 both `alpha.elohim.host` and `elohim.host` answered `200` at `/` with a page
that could not boot: the shell named a previous build's entry script, the script `404`'d,
and every visitor saw a blank page for most of a day. Two pipelines reported success over
it — the app pipeline's serving seatbelt checks only that `/` answers `200`, and the
served-head probe checks only the SSR server bundle, a limitation its own header names as
a follow-up. Nothing on either side recorded a failed serve, no surface said *which layer*
had failed, and the stewards found out by looking.

This spec makes deliverability a **verdict the authoritative dataplane computes when it
takes custody of a head**, states the **fallback ladder per layer** so a visitor always
lands on a page that names where responsibility lies, and makes that verdict the
**definition of done for a deploy**. It composes existing contracts — the catching-up shed
page, the freshness colours, the `x-elohim-bundle` provenance marker, the T4-1
`servedBundleHeads` attestation, the omnibar chrome as the trust surface, the
runtime-findings harvest — and adds no parallel system.

Two contracts, one line each:

- **The peer judges; the doorway relays.** Whether a head can boot is a property of the
  bundle bytes. The storage peer holds those bytes and judges them once, locally, with no
  network; every peer re-derives the same verdict from the same CID. The doorway holds no
  judgement of its own about the app — it chooses which peer-judged head to hand out and
  reports its own layer **last**.
- **A person never lands on a page that does not name its layer.** Fallback is honest at
  every rung: the served head, the declared head, and the failing layer ride the wire and
  the chrome.

## 1. The three layers, in the order they are judged

A served EPR app crosses three responsibility boundaries. The order of judgement is the
order of authority — substrate facts first, the projection surface last — so a doorway is
never the one deciding whether an app is broken, and never blamed for a head that cannot
boot anywhere.

| # | Layer | Owner | Judged where | Failure shapes (all observed) | Page's first sentence |
|---|---|---|---|---|---|
| 1 | **EPR app** | the app's stewards (the build) | the storage peer, at extraction / head adoption — a pure function of the blob | index names assets the bundle does not hold; invalid ZIP; server bundle panics in the renderer; empty render | "The current version of this app does not start. Its stewards have been told." |
| 2 | **Peer / substrate** | the storage peer and the replication plane | the storage peer, on its own custody | `App ZIP blob not found`; blob still syncing (`503 Retry-After: 5`); `App ZIP not available (no blob_hash)`; upstream circuit open or shedding | "The peer that holds this app is not serving it right now." |
| 3 | **Doorway** | this doorway process | the doorway, about itself only | projection blind to the row's head (`x-projection-ready: false`); warm-shell era mismatch (`last-reconciled` / `slug-resolved`); SSR breaker open; warm-up empty; catching up | "This doorway is behind the app's declared version." |

The layer is a **typed outcome**, not prose: `FailureLayer { EprApp, Peer, Doorway }`.
"Three maintenance pages" is one page with a different first sentence and link set per
layer.

## 2. The deliverability verdict — computed by the peer

### 2.1 What it judges

A **head** (the browser bundle's blob CID, plus the server bundle's where one is declared)
either boots by construction or it does not. The check is local to the bytes:

1. the ZIP opens and holds an `index.html`;
2. every same-origin `<script src>` and `<link rel=stylesheet href>` that `index.html`
   names is an entry in the same ZIP;
3. (server bundle) the renderer can load the bundle — the existing materialisation step,
   whose panic yesterday (`isUint8Array`) is the observed failure.

No network, no other peer, no doorway. The verdict is therefore a **deterministic
derivation of the CID**: two peers judging the same head reach the same answer, and a
receiver can re-derive it from the bytes (C5, evidence-not-authority).

```
DeliverabilityVerdict            // one per (blob CID), at the peer that holds it
  Boots                          // index present, every named asset held
  Broken { reason }              // MissingAsset(name) | InvalidZip | NoIndex | ServerRenderFails(msg)
  NotJudged { why }              // bytes not yet held (syncing) — honest absence, never a guess
```

### 2.2 When it is computed

At the moments the peer already touches the bytes, so nothing new runs on the hot path:

- **extraction** — `handle_app_request` already opens the ZIP and walks every entry to
  fill the extraction cache; the check is a second look at `index.html` inside that walk;
- **declared-head adoption** — when the row's `blobHash` changes (PATCH, or the sweep
  converging a head authored elsewhere), the peer judges the new head before it serves it;
- **seed / stage** — the seeder and `stage-spa-blob.sh` can ask the peer for the verdict
  of what they just uploaded, before the head is authored (§7).

The verdict is memoised per CID in storage (it is a fact about immutable bytes, so it
never expires) and re-derived on demand if absent.

### 2.3 Where it shows

On every surface the peer already answers about the row, so no consumer discovers a new
route:

- `/db/content/{slug}` — the row's declared head gains
  `deliverability: { verdict, reason?, judgedAt, judgedBy }`, and, when the peer holds a
  previous head with a `Boots` verdict, `lastBootsHead`.
- `/apps/{slug}/_capability` — `X-Deliverability: boots | broken | not-judged` and
  `X-Deliverability-Reason` beside the existing `X-Blob-Hash` / `X-Ready`.
- `/apps/{slug}/index.html` — the same two headers on the shell itself.

## 3. The fallback ladder at `/` — the doorway chooses, it does not judge

The doorway's `/` serve arm (`plan_shell_serve` in `render/warm_shell.rs`, the one seam
both arms pass through since the 2026-09-04 fix) consumes the peer's verdict as an input and
its own state as the last input:

1. **Declared head, peer says `Boots`** → serve it. No `x-elohim-bundle` header, green.
2. **Declared head `Broken` or `NotJudged`, peer names a `lastBootsHead`** → serve that
   head. `x-elohim-bundle: previous-head`, `x-elohim-freshness: amber`,
   `x-elohim-served-head`, `x-elohim-declared-head`, `x-elohim-delivery: boots-behind`.
   The previous head is the **peer's** proven-good head, never the doorway's last stocked
   copy (yesterday's shape was a last-stocked copy under an empty head).
3. **No proven-good head anywhere the doorway can reach** → the **maintenance fallback**
   (§4), `503` with `Retry-After`, `x-elohim-delivery: withheld`,
   `x-elohim-failure-layer: epr-app | peer | doorway`.

The doorway's own layer enters only where the ladder cannot reach a peer answer at all —
projection blind *and* no archived verdict, breaker open, warm-up empty. Then, and only
then, the failure layer is `Doorway`, and the doorway says so about itself.

The **reduced trust signal** on rung 2 is the existing freshness pricing, not a new axis:
app bundles are `ReadClass::Knowledge`, for which amber is acceptable at every declared
stage; authority reads never ride this ladder. Standing does not change *what* the public
is served — it changes what the chrome offers on top of it (§5).

## 4. The maintenance fallback and the diagnostic page

**Maintenance page.** `root_unavailable_html` already renders a layered "doorway visible /
substrate unknown-not-zero" page with a self-reload and a link to `/threshold`, but it
fires only when no `/` projection exists at all. It becomes reachable whenever the ladder
ends in `withheld`, and its first sentence and link set come from `FailureLayer` (§1).
Every withheld render carries *why, gauge, earn-path* — the trust-legibility rule, already
canon: never a bare `503`. Links: the peer's own status where the layer is `Peer` or
`EprApp`, the doorway's (`/threshold`, `/health/serving`) where it is `Doorway`, and
**"stewards: inspect this delivery"** → the diagnostic.

**Diagnostic** `GET /db/content/{slug}/delivery` — **declared in storage's
`build_manifest()`** and served through the doorway like every other row surface. HTML for
browsers, JSON otherwise (the catching-up page's content negotiation). Sections, in
judgement order, each answering `Present | Absent | Unreachable` (seam-contracts
`Answer<T>`, C4 honest absence):

- *epr-app* — declared head and its verdict with reason; `index.html`'s asset list with
  per-asset held/missing; the server bundle's materialisation result; `lastBootsHead`.
- *peer* — this peer's custody of the head (held / syncing / absent), the sync plane's
  position for the blob, `Retry-After`.
- *doorway* — appended **last, by the doorway**, about itself only: `x-projection-ready`,
  warm-shell provenance and era, SSR breaker state and last renderer error, warm-up state.
  This is the single doorway-specific addition, and it is a section, not a page.

**Reach gate — at the peer.** Storage already answers "may this caller read this row" by
tier: creator identity, then active `stewardship_allocations → contributor_presences.steward_id`,
then collectives (`epr_service.rs`). The diagnostic is served to callers storage resolves
as a **steward of the EPR**, and to the doorway's admins; everyone else receives the
maintenance page. The doorway forwards the caller's verified identity as it does for every
row read (`X-Agent-Cid`) and adds nothing. **Honesty clause (C13):** the landing row's
`createdBy` is `None` today and no allocation names it, so slice 3 ships the gate as
*labelled scaffold authority* (doorway admin OR global `is_steward`) with the per-EPR
resolution at storage as its named graduation trigger — never an unlabelled widening.

## 5. Instant feedback on arrival

A verdict is not a report someone reads later. When the peer judges a head, and when the
doorway's ladder changes rung, it is emitted at once on channels that already exist:

1. **Wire** — the headers of §2.3 and §3 on the responses themselves.
2. **Chrome** — the omnibar context gains `deliveryOutcome`, `servedHead`, `declaredHead`,
   `failureLayer`; the element renders a "showing the previous version" badge on
   `boots-behind` and, for a signed-in steward, an *inspect* link to the diagnostic. Today
   the doorway fills only `slug` and `authenticated` of the eleven declared context fields;
   this is an additive producer change plus element markup.
3. **Metric** — at the peer `storage_deliverability_verdict_total{slug, verdict, reason}`;
   at the doorway `doorway_delivery_rung_total{slug, rung, layer}` (C8, typed and counted).
4. **Ledger** — a **transition** of a head's verdict into `Broken`, or of a slug's served
   rung into `boots-behind` / `withheld`, writes one row at the peer:
   `{ts, fp, class: deliverability, node, provenance, slug, head, verdict, reason,
   first_seen, last_seen, count}` with `fp = sha256(node|class|provenance)[:12]` — the
   runtime-findings shape `runtime-harvest.py` already collects from serving endpoints, so
   a `Broken` landing reaches the runtime-triage agent through the flag→agent path that
   exists, with no new poller. Transitions only; per-request outcomes are counted, never
   stored.
5. **Log** — one `storage::deliverability` line per verdict, one `doorway::delivery` line
   per rung change, each naming the heads and the layer.

## 6. Inventoriable with one endpoint, one command

- **Endpoint (peer).** `GET /db/apps/deliverability` — declared in storage's manifest —
  lists every EPR app row this peer holds: declared head, its verdict and reason,
  `lastBootsHead`, custody state, and the transitions ledger (bounded, newest first). This
  is the authoritative inventory: one call per peer, no doorway state in it.
- **Endpoint (doorway).** The T4-1 attestation on `/health/startup` already carries
  `servedBundleHeads[]` for the server bundle. Each entry gains the browser side —
  `blobHash`, `declaredBlobHash`, `rung` (`boots | boots-behind | withheld`), `layer`, and
  the peer's `deliverability` as relayed — so the probe that reads T4-1 today reads the
  whole picture tomorrow, and the doorway adds only what it alone knows (which rung it
  served).
- **Command.** `scripts/ci/verify-projected-head.sh` gains the browser leg its header names
  as a follow-up: for the slug the build authored, require the peer's verdict `boots` at
  the declared head and at least one doorway on rung `boots`, printing every peer's verdict
  and every doorway's rung and layer. Locally the same read is `just status delivery`, a
  thin wrapper over the peer inventory and the doorway attestation. An `epr` verb is
  deliberately **not** minted: the CLI has no runtime-findings surface today, and one would
  be a second home for a fact the endpoints own.

## 7. The deploy is not done until the landing boots

This is the leg that stops the back-patting, and it moves *earlier* now that the peer
judges: a broken bundle can be refused before its head is ever authored.

- **Stage** — `stage-spa-blob.sh` uploads the blob and then asks the peer for the verdict
  of that CID (§2.2). `Broken` stops the pipeline **before `authorHeadOnce`**: no
  witnessed head is minted for a bundle that cannot boot, so nothing has to be rolled back
  and no peer ever adopts it.
- **Author + serve** — after `authorHeadOnce`, the serving seatbelt becomes the browser leg
  of `verify-projected-head.sh`. For the slugs this build authored, the peer's verdict not
  `boots` on the declared head, or **no** doorway on rung `boots`, is a hard **FAILURE**,
  not UNSTABLE — the dependency-chain rule that keeps the orchestrator alive was written
  for transient substrate churn, and a bundle that cannot boot is not churn. Failover keeps
  the clause honest: one doorway on `boots`, or on `boots-behind` during the convergence
  window, passes with a named warning.
- **Edge** — Dataplane Validation already runs `@dataplane` stories advisory; the
  `served-shell-boots.feature` scenario runs there unchanged. It stays advisory on edge
  because the edge build does not author the bundle — the app build does, and that is
  where the gate belongs.
- **Habit** — `doorway-failover`'s new check *is* this scenario; the habit re-greens only
  on a fleet build where the peer says `boots` and a doorway serves it.

The live-target-gate trap (a gate that deadlocks its own fix) is respected: the gate reads
the verdict for the head *this build* authored; a doorway that is behind serves
`boots-behind` and passes with a warning; a doorway that is *blind* cannot pass — and
should not, because that is the 2026-09-04 shape.

## 8. Placement (P2P design gate)

### Entity: DeliverabilityVerdict (per blob CID, at the peer)
- **Classification**: Ephemeral (C). A deterministic derivation of immutable bytes;
  deleting it costs one re-judgement of a ZIP the peer already holds. Reconstruction
  strategy: re-run §2.1 on the blob.
- **Content address strategy**: it is not an address; its key **is** the blob's CID
  (`bafkrei…` canonical; the bare `sha256-…` marker is the legacy blob-plane form, whose
  migration is the named blob-plane arc, not this spec's). Two peers keying the same CID
  hold the same verdict.
- **Source of truth**: the bytes. The stored verdict is a cache of a derivation, held in
  the peer's SQLite beside the extraction cache (`-- Source of truth: derived from blob
  bytes (re-derivable)`). No `dht_anchor_hash`.
- **Integrity zome / coordinator**: none. No DHT entry, no DNA-hash movement.
- **Projections**: SQLite `app_deliverability` (cid, verdict, reason, judged_at); no
  Automerge projection (not content). Exposed on the row (§2.3) and the inventory (§6).
- **Network stakes**: identical at every declared stage — it is observation, not
  authority; nothing is stage-priceable or floor-protected here.
- **Anti-pattern check**: not a route-first design (the routes extend the row and the
  T4-1 attestation); no per-request data persisted; reach, head and replication stay
  distinct planes — the verdict names a CID, custody names a peer, reach gates the
  diagnostic; no doorway-owned truth about the app.

### Entity: DeliverabilityTransition (the ledger row of §5.4)
- **Classification**: Ephemeral (C) at the peer, with a **held** graduation: a transition
  is the one moment a community might want witnessed. The existing `WitnessedInteraction`
  primitive (`elohim/epr/src/witness.rs` — object CID, substrate, REA verb, classification
  magnitude, witness) is the shape, on the head's CID, minting **no new entry type** and
  witnessed by the peer, which is the substrate actor. Held until the ledger has run on the
  fleet for a deploy cycle and the stewards say local feedback is not enough. Head-plane
  cost if graduated: one link per transition, tens per year.

### Decision predicates
- `judge_deliverability(zip) -> DeliverabilityVerdict` — **storage**, `verdict-fn` +
  `reason-outcome-enum`, registered in `elohim/elohim-storage/seam-registry.yaml` at birth
  with contract tests (a fixture ZIP whose index names a missing asset; an invalid ZIP; a
  boots fixture).
- `choose_rung(peer_verdict, last_boots_head, doorway_state) -> Rung` — **doorway**,
  `pure-decision-predicate`, registered in `doorway/doorway-service/seam-registry.yaml`;
  it consumes the peer's answer and the doorway's own state, and its only authority is over
  which of the peer's heads to hand out.
- **Canon answers** (both): C0 plane location — the verdict on the peer-hoster dataplane
  (T2), the rung on the doorway projection (T4), stated, never mixed; C4 honest absence —
  `NotJudged` and `Unreachable` are distinct from `Broken` on every surface; C5
  evidence-not-authority — the verdict is re-derivable by any receiver from the bytes; C6a
  bounded work — one judgement per CID, inside the extraction walk; C8 observability — typed
  verdict and rung, counted; C10 contract evolution — the row and the T4-1 entry gain
  fields, remove none, and an absent `deliverability` reads as SKIP in the probe (the T4-1
  forward-compatibility idiom); C13 graduated authority — the steward gate is labelled
  scaffold with a named trigger; C3 liveness — no verdict blocks a serve; C12 consent — the
  diagnostic is reach-gated at the peer, the maintenance page is public; C1/C2/C7/C9/C11/C14
  — `n-a`: no election, no authority monotonicity, no advertise/serve split (the peer
  advertises the verdict it serves by), no identity lineage, no backpressure source, no
  residual beyond the held transition graduation.

### Routes (all declared in storage's `build_manifest()`, served through the doorway)
- `GET /db/content/{slug}` — existing; gains `deliverability` and `lastBootsHead`.
- `GET /db/content/{slug}/delivery` — the diagnostic (§4); `{slug}` is the app's routing
  name, never a hash — the page is *about* which hash the peer resolves for that slug.
- `GET /db/apps/deliverability` — the inventory (§6).
- Doorway additions are **headers and one appended section**, never a route:
  `x-elohim-delivery`, `x-elohim-failure-layer`, the `rung`/`layer` fields on the T4-1
  entry, and the *doorway* section of the diagnostic.

## 9. Slices, smallest first

1. **Peer verdict + wire + stage gate.** `judge_deliverability` inside the extraction
   walk; memoised per CID; `deliverability` on the row and `X-Deliverability` on the app
   surfaces; `stage-spa-blob.sh` refuses to proceed to `authorHeadOnce` on `Broken`;
   storage metric and log. This alone would have stopped 2026-09-04 at the stage step, on
   the right slug, naming the missing asset.
2. **Doorway rung + inventory + deploy verify.** `choose_rung` consuming the peer verdict;
   `x-elohim-delivery` / `x-elohim-failure-layer`; T4-1 entry gains the browser side; the
   browser leg of `verify-projected-head.sh`; app pipeline FAILURE on no doorway at
   `boots`; the peer inventory route.
3. **Previous-head serve + chrome badge.** `lastBootsHead` on the row (the peer keeps the
   last `Boots` head's bytes reachable); rung 2 of the ladder; chrome context fields and
   the badge.
4. **Maintenance page on `withheld` + diagnostic + scaffold reach gate + ledger.** Extend
   `root_unavailable_html` to the ladder's end with the layer sentence and links; the
   diagnostic route with the doorway section appended last; the transitions ledger and its
   runtime-harvest pickup.
5. **Held.** Per-EPR steward resolution at storage for the gate (graduation trigger of
   slice 4); witnessed transitions on the head CID; `just status delivery` as a first-class
   verb.

Each slice ends with the `served-shell-boots` scenario and a one-line delta in the
`doorway-failover` habit atom. The habit flips green on slice 2's fleet build.

## 10. What this deliberately does not do

- It does not fix the doorway projection blindness (DEV_MODE engine wiring, no
  re-projection on `content.updated`). That is its own backlog row; this design must work
  *while* that gap exists, which is why the ladder keys on the peer's verdict and the
  peer's proven-good head rather than on the doorway's projection.
- It does not give the doorway any judgement about the app. The doorway's only decisions
  are which peer-judged head to serve and what to say about its own layer, last.
- It does not touch reach vocabulary. The gate consumes storage's existing steward answer.
- It does not put per-request data anywhere durable.
