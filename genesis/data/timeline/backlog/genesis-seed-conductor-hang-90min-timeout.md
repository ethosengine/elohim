---
id: "backlog-genesis-seed-conductor-hang-90min-timeout"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "genesis Seed Database hangs ~90min on conductor-path retry when the conductor OOM-flaps — convert to a fast leak-aware SKIP, not a timeout-abort"
slug: "genesis-seed-conductor-hang-90min-timeout"
written: "2026-06-18"
status: "open"
priority: "high"
ci_status: blocked
tags: [genesis, seed-database, conductor-leak, ci-timeout, stampProvenance, circuit-open, leak-aware-preflight]
cites:
  - genesis/scripts/ci/verify-doorway-readiness.sh
  - genesis/docs/superpowers/plans/2026-06-18-genesis-seed-stabilization-postleakfix-plan.md
  - HANDOFF-2026-06-18-conductor-leak-rca-reopened.md
relatedNodeIds:
  - backlog-resilience-card-self-cid-provide-loop-gate
---

# genesis Seed Database hangs ~90min on a leak-flapping conductor → timeout-ABORT (should be a fast SKIP)

Observed on **elohim-genesis/dev #1173** (the 2026-06-18 feat→dev integration run): the build **ABORTED at
90m22s** (SIGTERM, exit 143 — pipeline 90-min timeout). Timeline:
- `matthew` seeded fine (~12s).
- `adam` seed entered an extended **conductor-path retry loop** (~4.8m+ when killed): `stampProvenance: reach
  circuit OPEN after 5 consecutive conductor-path failures` → all 3,429 content rows stamped provenance-only
  (no DHT anchors — the bulk-seed anchor gap) — then the whole pipeline ran out the 90-min clock and was killed.
- **Every downstream stage SKIPPED:** Verify Seeding, Seed Substrate, Seed Custody Commitments, Seed REA
  Commitments, Verify Substrate Propagation, **Verify Delivery Events**, **Verify Projection Sync**, Verify
  Federation/Resilience, E2E.

**Root cause = the unsolved conductor off-heap leak** (`HANDOFF-2026-06-18-conductor-leak-rca-reopened.md`):
the conductor OOM-flaps mid-seed → conductor-path calls fail → circuit opens → seed retries → 90-min hang.
This is leak-gated; it is NOT fixable in genesis. **But the 90-min hang-then-abort is itself wasteful and
masks signal** — each genesis run burns ~90min producing nothing measurable while a single peer's conductor is
unhealthy.

**The fix (CI-hardening, leak-INDEPENDENT — already listed as "optional hardening" in the
genesis-seed-stabilization plan; #1173 elevates its priority from nice-to-have to load-bearing):** make the
seed/verify preflight **leak-aware** — a canary anchored write or a cell-status assert per peer, so a
connected-but-CellDisabled / circuit-open conductor converts that peer's seed from a **90-min retry-hang** into
a **fast honest SKIP** (`verify-doorway-readiness.sh`). Honest skip, not false green; the downstream verify
stages then report SKIP (env-blocked) instead of being silently dropped after a timeout-abort. Saves ~90min
per run and makes "the seed couldn't run because the conductor is unhealthy" legible at minute 1, not minute 90.

**Separately discovered (non-blocking, not the integrating change):** the `Generate Schema Types` stage logged
`Error opening file "epr:schema:manifest:tending-policy-floor"` (line 555) — an EPR schema-manifest reference
that doesn't resolve. The pipeline continued (soft error). Owner: the EPR/tending-policy schema track — flag to
reconcile the `tending-policy-floor` manifest id.

**Until the leak is cured, genesis cannot complete a seed → the seeder stages + the downstream verify stages
(incl. the serve-blob / projection-sync fixes that landed on dev d433d085c) cannot be measured on alpha.** The
clean measurement needs a genesis re-run AFTER the conductor leak fix lands.
