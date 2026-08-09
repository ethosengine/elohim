---
id: content-gap-limit-cycle-sprint-handoff
title: Content-Gap Limit-Cycle Sprint — break the oscillation, land convergence, harvest the waiting greens
status: Draft
class: protocol-canonical
topic: [dataplane, projection-reconcile, convergence, content-gap, limit-cycle, notary-authority]
domain: D5
sprint: content-gap-limit-cycle
cites:
  - genesis/data/timeline/backlog/content-gap-limit-cycle-blocks-convergence.md
  - genesis/data/timeline/backlog/shem-relay-dns-iroh-bypasses-hosts.md
  - genesis/data/timeline/backlog/fleet-quiesce-pass-not-convergence.md
  - genesis/manifests/habits.yaml
---

# Content-Gap Limit-Cycle Sprint Handoff

**Context (evidence, 2026-08-09):** the convergence-serve-path shift cured the
transport seam five layers deep (serve 503s; authority-before-shed in storage
ab316cad7 + doorway 85a128997; honest A-QUIESCED gate banner; node-local-dns
hairpin e4cb2a2 after proving iroh ignores /etc/hosts; the cross-relay
preflight fail-closed defect, fork e4a1c9bb2 vendored kitsune2_transport_iroh
patch, shipped via the relay-fallback conductor image). Post-#1332 proof: both
relay error classes at ZERO fleet-wide. Then the settle-clock delivered its
verdict: with the transport fully open, per-pod
`elohim_projection_reconcile_divergent{stream="content"}` OSCILLATES in
bounded bands for 6h with no drain (matthew 1622↔1858, adam 451↔1750, susan
0↔3129 — touches zero, re-spikes). The content-gap limit cycle is now the
SOLE gate in front of: fleet caughtUp, the doorway shed lifting,
@concern:notary-authority 3/3 (top red habit, active), saga ch04/06/10, and
the head-plane §6.4 fork-deploy decision (input recorded as: quiesce window
unmeasurable until this lands).

**Objective (one sentence):** break the content-plane limit cycle so per-pod
content divergence drains monotonically to tolerance and caughtUp=true
sustains fleet-wide — measured by the same judge as the last shift:
`@concern:notary-authority` 3/3 in a fresh edge Dataplane Validation run
(which now can only pass through real convergence).

## Tasks

| ID | Task | Tier / owner | Gate |
|----|------|--------------|------|
| G1 | **Instrument before optimizing.** The oscillation signature (fixed recurring values; susan 0↔3129) says the divergence gauge may be per-sweep-subset, not fleet-state — verify what the gauge MEASURES (projection_reconcile.rs sweep scope, gauge write points) before trusting any drain number. If the gauge aliases, fix the instrument first (a monotone-state gauge or a per-sweep-labeled family). | Sonnet verify → Opus decide | The corrected gauge's semantics documented at the metric write site; measure-before-ranking law |
| G2 | **Root-cause the cycle.** Why do heals not stick / gaps not close: 120s leg budget vs ~2.9k pending (the known plateau arithmetic), fills reverting, re-divergence, or subset re-measurement. The 6h Prometheus series + fresh Loki sweeps are the evidence base; the backlog item names the design direction: **F-B fan-out + peer-probe source widening** — now with a LIVE full mesh to widen across (first time the fix has a working transport under it). | rust-architect (Opus) | Reproduce the cycle's mechanism with quoted evidence before writing the fix |
| G3 | **Land the drain.** Implement the fan-out/source-widening (or whatever G2 proves), with pacing bounded by the adam write-guard saturation watch-out — a widened fan-out over a newly-opened full mesh is exactly the shape that melted adam on 2026-07-20. AIMD-style pacing exists in the head-plane batch seam; compose, don't reinvent. | rust-architect + Sonnet legwork | Per-pod divergent{content} monotone → tolerance in Prometheus over a real window; caughtUp=true sustained |
| G4 | **Harvest the waiting greens.** After G3: one [build:edge] wave → gate measures (honest banner), suite runs → notary 3/3 flips the habit with build evidence; saga ch04/06/10 recover; peer-mesh divergent-anchor scenario passes; F2's CLEAN quiesce window gets measured by the fixed gate and feeds §6.4. | Orchestrator | Stability 2/2 across fresh triggers, per the shift discipline |
| G5 | **Residual watch items** (fold in only if they block G4): doorway-A ingress readiness 503 (pod alive, ingress pulling it); A/B doorway federation mutual-reach (islands class); genesis pipeline dispatch coverage (docs pushes may be under-dispatching — verify against the walker). | Sonnet as needed | Each either cleared or re-filed with evidence |

## Watch-outs (born-warned)

- **Sampling artifacts bite here twice in one day**: never read convergence
  from the pool-fanned /health endpoint; per-pod Prometheus only.
- **Vacuous greens**: the REA oracle's `--ignored` trap and the overloaded
  error-string class are both this seam's local history — grep the
  `N passed` line, and treat any error message as a claim to verify, not a
  diagnosis.
- **Write-guard saturation** (adam, 2026-07-20): pacing is part of G3's
  correctness, not a tuning afterthought.
- **p2p-design-gate** is mandatory if G3 adds entities/messages.

## Ceiling (operator)

None known at kickoff — the limit cycle is repo-side dataplane work end to
end. (The Jenkins API token 302s if a parameterized job run is ever needed;
the edgenode image path is already settled from the last sprint.)

## First wave suggestion

{G1} then {G2} sequentially (G2 needs G1's instrument verdict); G5's
genesis-dispatch check can run parallel to either. Same contract as the
prior sprints: tiered legwork, commit-only workers, orchestrator reviews
every diff, one push per batch, evidence-gated pushes.
