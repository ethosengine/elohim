---
id: "backlog-genesis-pair-cross-conductor-fetch-blocks-canonical-convergence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis-pair cross-conductor DHT fetch regressed — adam cannot retrieve Matthew-advertised REA commitments (notary-authority recurrence)"
slug: "genesis-pair-cross-conductor-fetch-blocks-canonical-convergence"
written: "2026-07-11"
author: "shift notary-scenario2-green"
status: "wip"
priority: "high"
area: "substrate/kitsune2-connectivity"
domain: "operator"
jobs: [elohim-edge]
relatedNodeIds:
  - "memory:project_alpha_topology_bootstrap_pair"
  - "memory:feedback_reach_head_replication_distinct_planes"
cites:
  - genesis/data/timeline/backlog/view-federation-request-flakiness-mesh-wide.md
  - peer-discovery-fractal-federation | Peer Discovery as Fractal Federation | sha256:42ae0e67f9e9d4bc | path: genesis/docs/superpowers/specs/2026-07-09-peer-discovery-fractal-federation-design.md
  - genesis/a2o/features/dataplane/notary-authority.feature
tags: [substrate, kitsune2, tx5, dht-fetch, genesis-pair, notary-authority, f-t19, signal-bus, canonical-head, rea-commitments, recurrence]
---

# Genesis-pair cross-conductor DHT fetch regression

## REOPENED 2026-07-24 — REA inventory crosses the pair; Adam's conductor cannot fetch the 62-row set

This is the same authority-boundary class this thread resolved for canonical
content on 2026-07-11, not a fourth diagnosis. The 2026-07-23 projected-head
red was correctly split to the Matthew saturation/breaker thread
(`self-heal-doorway-alpha-storage-breaker-matthew-rekey.md`): the alpha-only
probe was one build behind while Matthew's storage shed every request. Today's
evidence is the opposite direction and re-owns the conductor-fetch concern
here: both doorway diagnostics expose the same addressed agent set, Adam's
storage receives Matthew's REA inventory, and then Adam's **own conductor**
returns none for every one of the 62 gaps.

### Prometheus evidence (live alpha, 2026-07-24)

Queries ran through the read-only observability MCP, datasource `prometheus`.
The point-in-time values in the handoff are confirmed, but they are counters,
not distinct-entry counts:

| UTC sample | Adam series | value | interpretation |
|---|---|---:|---|
| 16:50:57 | `elohim_projection_heal_outcomes_total{stream="rea",outcome="missing"}` | **372** | 372 failed fetch **attempts**, not 372 IDs |
| 16:20:59–18:00:59 | `elohim_projection_reconcile_local_total{stream="rea"}` | **0 throughout** | no REA commitment ever projected on Adam during the observed series |
| 16:46:02 and 16:51:02 | `elohim_projection_heal_outcomes_total{stream="content",outcome="healed"}` | **22** | 22 successful content-heal attempts; also not a distinct-ID count |
| 16:21:04–18:01:04 | `elohim_projection_reconcile_local_total{stream="content"}` | **4,159 throughout** | content authority remained locally populated |

The REA missing counter advances by exactly **62 every five minutes**:
62, 124, 186, 248, 310, **372**, …, 1,240 at 18:00:57. The heal tracker is
rebuilt from discovery on each sweep, so this is repeated failure over a stable
62-ID gap set. It must not be reported as “372 missing entries.” The content
healed counter continued to 83 by 18:01. That rules out an Adam-wide failure of
the HcClient/conductor call path, but the two candidate sets are structurally
non-comparable: content heal is existing-row-only and
`resolve_content_head` can return a local-chain fallback, while the REA gaps are
remote-advertised IDs resolved through `IdToCommitment`. Content success does
not by itself prove the remote fetch path healthy.

Reproduction queries:

```promql
elohim_projection_heal_outcomes_total{
  pod="elohim-adam-alpha-0",stream="rea",outcome="missing"
}
elohim_projection_reconcile_local_total{
  pod="elohim-adam-alpha-0",stream="rea"
}
elohim_projection_heal_outcomes_total{
  pod="elohim-adam-alpha-0",stream="content",outcome="healed"
}
elohim_projection_reconcile_local_total{
  pod="elohim-adam-alpha-0",stream="content"
}
```

### Loki correlation — discovery succeeds; the conductor fetch is the failed leg

At 16:46:15–28, Adam received five REA inventories:

| responder peer | advertised rows |
|---|---:|
| `12D3KooWQAaK…F73N4` (Matthew; peer id already pinned in the June #1119 mesh evidence) | **62** |
| `12D3KooWBS8a…23KQv` | 18 |
| `12D3KooWFhAP…XQwwq` | 18 |
| `12D3KooWCGiw…6bJv` | 6 |
| `12D3KooWGPmV…qjK2W` | 0 |

The corresponding 16:47:39 heal line says:

```text
peers_asked=5 ids_discovered=104 healed=0 conductor_missing=62
divergent_anchor=0 local_total=0
```

Reproduction filters (use the snapshot window
`2026-07-24T16:40:00Z`–`16:55:00Z`):

```logql
{namespace="elohim-alpha",pod="elohim-adam-alpha-0",container="elohim-node"}
  |= "projection-reconcile: peer inventory received"
{namespace="elohim-alpha",pod="elohim-adam-alpha-0",container="elohim-node"}
  |= "projection-reconcile: heal complete"
```

The same result recurred at 16:54:01. At 18:01, with four responders,
`ids_discovered=98` still reduced to the same `rea_gaps=62`. Therefore every
distinct gap on Adam is present in Matthew's advertised 62-row set; the other
inventories are subsets and do not enlarge the union.

**Authorship limit:** “Matthew-advertised/held” is proven; “all 62 were
Matthew-authored” is not yet. Over the preceding 24 hours, this query returned
only Matthew's pod, with 66 author-local projection-signal events:

```logql
sum by (pod) (
  count_over_time(
    {namespace="elohim-alpha",container="elohim-node"}
      |= "Projecting Commitment from DHT" [24h]
  )
)
```

But grouping those events by `fields_id` yields only 12 distinct IDs (six
`custody-blob-*`, each observed six times; six `project-epr-*`, each observed
five times). That is consistent with authoring being concentrated on Matthew,
but it does not attribute the other 50 persisted inventory rows. The next
diagnostic must compare the 62 inventory IDs/action authors directly; do not
promote the stronger authorship claim from aggregate counts.

### Both doorway conductor diagnostics (2026-07-24 17:56–17:57 UTC)

Both public reads returned HTTP 200:

```text
https://doorway-alpha.elohim.host/db/p2p/conductor-diagnostics
https://elohim.host/db/p2p/conductor-diagnostics
```

After removing the volatile `createdAt`/`expiresAt` fields, their
`(agent, space, url, storageArc)` sets are identical:

- 35 rows = 7 agents in each of 5 spaces;
- relay split identical: 20 `signal.elohim.host` URLs and 15
  `signal.doorway-alpha.elohim.host` URLs;
- 32 full arcs `[0,4294967295]`, 3 `storageArc:null`, identically placed.

This rules out the July-11 stale-addressing/churn shape and any
peer-store/advertised-arc asymmetry for this observation.
The richer diagnostic payload is partially blind on both sides:
`transportStats` fails to decode because the client expects an `is_direct`
field that the conductor payload lacks, and `?include=metrics` fails to decode
`networkMetrics` because the expected `local_op_count` field is absent. Those
are conductor/client version-skew observability gaps,
not evidence that transport is healthy; the shared live agent map is the
confirmed fact.

### Historical anchor-divergence anchors (same lineage, not new diagnoses)

- **2026-07-07:** DHT anchor sweeps were 0–6 divergent, while content projection
  was node-localized: shem peers held ~4,158 rows with
  `divergent_anchor=0`; Matthew plateaued at 1,941–2,116, Jessica at
  2,105–2,161, and James at 2,212–3,268 for 20+ sweeps
  (`content-projection-plateau-ethosengine-household.md`).
- **2026-07-12:** this thread recorded Adam draining a roughly 2,838-row
  post-restart content divergence queue before canonical convergence could be
  measured.
- **2026-07-18:** the Matthew breaker/lineage thread measured
  `divergentAnchor` **2,091→2,177 and climbing** while storage reads shed 503.
  Its 2026-07-23 recurrence is the breaker-split rule that prevents today's
  REA-specific authority failure from being mislabeled as another generic
  saturation red.

### Bounded diagnosis and next legal move

The failing boundary is now:

```text
Matthew-held REA projection inventory
  → Adam storage discovers the 62 IDs
  → Adam's own Lamad `content_store` cell queries each ID's
    `IdToCommitment` links
  → get_rea_commitment(id) returns none for all 62
```

REA and content both use the same Lamad `content_store` cell, so a
different-DNA/cell explanation is refuted. The narrower leading hypothesis is
that the remote-authored `IdToCommitment` link ops (or their targets) do not
integrate/fetch from Matthew to Adam; content can still produce successful
attempts because it skips remote-only rows and has a local-chain fallback.
Advertised arcs and peer addressing are symmetric, while the transport metrics
that would discriminate integration/fetch are version-skew blind. The legal
first move is now a runnable red in
`tests/sweettest/src/tests/rea_commitment_replication.rs`: author one
build-unique REA commitment on isolated conductor A, exchange peer information,
then resolve it through conductor B within 60 seconds. The existing target had
been `#[ignore]`d even though the Jenkins nextest shard does not pass
`--run-ignored all`; it is now live and uses a per-invocation `unique_id()` so
the process-global mem-bootstrap store cannot self-poison retries.

A focused local run on 2026-07-24 reproduced the failure:

```text
Error: Bob could not retrieve Alice's REA commitment
test-project-epr-doorway:test|epr:lamad-1784916935702110971
via get_rea_commitment within 60s after peer exchange

test result: FAILED. 0 passed; 1 failed; finished in 229.23s
```

The total includes conductor setup; the retrieval assertion itself is bounded
to 60 seconds. Conductor tuning or manifest changes remain proposals until this
red plus action-author evidence pins the substrate mechanism.

## What works (proven live, 2026-07-11 overnight shift)

The entire notary head-election chain above the substrate is DONE:
tier-aware cross-root selector hot-swapped onto BOTH genesis conductors
(functional proof: scenario 3 green — the new guard refuses unauthorized
moves; elohim.host's conductor answers the new fn's own retrievability
refusal from lib.rs:3207), declaration act wired (deploy designates via
`POST /db/content/{id}/canonical-head`, propagated to EVERY doorway each
app deploy — `DECLARE_ONLY` leg in `scripts/ci/stage-spa-blob.sh`), and
the declaring side converges within the same deploy.

## The blocker

`elohim.host`-side conductors cannot RETRIEVE matthew-authored actions:
every propagation attempt returns

    Guest("declare_canonical_head: target action ActionHash(...) is not retrievable")

(zome-side network `get` timeout). Watched divergent 48+ minutes across
multiple freshly-authored heads. This is the F-T19 class the spine's
notary-authority evidence already names.

**DELTA 2026-07-11 (dht-unity T1/T2 executed):** candidate 2 is REFUTED —
the outside-in SBD probe (`doorway/doorway-service/tools/sbd-cross-relay-probe.py`,
authenticated clients on both PUBLIC relays) delivered all four legs
including cross-relay A→B and B→A: the mongo bus bridges frames at runtime.
Bootstrap is also proven shared end-to-end (both doorways'
`/admin/bootstrap-coherence`: identical 5 spaces × 35 agents — every
conductor publishing). T2 pinned the failing member: each doorway's
declare/resolve rides its PRIMARY conductor (B→adam, A→matthew), so this is
specifically ADAM's conductor (shem-pinned, cloud NAT) failing to fetch from
MATTHEW's (on-prem, home NAT). Both conductor configs are STUN-only — **no
TURN anywhere in the manifests** — so a failed srflx↔srflx ICE pairing has
no relay fallback (unless tx5's own sbd-relay data fallback engages — the
open question). Remaining candidates, sharpened:

1. tx5 ICE failure with no TURN fallback (WAN NAT pair adam↔matthew) —
   and/or tx5's sbd-relay data fallback not engaging despite the now-bridged
   relay plane; possibly stuck long-backoff sessions.
2. Node-level egress on the shem side (original F-T19 framing: outbound
   timeouts to ~11 peers).

**Instrument (deploys with the next edge build):**
`GET /db/p2p/conductor-diagnostics[?include=metrics]` — the conductor's own
peer store, live transport connections, fetch queue with `peers_on_backoff`,
gossip round summaries. Read it on BOTH doorways to pick between the two
candidates with evidence, then cure (TURN deploy / backoff flush / egress
fix) as ONE measured change.

**DELTA 2026-07-11 (evening) — seam DEFINITIVELY isolated to the data channel.**
App #1604 ran the declare propagation in a verified CONVERGED peer-store
window (conductor-diagnostics on both doorways: 0/7 agents URL-mismatched at
fire time) and STILL got the not-retrievable refusal on every leg. Combined
with T1 (bus delivers cross-relay 4/4) and the shared-bootstrap proof, every
layer above the WebRTC data channel is now exonerated; candidate 1 (ICE with
no TURN across the shem↔on-prem NAT pair) is the seam. CURE SHIPPED: TURN
relay fallback (diagnostic third-party, TCP:80/443) added to
webrtc_config.ice_servers in the edgenode template + adam manifest; sovereign
replacement tracked in sovereign-turn-relay-transport-commons.md. The
measurement-during-churn effect is REAL but secondary (stale peer-store URLs
for ~20min post-restart — quantified 5/7 mismatched → 0/7 after expiry) — it
delays convergence after every deploy but is not the root cause.

**ROOT CAUSE REFINED (2026-07-11, late):** the conductor config key was
`ice_servers` (snake_case) — but Holochain passes `webrtc_config` verbatim
into tx5's `WebRtcConfig`, which is `#[serde(rename_all = "camelCase")]`:
the real key is **`iceServers`** (Holochain's own conductor-config doc
example uses camelCase). The snake_case key was silently ignored, so the
fleet has run with ZERO ICE servers since inception — no STUN, no srflx,
host candidates only. That is why WebRTC worked while the pair was
co-located (2081-anchor era) and died at the 2026-05-27 shem split, and why
the first TURN cure (same wrong key) changed nothing. Fix: key renamed to
`iceServers` (template + adam manifest) — STUN + TURN now actually reach the
WebRTC backend.

**RESOLVED 2026-07-11 20:40 UTC.** After the `iceServers` key fix deployed
(edge #1179), elohim.host's conductor retrieved + verified the
matthew-authored declared head and adopted it: both doorways now resolve
identical headActionHash + blobHash for elohim-host-landing
(uhCkkPVC7g… / sha256-84e1d803…, trust=notarized, same updatedAt). The
fetch seam is closed; scenario-2 green banking (×2 fresh edge validations)
is measurement follow-through. Guards added so this class cannot recur
silently: render-time conductor-config ICE validator (gates every human
manifest render) + substrate-seam-smoke in Dataplane Validation.

**POST-RESOLUTION REGRESSION + GUARD (2026-07-11 ~20:42):** two minutes
after the first adoption, edge #1180's deploy restarted B's storage and the
boot-time projection-reconcile heal RESURRECTED the superseded head: the heal
calls resolve_content_head, which falls through to the root-author election
while the canonical link is not yet retrievable on a cold conductor, and
stamped that fallback over the adopted canonical row (scenario 4's exact
durable-upgrade class — the first live seam-smoke run caught it:
dht-fetch ADVISORY-DIVERGENT). GUARD: stamp_declared_head gains
StampMode::{Declare,GapFill} — heal/boot paths are GapFill and can never
move a row that already carries a different declared head; only canonical
channels (declare route, propagation, ContentHeadDeclared signal) move
declared heads. Unit-tested (gapfill_stamp_never_resurrects_over_a_declared_head).

**SHIFT-END STATE 2026-07-12 (all fixes deployed; convergence now GATED, not blocked).**
Fleet rolled with the full stack (canonical-aware coordinator + heal, validator
7/7, guards) — edge #1186, adam on fresh pod. App #1611 re-authored A's head
(uhCkko_clhqTYHFR) and ran the declare+propagate leg. B (adam) did NOT converge
within the watch window — but the reason is now precise and NOT an unknown:
adam's `/db/content/.../head` returns `503 {"status":"catching-up",
"retryAfter":30}`. After its restart, adam is grinding its post-restart
reconcile backlog (the ~2,838-row divergence queue), and the head + likely the
canonical-head POST endpoints are gated 503 during catch-up. So convergence for
any ONE row waits on adam draining the WHOLE backlog — the heal-throughput smell
(~10s/row × thousands ≈ hours) has become load-bearing for convergence TIMING.
NEXT AGENT: (1) re-read elohim.host /head — if catch-up finished and it shows
uhCkko_clhqTYHFR (or newer A head), CONVERGED → bank scenario 2. (2) If still
503 catching-up, wait for drain OR address heal throughput (parallelize /
prioritize the declared row / raise WITNESS_MAX_PER_TICK). (3) If catch-up done
but head is still the OLD uhCkkwLFE, the declaration did not land during catch-up
503 — re-fire [build:app] once adam is serving 200s, and the 8× ladder will
land it. Everything is deployed; this is timing + the filed heal-throughput
residual, not a new defect. Do NOT push while an app build is mid-flight
(re-authors the target the ladder is chasing).

**DELTA 2026-07-12 (midday) — shift-end branch 3 was HALF the story: it landed,
then regressed.** The decision tree assumed the declaration never landed. Live
evidence says otherwise: edge #1187's seam-smoke (~05:40Z) read `dht-fetch: OK —
CONVERGED (uhCkko_…)` on BOTH doorways — B's row briefly held the fresh head —
and #1188 (~07:40Z) read `ADVISORY-DIVERGENT — A=uhCkko_ B=uhCkkwLFE` again.
The mover: adam's own-conductor heal (`projection-reconcile[content]: HEALED
content anchor from own conductor`, 11:26:49Z — matches the row's updatedAt to
the second). Adam's conductor had not integrated the newer canonical link, so
its resolve answered the OLD canonical record — `canonical: true`, yet STALE —
and `heal_content_one`'s canonical⇒Declare stamp legally moved the declared
head BACKWARDS. Same resurrection class as the 2026-07-11 20:42 regression,
one level up: the GapFill guard covered fallback answers; a stale canonical
answer is the same rollback wearing a canonical costume.

Second defect, same window: the DECLARE_ONLY retry ladder treated HTTP 503
(`{"status":"catching-up"}` — adam's admission layer shedding writes while the
reconcile drain holds the write pool) as STRUCTURAL and aborted at attempt 3/8
(app #1612). Backpressure is retryable by definition — the ladder gave up on
the exact signal that says "try again shortly."

CURE SHIPPED (this commit):
1. `StampMode::HealCanonical` — heal with a canonical answer may fill, refresh
   the same head, or move a declared row ONLY with proof of forward ordering:
   new `content.declared_head_at` column (zome declaration Timestamp, i64 µs)
   compared against the answer's `ContentHeadWire.declared_at`; NULL on either
   side refuses the move. Deliberate channels (declare route, propagation POST,
   own-conductor signal) keep unconditional Declare (revert stays a legitimate
   authority act). Unit: `heal_canonical_stamp_is_monotonic`.
2. Ladder: 503/429 now retry on a 30s cadence (12 attempts total; 90s stays
   the not-retrievable cadence).
Booting the new binary cannot resurrect: pre-migration rows wake with
`declared_head_at = NULL`, and NULL-ordering rows are unmovable by heal.

**DELTA 2026-07-12 (evening) — app-layer machinery COMPLETE; blocker isolated to
the dead TURN fallback.** With the monotonic-heal guard + resilient ladder
deployed, three declare cycles ran: #1613 (12×, all 503-shed), #1614 (12×,
mixed shed + zome not-retrievable, refused at ~18min post-author), #1615
(24× ≈ 36min, STILL not-retrievable at attempt 23). Meanwhile the guard held:
adam never regressed further (B pinned at uhCkkwLFE all day — stale but
stable). Substrate evidence for the window (Loki, both sides):
- matthew: `kitsune2 core_publish: could not send publish ops: tx5 send error
  … timed out` — the authoring side cannot PUSH its ops out; "not retrievable"
  on adam is downstream of failed publish, not just failed fetch.
- BOTH sides: the diagnostic third-party TURN (openrelay.metered.ca →
  216.39.253.123) is unreachable — `no route to host` from shem, `connection
  timed out` from matthew's network. The free relay service is dead. The
  2026-07-11 iceServers cure therefore provides STUN-only in practice:
  srflx↔srflx across shem-cloud ↔ home NAT is an ICE lottery (it won at
  #1187 ~05:40Z, lost all evening).
CONCLUSION: scenario-2 convergence is now blocked SOLELY on
[[sovereign-turn-relay-transport-commons]] (or any reachable TURN) — operator
domain. Once one declare lands, `declared_head_at` ordering is recorded and
every future deploy self-converges through the monotonic heal even when the
POST window is missed. No further app-layer work is useful until the relay
leg exists.

## The standing diagnostic (free, every deploy)

Every app deploy now emits the live probe in the `authorHeadOnce` /
propagation console output: `✓ canonical head propagated to <doorway>`
means the fetch path healed; the `⚠ not retrievable` warning means it is
still down. No manual reproduction needed.

## Why this is the last mile

The moment cross-conductor fetch works, convergence is AUTOMATIC on the
next app deploy (no code change): the propagation declare succeeds on the
second peer, its row eager-stamps, and notary-authority scenario 2 flips
green. Everything above the substrate is waiting on this single link.
