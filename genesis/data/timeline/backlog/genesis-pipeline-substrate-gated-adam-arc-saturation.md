---
title: Genesis pipeline stability is substrate-gated on adam's full-arc gossip saturation
status: open
ci_status: blocked
severity: high
discovered: 2026-07-06
discovered_by: shift/overnight-genesis-pipeline-stabilize
domain: dataplane / substrate
ceiling: true
pipelines: [elohim-genesis]
requires_env: alpha-cluster-6peer
needs: brainstorm  # target_arc_factor policy for a genesis seed anchor is a design decision
---

## Current decision

**BLOCKED — above the shift ceiling.** The genesis pipeline cannot be flipped to
stable by any in-repo change tonight because its stability is gated on the alpha
substrate, specifically the genesis anchor **adam** being gossip-saturated. The
real lever — `target_arc_factor < 1` on a *seed anchor* — trades content
availability and needs an operator design decision + a deploy (cluster access this
shift does not have). Route to `/brainstorm` for the arc-factor policy.

## Evidence (grounded, 2026-07-06 ~04:50 UTC)

- **genesis #1258 UNSTABLE → #1259 FAILURE → #1260 (genesis-alone) FAILURE.** All three
  recent genesis builds are red; #1259/#1260 fail *harder* (FAILURE) at "Verify Target
  Health" with **exit 124** (a `timeout 120s` on `curl "${TARGET_HOST}"` where
  TARGET_HOST=`https://alpha.elohim.host`).
- **`alpha.elohim.host/` returns HTTP 503** (sustained; Retry-After 30). The 120s probe
  loops on 503 → times out → hard FAILURE → all seeding/verify stages cascade.
- **Doorway pods are HEALTHY** (elohim-doorway-alpha + alpha-b: ready, 0 restarts,
  serving /health, conductor-connected) — so the 503 is at the serve-root layer, NOT a
  doorway crash. The root path needs conductor-served EPR/SPA content that a saturated
  adam can't compose.
- **adam conductor is full-arc gossip-saturated:** logs are a wall of `kitsune2_gossip`
  round timeouts + `Failed to send force initiate err="Full(..)"` (queue overflow) +
  `Outbound sync/shard request failed: Timeout`; arc_set = full **512/512 sectors**.
  READY=1, memory 3.5/8 GiB (NOT OOM), but restarting intermittently (3× in 3h).
- The reanchor heal (elohim-storage dev-8a1f7a29) **is live on adam and confirmed** — it
  stopped the reanchor *thrash*, but the *full-arc gossip working set* is the deeper,
  separate pressure. Heal necessary, not sufficient.

## H2 refuted

Hypothesis that the FAILURE was a self-inflicted deploy-fronted wave (combined
`[build:genesis,app]` redeploying alpha.elohim.host concurrent with genesis) was tested
with a **genesis-alone** retrigger (c53a912e2, `[build:genesis]`) → still FAILURE at the
same stage. The 503 is present with nothing deploying → substrate, not deploy-front.

## Operator decision menu (escalation)

1. **arc-factor policy for genesis anchors** — set `target_arc_factor < 1` on adam
   (and/or matthew) so the anchor holds a bounded arc slice instead of the full 512
   sectors. Trade-off: content availability/authority on the seed anchor vs gossip load.
   This is the crux fix. (Needs `/brainstorm` + a deploy.)
2. **Bulk-purge adam's leaked `e2e-*` content** to shrink his working set (the reanchor
   heal only stops future thrash; already-leaked rows persist).
3. **Bounce adam** after (1)/(2) so he reloads a smaller arc.

## Companion pipeline-robustness candidate (in-repo, needs a healthy substrate to verify)

`genesis/Jenkinsfile` "Verify Target Health" gates seed-readiness on `TARGET_HOST`
(the app SPA host) responding — but seeding targets INTERNAL_STORAGE_URL /
INTERNAL_DOORWAY. Coupling the seed gate to SPA-root availability means an app-host 503
(orthogonal to storage/doorway readiness) hard-fails the whole run. Consider decoupling
(gate on storage/doorway readiness; treat app-host as browser-suite precondition only).
NOT done tonight — unverifiable while the substrate is degraded.
