---
title: The Substrate Trust Contract — invariants, probes, and the per-seam runbook
id: substrate-trust-contract-runbook
date: 2026-07-12
status: reference
author: dht-unity arc close (Fable session, 2026-07-11/12)
cites:
  - genesis-pair-dht-unity-plan | Genesis-Pair DHT Unity | path: genesis/docs/superpowers/plans/2026-07-11-genesis-pair-dht-unity-plan.md
  - genesis/data/timeline/backlog/genesis-pair-cross-conductor-fetch-blocks-canonical-convergence.md
  - genesis/data/timeline/backlog/sovereign-turn-relay-transport-commons.md
  - genesis/a2o/features/dataplane/notary-authority.feature
---

# The Substrate Trust Contract

**Purpose.** This is the substrate's SDK-README for agents and operators: the
invariants the dataplane now holds, the probe that watches each one, and the
decision tree to run when a probe reds. It converts the 2026-07-11/12
convergence arc's judgment into procedure, so a maintaining agent (Opus-tier
or below) can operate the substrate without re-deriving it. When this doc and
live behavior disagree, run the probes — they are the authority.

**Rescope (2026-08-02).** This doc remains *dataplane-scoped* reference — I1–I6
and their probes are the 2026-07-11/12 convergence arc's invariants and are not
restated or reinterpreted below. It now additionally carries a **concern-canon
subsection** (§1a / §2a): the four concern classes that have a **live meter**
get an invariant→probe row here, because this doc's own doctrine is that every
trust claim gets a probe. Design-time classes (C0, C1, C3, C4, C5, C6b, C9, C10,
C12, C13, C14 — and C2, whose home is the policy registry) deliberately get **no
row here**: they cite a contract test in the per-crate seam registry instead. A
probeless runbook row would advertise a watch this doc does not serve, which is
concern class C7 committed by the canon itself. The canon is defined once in
`.claude/epr-meta/concerns.yaml` (C0, C1, C3–C14) and
`.claude/epr-meta/policies.yaml#c2-monotonic-authority` — these rows *bind* it,
they do not redefine it. Standing shape of this doc's rows (the brit frame):
**pin** — a sealed fingerprint re-blessed by judgment (`cite-gen --refresh`),
never auto-blessed by recency; the guarantees themselves stand as
**attestations** in the two registry homes.

## 1. The invariants (what you may now assume)

| # | Invariant | Where enforced |
|---|---|---|
| I1 | **Verification terminates in the receiving peer's own conductor.** No peer ever adopts a head from gossip, HTTP, or announcement payloads (REQ-N5/REQ-F4). Announcements are doorbells. | serve path (`declared_head_served_blob`), heal path, declare route |
| I2 | **Canonical channels alone move a DECLARED head.** The declare route, canonical propagation, and `ContentHeadDeclared` signal stamp in Declare mode; heal/boot paths stamp GapFill (fill-only, never move). | `content_diesel::StampMode` |
| I3 | **A conductor resolve names its own authority.** `resolve_content_head` output carries `canonical: bool` — TRUE for the cross-root canonical record or a declaration act, FALSE for the root-author fallback a cold conductor gives. FALSE answers may never displace declared rows (I2). | zome `ContentHeadOutput.canonical`; `heal_content_one` |
| I4 | **One DHT space; transport is configuration.** All conductors share one DNA hash, one bootstrap store (mongo `elohim-bootstrap`), a bridged signal plane (SBD relays + mongo bus), and ICE (STUN + TURN) that actually reaches tx5 (`iceServers`, camelCase — the snake_case form is silently dead and gated at render). | conductor-config + `validate-conductor-config.sh` |
| I5 | **A freshly-authored action is not yet fetchable.** DHT publish takes minutes; anything that declares a seconds-old action to a remote peer must retry through the window (the propagation retries `not retrievable` ×4/90s). | `stage-spa-blob.sh` DECLARE_ONLY |
| I6 | **Restarts churn peer addressing for ~20 min.** Every conductor restart mints new relay-client URLs; peer stores converge after the expiry window. Measurements taken inside the window are churn artifacts, not regressions. | measured 2026-07-11 (5/7 stale → 0/7) |

**Watch-out: I6's ~20min figure is the addressing floor, not the ceiling.**
When `divergentAnchor` exceeds roughly 200 at the moment of restart, the
reconcile-projection catch-up is a different, HOURS-scale class — a
breaker-flap pattern, not a longer version of the same churn. Precedent: the
2026-07-19 doorway-catching-up incident, and the 2026-08-01 saga-recording
incident where `divergentAnchor=1763` rode a 2h+ catch-up window before
`elohim_projection_reconcile_converged` read 1 again. Third precedent
2026-08-06: a full-fleet simultaneous conductor restart (edge deploy at
16:54Z) put every DHT-dependent doorway route dark for ~2.5-3h of fleet-wide
`PTxnGuard` write-guard contention (0.6-1.9s holds even on healthy peers)
before self-recovering with no cure deployed — the simultaneous-roll shape
makes this class unavoidable until restarts are staggered
(backlog `staggered-conductor-fleet-restarts`). A measurement run fired
against the 20-min heuristic in this regime records a false red, because the
fleet is still churning long after I6's window closed. Measurement runs must
ride the bounded fleet-quiesce gate (`scripts/ci/fleet-quiesce-gate.sh`,
wired into the edge Dataplane Validation stage) rather than a fixed sleep —
it polls storage `p2p/status.pull.caughtUp`, the `elohim_projection_reconcile_converged`
gauge, and doorway content-serving on both sides, and only declares
quiescence once a fresh reconcile sweep has run and still reads converged.

### 1a. Concern-canon invariants (the live-metered classes only)

Numbering continues I1–I6. These are the **cross-family concern classes** that
happen to have a live dataplane meter; each states its guarantee in falsifiable
form, and each is watched by a probe in §2a. The "Where enforced" column names
the *meter*, not a code path — that is what makes these four eligible for a row
here at all.

| # | Invariant | Where enforced |
|---|---|---|
| I7 | **Bounded work: mint-then-quiet** (concern class C6a). Every sweep, loop and retry ladder carries a declared budget it provably respects, and once state is settled a replay mints **nothing**. A retry policy against an uncancellable call is a loop even when no `loop` token appears. | `elohim_content_canonical_links_minted_total{source}` read *against* `elohim_projection_reconcile_sweeps_total`; budget exhaustion is `elohim_content_witness_sweep_abandoned_total` |
| I8 | **Advertise/serve symmetry** (concern class C7). What a surface advertises equals what it serves — capability, inventory, provider, coverage. An advertisement that resolves to nothing is counted at the advertiser, never left to surface as a transport fault at the requester. | `elohim_content_head_record_degraded_total{cause}`, `elohim_provide_provider_unresolved_total`, `elohim_salvage_provider_unresolved_total`, `elohim_shard_push_peer_unresolved_total` |
| I9 | **Observability-per-decision** (concern class C8). Every decision outcome increments a labeled counter through a typed reason; failures are counted beside successes; no label is structurally constant; every meter names its semantics — census or sample. | `elohim_content_canonical_answers_total{tier}` (`none`/`staging`/`earned`) beside `elohim_content_contest_failed_total{class}` and `elohim_content_election_obeyed_total{path}` / `elohim_content_election_obey_failed_total{class}` |
| I10 | **Externally-imposed backpressure degrades by a declared, counted policy** (concern class C11). Under load it did not schedule, a seam defers-with-`Retry-After`, sheds, or declines — each naming its reason — never by unbounded queueing or OOM. Distinct from I7: I7 asks "does my sweep respect **my** budget", I10 asks "do I survive traffic **I did not choose**". | `doorway_admission_shed_total`, `doorway_upstream_breaker_open_total`; the `catching_up` route's `Retry-After`; `steward/node`'s `DeferReason` (`pod/admission.rs`) already discriminates I7 from I10 |

**Reading I7–I10 is comparative, never absolute.** Each is a *ratio or a pair*:
`minted` alone is meaningless without `sweeps` (mint-then-quiet is minted-flat
while sweeps-rise); `head_record_degraded` rising while requester-side
`elohim_view_federation_outbound_total{result="timeout"}` falls is the C7 cure
working, while a rise in **both** means the budget is too tight; a sustained
~100% `canonical_answers{tier="none"}` against non-zero
`elohim_projection_reconcile_divergent_refused` means no election exists to
arbitrate — a supply-side fact, not a selector fault. A bare failure count is
not a readable gauge (that is the C8 clause these rows are watched by).

## 2. The probes (how each invariant is watched)

| Probe | Surface | Watches |
|---|---|---|
| `seam-smoke[bootstrap-sharing]` | edge Dataplane Validation | both doorways read the identical bootstrap store (spaces × agents) |
| `seam-smoke[signal-bus]` | same (SKIPs until pynacl/websockets land in the runner) | SBD frames deliver cross-relay via `doorway/doorway-service/tools/sbd-cross-relay-probe.py` |
| `seam-smoke[peer-store]` | same | each doorway's PRIMARY conductor holds addressed agent-infos |
| `seam-smoke[dht-fetch]` | same (ADVISORY → flip `--gate` after scenario 2 is green ×2) | landing canonical head identical on A and B |
| `✓/⚠ canonical head propagated` | every APP deploy console (`authorHeadOnce`) | live cross-conductor declare against a freshly-authored head |
| `GET {doorway}/db/p2p/conductor-diagnostics` | on demand | the routed PRIMARY conductor's peer store (agent → relay URL); `?include=metrics` when conductor/client versions align |
| `GET {doorway}/admin/bootstrap-coherence` | on demand | kitsune2 store shape |
| `POST {doorway}/admin/steward-peers/refresh` | on demand | which storages answer, at which manifest (route counts); re-registers routes without a doorway restart |
| Loki (`instance="<name>-alpha"`, container `elohim-node`) | on demand | conductor/storage behavior; heal lines are `projection-reconcile[content]` |
| `validate-conductor-config.sh` | every human-manifest render (GATE) | ICE config actually parses into tx5's contract; dependency-free by hard requirement |

Primary routing fact for all reads: each doorway's declare/resolve rides its
PRIMARY conductor — `elohim.host`→adam (shem), `doorway-alpha`→matthew
(on-prem). "B can't X" means *adam's conductor* can't X.

### 2a. Concern-canon probes (one row per live-metered invariant)

| Probe | Surface | Watches |
|---|---|---|
| `minted` flat while `sweeps` rises | storage `/metrics` (`elohim_content_canonical_links_minted_total{source}` ÷ `elohim_projection_reconcile_sweeps_total`), on demand | **I7** — mint-then-quiet. Minted rising monotonically with sweeps against settled state is the C6a budget being missed — the sweep isn't converging; `elohim_content_witness_sweep_abandoned_total` rising is the same budget being hit from the other direction (the sweep respected it — a saturated conductor, not an unbounded loop) |
| advertiser-side unresolved counters | storage `/metrics` (`elohim_provide_provider_unresolved_total`, `elohim_salvage_provider_unresolved_total`, `elohim_shard_push_peer_unresolved_total`, `elohim_content_head_record_degraded_total{cause}`), on demand | **I8** — an advertisement that cannot be served is counted *at the advertiser*. Zero here plus requester-side timeouts is the pre-cure shape (fleet-wide adoption failure reading as a transport fault); read against `elohim_view_federation_outbound_total{result="timeout"}` |
| `elohim_content_canonical_answers_total{tier}` | storage `/metrics`; also surfaced by the edge Dataplane Validation stage | **I9** — the per-decision census the canonical seam was blind to before wave 4 (`consumed but never counted`). `tier` must actually vary: a structurally-constant label is worse than an absent one |
| `doorway_admission_shed_total` / `doorway_upstream_breaker_open_total` | doorway `/metrics`; `shedTotal` on the doorway status surface | **I10** — declared, counted degradation. A breaker that opens and never half-closes is the `halfopen_without_record_deadlocks_forever` liveness class (C3) wearing a backpressure costume — check that shed counts *stop* rising after upstream recovers |

Every row above is comparative (see the I7–I10 reading note). The design-time
classes are absent from this table **by construction**, not by omission: their
contract is a test citation in the owning crate's seam registry.

## 3. The runbook (what to do when a probe reds)

**`dht-fetch` divergent / scenario 2 red.** Run in order; stop at the first
hit:
1. **Churn window?** If a deploy restarted conductors < ~25 min ago → wait
   out the window, re-read (I6). Not a defect.
2. **Addressing converged?** Compare both doorways' `conductor-diagnostics`
   agent→URL maps. Persistent mismatch past the window → bootstrap read-path
   defect (new class — investigate; has not occurred since the ICE fix).
3. **Declare fresh-action race?** The app console's propagation line says
   `not retrievable` on all retries → check whether A's head was authored
   minutes ago (I5) — retry via a later `[build:app]` before digging.
4. **B stuck on an old DECLARED head with no adoption over hours?** Check
   Loki on the primary for `heal left it to the canonical channels`
   (fallback answers being correctly refused) vs `HEALED` (adoption). If
   fallback-refusals persist forever, B's conductor never retrieves the
   canonical record → transport question: verify `iceServers` present in the
   live conductor-config ConfigMap and TURN reachable. The dead-key class is
   render-gated, but config can drift by other paths.
5. **Escalate** with the evidence bundle: both heads + timestamps, the
   propagation console line, both diagnostics reads, the Loki heal lines.

**`peer-store` thin (<5 addressed).** The primary conductor lost bootstrap
read or just booted. Re-read after the churn window; if still thin, check
the bootstrap-coherence counts and mongo health (`mongodb-alpha` in Loki).

**`bootstrap-sharing` mismatched.** The two doorways are reading different
stores — check `BOOTSTRAP_MONGODB_DB`/`MONGODB_URI` env drift in the doorway
manifests. This was never observed post-unification; treat as config drift.

**`signal-bus` failing (once armed).** Run the outside-in probe manually
(needs `pip install pynacl websockets`). Controls pass + cross fails →
`bus_mongo.rs` drain/cursor defect or mongo down. All four legs fail →
relay/ingress problem.

**Content view sheds 503 "catching-up" while `/health` reads healthy.**
Signature: `GET /db/content/<id>` answers 503 `{"status":"catching-up"}` in
milliseconds (instant refusal, not a timeout) while the same doorway's
`/health` shows conductor pools healthy and p2p caughtUp/converged true.
Disambiguate by counters on `/metrics` (admission-exempt): 
`doorway_admission_shed_total` climbing → genuine load shed (wait/shed
storm); `doorway_upstream_breaker_open_total` climbing → the per-upstream
circuit breaker is refusing without calling storage. If the breaker stays
open past a few cooldowns (30s each) while the storage upstream answers
in-cluster probes fine, suspect the **latched half-open class**
(`upstream_health.rs` test `halfopen_without_record_deadlocks_forever`): a
caller consumed the half-open trial and never recorded an outcome — the
breaker never recovers on its own; a doorway restart resets it, the durable
cure is record-on-every-terminal-path in the caller (first observed live
2026-08-01 on both doorways at once, /db/content path, after a churn
window legitimately opened the breakers).

**Deploy failed before kubectl apply, all humans.** Read the render-stage
console — the validator and ingress-conflict gates run there and fail
loudly. Gate scripts are bash+coreutils ONLY (the deploy container has no
PyYAML — edge #1183).

**Sweettest `already exists` on multi-conductor tests** (the notary family
lives in `tests/sweettest/src/tests/lamad.rs`). Retry self-poisoning via the
process-global mem-bootstrap store — content ids must be per-invocation
(`unique_id()`); never reintroduce fixed ids in multi-conductor tests.

## 4. Change discipline (what a maintaining agent may touch)

- **May do freely:** storage/doorway native Rust (fmt/clippy/nextest gates);
  coordinator-zome changes (partition-SAFE — no DNA-hash move; but the
  hot-swap only LANDS where `ALLOW_COORDINATOR_UPDATE` is enabled, non-prod
  true / prod false — verify delivery with the zome's own error text from a
  live call, the trick that proved the selector deploy); a2o scenarios; CI scripts (test them dependency-free);
  manifests that the render gates validate.
- **Must treat as network events, never routine:** integrity-zome changes
  (DNA hash moves → partition risk; read dna-upgrade-governance first);
  `RESET_*` params; re-keying; anything under `webrtc_config` beyond adding
  servers (and keep the key camelCase — the validator enforces it).
- **Standing debts with owners:** sovereign TURN (Tier-A transport commons —
  replaces the openrelay diagnostic entries); arming `signal-bus` smoke deps
  in the CI image; flipping `dht-fetch` to `--gate` and de-`@wip`ing the
  native-omni + doorbell scenarios once green ×2; the heal-throughput smell
  (~10s per row × thousands post-restart); dump_network_stats/metrics
  version skew (works when conductor ≥ client types).

## 5. Why this doc exists

Scenario 2 was red for days as "unexplained divergence." The cure turned out
to be five stacked, individually-invisible defects — a silently-dropped
config key, a boot-time heal resurrecting superseded state, a guard that
then blocked legitimate forward adoption, a probe racing publish lag, and a
gate script that couldn't run in its own container. None were visible from
the outcome measure alone; all are now watched by named probes. The lesson
is the doctrine: **every trust claim gets a probe, every probe failure names
itself, every fix leaves its guard behind.**
