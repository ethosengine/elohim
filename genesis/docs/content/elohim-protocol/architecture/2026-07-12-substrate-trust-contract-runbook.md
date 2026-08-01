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
`elohim_projection_reconcile_converged` read 1 again. A measurement run fired
against the 20-min heuristic in this regime records a false red, because the
fleet is still churning long after I6's window closed. Measurement runs must
ride the bounded fleet-quiesce gate (`scripts/ci/fleet-quiesce-gate.sh`,
wired into the edge Dataplane Validation stage) rather than a fixed sleep —
it polls storage `p2p/status.pull.caughtUp`, the `elohim_projection_reconcile_converged`
gauge, and doorway content-serving on both sides, and only declares
quiescence once a fresh reconcile sweep has run and still reads converged.

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
