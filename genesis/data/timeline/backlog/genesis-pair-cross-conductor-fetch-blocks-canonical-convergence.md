---
id: "backlog-genesis-pair-cross-conductor-fetch-blocks-canonical-convergence"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Genesis-pair cross-conductor DHT fetch is down — elohim.host-side conductors cannot retrieve matthew-authored actions, the SOLE remaining blocker for notary-authority scenario 2"
slug: "genesis-pair-cross-conductor-fetch-blocks-canonical-convergence"
written: "2026-07-11"
author: "shift notary-scenario2-green"
status: "resolved"
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
tags: [substrate, kitsune2, tx5, dht-fetch, genesis-pair, notary-authority, f-t19, signal-bus, canonical-head]
---

# Genesis-pair cross-conductor DHT fetch is down

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
