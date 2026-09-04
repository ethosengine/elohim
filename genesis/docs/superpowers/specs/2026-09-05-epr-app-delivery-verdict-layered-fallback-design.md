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

This spec makes delivery a **verdict the serving side computes on arrival**, states the
**fallback ladder per layer** so a visitor always lands on a page that names where
responsibility lies, and makes that verdict the **definition of done for a deploy**. It
composes existing contracts — the catching-up shed page, the freshness colours, the
`x-elohim-bundle` provenance marker, the T4-1 `servedBundleHeads` attestation, the omnibar
chrome as the trust surface, the runtime-findings harvest — and adds no parallel system.

The one-line contract: **a doorway never hands a person a page it has not judged, and
never judges without saying at which layer the judgement failed.**

## 1. The three layers, and the page each one owes

A served EPR app crosses three responsibility boundaries. A failure belongs to exactly one,
and the fallback page must say which — "three maintenance pages" is the same page with a
different first sentence and a different set of links.

| Layer | Owner | Failure shapes (all observed) | Page's first sentence |
|---|---|---|---|
| **Peer / substrate** | the storage peer and the replication plane | upstream circuit open or shedding; `App ZIP blob not found`; blob still syncing (`503 Retry-After: 5`); `App ZIP not available (no blob_hash)` | "The peer that holds this app is not serving it right now." |
| **Doorway** | this doorway process | projection blind to the row's head (`x-projection-ready: false`); warm-shell era mismatch (`last-reconciled` / `slug-resolved`); SSR breaker open; warm-up empty; catching up | "This doorway is behind the app's declared version." |
| **EPR app** | the app's stewards (the build) | declared head held but broken by construction: index names assets the bundle does not hold; server bundle panics in the renderer; empty render; invalid ZIP | "The current version of this app does not start. Its stewards have been told." |

The layer is a **typed outcome**, not prose: `FailureLayer { Peer, Doorway, EprApp }`. It
is chosen by the first failing check in the verdict order of §2, which is also the order
of responsibility — a peer that cannot serve is never blamed on the app.

## 2. The delivery verdict

Computed at the one seam both `/`-serving arms already pass through: `plan_shell_serve`
in `doorway/doorway-service/src/render/warm_shell.rs`, which returns the declared head
(the doorway's projection) and the served head (the warm shell's `blob_hash` +
`head_bound`). That seam knows heads; it does not know **deliverability**. The verdict
adds one check and one enum.

```
DeliveryOutcome
  Boots        { head }                       // declared head served; every named asset resolves
  BootsBehind  { served, declared }           // a proven-good previous head served (amber)
  Broken       { layer, reason, head }        // the head this doorway would serve cannot boot
  Withheld     { layer, reason }              // nothing servable; the fallback page was handed out
```

**Deliverability check** (the new piece): parse the shell's `<script src>` and
`<link rel=stylesheet href>` (same-origin, relative names) and require each to resolve at
the served head. Resolution is local first — `AppFileCacheService::get(slug, asset, head)`
— then one bounded `HEAD /apps/{head}/{asset}` to storage under the existing 2 s upgrade
budget. The result is **memoised per `(slug, head)`**: a head is judged once per process,
re-judged only when the head changes or the archive is evicted. The steady-state hot path
pays nothing.

**Verdict order** (first failure names the layer):

1. upstream available? — no → `Withheld { Peer }` (the existing catching-up shed, unchanged)
2. declared head known to this doorway? — no → doorway layer *unless* a proven-good head
   exists (→ `BootsBehind`)
3. bytes for the head obtainable? — `App ZIP blob not found` / syncing → `Peer`
4. shell obtained but its assets do not resolve at that head → `EprApp`
5. SSR breaker open with a renderer panic for this head → `EprApp` (the CSR shell still
   boots, so this is `Boots` with a *server-render* concern, not `Broken`; recorded as a
   reason, never as a failure of delivery)

Every verdict is **evidence the receiver re-derives** (C5): the page's own asset list and
the doorway's own fetches, not a claim forwarded from storage.

## 3. The fallback ladder at `/` (public)

Ordered, and each rung marked on the wire so the trust surface can render it:

1. **Declared head boots** → serve it. No `x-elohim-bundle` header (confirmed head), freshness green.
2. **Declared head broken or not yet servable, a proven-good previous head exists** →
   serve the previous head. `x-elohim-bundle: previous-head`, `x-elohim-freshness: amber`,
   `x-elohim-served-head`, `x-elohim-declared-head`, `x-elohim-delivery: boots-behind`.
   "Proven-good" means a head with a recorded `Boots` verdict **at this doorway** — never
   merely the last stocked copy (the 2026-09-04 shape was a last-stocked copy).
3. **No proven-good head** → the **maintenance fallback** (§4), `503` with `Retry-After`,
   `x-elohim-delivery: withheld`, layer named in the body and in `x-elohim-failure-layer`.

The **reduced trust signal** on rung 2 is the existing freshness pricing, not a new axis:
app bundles are `ReadClass::Knowledge`, for which amber is acceptable at every declared
stage; authority reads never ride this ladder. Standing does not change *what* the public
is served — it changes what the chrome offers on top of it (§5).

## 4. The maintenance fallback and the diagnostic page

**Maintenance page.** `root_unavailable_html` already renders a layered "doorway visible /
substrate unknown-not-zero" page with a self-reload and a link to `/threshold`, but it fires
only when no `/` projection exists at all. It becomes reachable whenever the verdict is
`Withheld`, and its first sentence and link set come from `FailureLayer` (§1). Every
withheld render carries *why, gauge, earn-path* — the trust-legibility rule, already canon:
never a bare `503`. Links: doorway status (`/threshold`, `/health/serving`), the peer's
status where the layer is `Peer`, and **"stewards: inspect this delivery"** →
`/epr/{slug}/_delivery`.

**Diagnostic page** `GET /epr/{slug}/_delivery` — HTML for browsers, JSON otherwise (the
catching-up page's content negotiation). It lives in doorway-service because it answers
"what is **this doorway** trying to serve, and where does it stop" — a doorway-specific
fact, the one case `doorway/CLAUDE.md` reserves for doorway routes. Three sections, one
per layer, each answering `Present | Absent | Unreachable` (seam-contracts `Answer<T>`,
C4 honest absence):

- *peer* — primary endpoint, circuit, `blob_hash` held / syncing / absent, `Retry-After`
- *doorway* — `x-projection-ready`, warm-shell provenance and era, SSR breaker state and
  last renderer error, warm-up state, upgrade attempts since head change
- *epr-app* — declared head, served head, the shell's asset list with per-asset
  resolution, the renderer's last panic for this head, the last `Boots` head

**Reach gate.** The page is served to (a) doorway admins and (b) **stewards of the EPR**.
Steward resolution does not mint a new authority: the doorway asks storage the question it
already answers for content reads — creator identity plus active
`stewardship_allocations → contributor_presences.steward_id` (`epr_service.rs`) — via one
bounded call, cached per `(slug, agent_cid)`. Everyone else receives the maintenance page.
**Honesty clause (C13):** today the landing row's `createdBy` is `None` and the doorway
holds only a global `is_steward` boolean, so slice 3 ships the gate as *labelled scaffold
authority* (`admin OR is_steward`) with the per-EPR resolution as its named graduation
trigger — never an unlabelled widening.

## 5. Instant feedback on arrival

A verdict is not a report someone reads later. On the first `/` after a head change (and on
every post-deploy probe) it is emitted five ways at once, all existing channels:

1. **Wire** — the headers of §3 on the response itself.
2. **Chrome** — the omnibar context gains `deliveryOutcome`, `servedHead`, `declaredHead`,
   `failureLayer`; the element renders a "showing the previous version" badge on
   `BootsBehind` and, for a signed-in steward, an *inspect* link to `_delivery`. Today the
   doorway fills only `slug` and `authenticated` of the eleven declared context fields; this
   is an additive producer change plus element markup.
3. **Metric** — `doorway_delivery_verdict_total{slug, outcome, layer}` beside the existing
   freshness and breaker counters (C8, typed and counted).
4. **Ledger** — a **transition** into or out of `Broken` / `Withheld` writes one row
   `{ts, fp, class: delivery, node, provenance, slug, served, declared, outcome, layer,
   reason, first_seen, last_seen, count}` with `fp = sha256(node|class|provenance)[:12]` —
   the runtime-findings shape `runtime-harvest.py` already collects, so a `Broken` landing
   reaches the runtime-triage agent through the flag→agent path that exists, with no new
   poller. Only transitions are recorded; per-request outcomes are counted, never stored.
5. **Log** — one `doorway::delivery` line per transition, naming both heads and the layer.

## 6. Inventoriable with one endpoint, one command

- **Endpoint.** The T4-1 attestation on `/health/startup` already carries
  `servedBundleHeads[]` for the server bundle. Each entry gains the browser side:
  `blobHash`, `declaredBlobHash`, and `delivery: { outcome, layer, reason, checkedAt }`.
  One entry per served EPR per doorway; the transitions ledger is exposed beside it as
  `delivery.transitions[]` (bounded, newest first). Nothing new to discover: the probe that
  reads T4-1 today reads this tomorrow.
- **Command.** `scripts/ci/verify-projected-head.sh` gains the browser leg its header names
  as a follow-up: for the slug the build authored, require `delivery.outcome == boots` at
  the declared head on at least one doorway, and print every doorway's verdict, layer and
  reason. Locally the same read is `just status delivery` (a thin wrapper over the same
  endpoint on each doorway). An `epr` verb is deliberately **not** minted in this spec: the
  CLI has no runtime-findings surface today, and one would be a second home for a fact the
  endpoint already owns.

## 7. The deploy is not done until the landing boots

This is the leg that stops the back-patting.

- **App pipeline** (`Jenkinsfile`, root): the serving seatbelt after `authorHeadOnce`
  becomes the browser leg of `verify-projected-head.sh`. For the slugs this build authored,
  `outcome != boots` on **every** doorway is a hard **FAILURE**, not UNSTABLE — the
  dependency-chain rule that keeps the orchestrator alive was written for transient
  substrate churn, and a bundle that cannot boot is not churn. Failover keeps the clause
  honest: one doorway booting the head (or serving `boots-behind` during the convergence
  window) is a pass with a named warning.
- **Edge pipeline**: Dataplane Validation already runs `@dataplane` stories advisory; the
  `served-shell-boots.feature` scenario runs there unchanged. It stays advisory on edge
  because the edge build does not author the bundle — the app build does, and that is where
  the gate belongs.
- **Habit**: `doorway-failover`'s new check *is* this scenario; the habit re-greens only on
  a fleet build where the verdict says `boots`.

The live-target-gate trap (a gate that deadlocks its own fix) is respected: the gate reads
the verdict for the head *this build* authored, and a doorway that is behind serves
`boots-behind`, which passes with a warning. A doorway that is *blind* cannot pass — and
should not, because that is the 2026-09-04 shape.

## 8. Placement (P2P design gate)

### Entity: DeliveryVerdict (the per-`(doorway, slug, head)` judgement)
- **Classification**: Ephemeral (C). Reconstructable at any time by re-running the check
  against the served head; deleting it costs one re-judgement.
- **Content address strategy**: none — it is not an address. Its key is a dedupe
  fingerprint (`sha256(node|class|provenance)[:12]`), the runtime-findings convention; the
  heads it names are the existing `sha256-…` blob markers (legacy form, migration to
  `bafkrei…` is the named blob-plane arc, not this spec's).
- **Source of truth**: doorway-local (in-memory, archived in the doorway's Mongo beside
  the app-file archive so it outlives the pod). Storage is not asked to hold it — the
  verdict is *this doorway's* judgement of *its own* delivery.
- **Integrity zome / coordinator**: none. No DHT entry, no DNA-hash movement.
- **Projections**: none to SQLite or Automerge. Exposed on `/health/startup` (§6).
- **Network stakes**: behaves identically at every declared stage; nothing here is priced
  by stakes — it is observation, not authority.
- **Anti-pattern check**: not a table; not a route-first design (the route in §6 extends
  an existing attestation); per-request outcomes are counted, never persisted (no granular
  data anywhere durable); reach is not conflated with head or replication — the verdict
  names a head and a layer, never a reach.

### Entity: DeliveryTransition (the ledger row of §5.4)
- **Classification**: Ephemeral (C) locally, with a **held** graduation: a transition is
  the one moment a community might want witnessed. The existing `WitnessedInteraction`
  primitive (`elohim/epr/src/witness.rs` — object CID, substrate, REA verb, classification
  magnitude, witness) is the shape for that, on the EPR head's CID, minting **no new entry
  type**. Held until the transitions ledger has run on the fleet for a deploy cycle and the
  stewards say the local feedback is not enough. Head-plane cost if graduated: one link per
  transition, tens per year, far under the bundling threshold.

### Decision predicate: `judge_delivery` → `DeliveryOutcome` / `FailureLayer`
- **Kind**: `verdict-fn` + `reason-outcome-enum`; registered in
  `doorway/doorway-service/seam-registry.yaml` at birth with contract tests.
- **Canon answers**: C0 plane location — doorway projection (T4), the plane that serves;
  C4 honest absence — `Absent | Unreachable | Refused` are distinct in every section of the
  diagnostic; C5 evidence-not-authority — the verdict is receiver-re-derived (own fetches);
  C6a bounded work — one memoised judgement per `(slug, head)`, 2 s budget; C8
  observability-per-decision — typed outcome + layer, counted; C10 contract evolution —
  the `servedBundleHeads` entry gains fields, removes none, and an absent `delivery` field
  reads as SKIP in the probe (the T4-1 forward-compatibility idiom); C13 graduated
  authority — the steward gate is labelled scaffold with a named trigger (§4); C3 liveness —
  no verdict blocks a serve; C12 consent — the diagnostic is reach-gated, the maintenance
  page is public; C1/C2/C7/C9/C11/C14 — `n-a`: no election, no authority monotonicity, no
  advertise/serve split (the verdict *is* the serve), no identity lineage, no backpressure
  source, no residual to witness beyond the held transition graduation.

### Route: `GET /epr/{slug}/_delivery`
- Doorway-specific by definition (§4); not declared in storage's manifest. HTML/JSON
  negotiated. `{slug}` is the EPR slug (the app's routing name), never a hash — the page
  is *about* which hash the doorway resolves for that slug.

## 9. Slices, smallest first

1. **Verdict + wire + inventory + deploy gate.** `judge_delivery` at `plan_shell_serve`;
   headers; `servedBundleHeads[].delivery`; browser leg of `verify-projected-head.sh`; app
   pipeline FAILURE on `!boots`; metric; log. This alone would have turned 2026-09-04 into
   a red build within the deploy, on the right slug, with the layer named.
2. **Previous-head serve + chrome badge.** Proven-good archive (a head is stocked as
   *good* only after a `Boots` verdict); rung 2 of the ladder; chrome context fields and
   the badge.
3. **Maintenance page on `Withheld` + diagnostic page + scaffold reach gate.** Extend
   `root_unavailable_html` to the broken-projection case with the layer sentence and links;
   `/epr/{slug}/_delivery`; transitions ledger and the runtime-harvest pickup.
4. **Held.** Per-EPR steward resolution for the gate (graduation trigger of slice 3);
   witnessed transitions on the head CID; `just status delivery` as a first-class verb.

Each slice ends with the `served-shell-boots` scenario and a one-line delta in the
`doorway-failover` habit atom. The habit flips green on slice 1's fleet build.

## 10. What this deliberately does not do

- It does not fix the doorway projection blindness (DEV_MODE engine wiring, no
  re-projection on `content.updated`). That is its own backlog row; this design must work
  *while* that gap exists, which is why rung 2 keys on a proven-good head rather than on
  the projection.
- It does not add a storage-side record of failed serves. Storage's `/apps` 404s stay
  honest status codes; the doorway is the layer that owes the person a page, so the
  doorway owns the verdict.
- It does not touch reach vocabulary. The gate consumes storage's existing steward answer.
- It does not put per-request data anywhere durable.
