# Backlog: Break the 503 — operator-gated levers (staged proposals)

**Status:** staged, awaiting operator approval · **Captured:** 2026-06-16 (shift `alpha-conductor-oom-arc-leecher`) · **Class:** runtime availability / capacity · **Why operator-gated:** capacity + live-ingress decisions only the operator can confirm (CLAUDE.md: cluster ops are operator-owned).

## Why these are the levers
The autonomous lever (arc-shrink) is **falsified** — `target_arc_factor=0` is honored but does NOT bound conductor memory (jessica soak: confirmed leecher, still OOMs to 4Gi every ~40 min; see `arc-shrink-ineffective-memory-soak.md`). matthew is the genesis anchor (cannot leecher) and its doorway is at **100 restarts** — the 503 root. The durable levers below are all operator-side. Apply in order; #3 is the fast availability stopgap that unblocks Genesis immediately.

---

## Lever 1 — matthew RAM 8Gi → 16Gi  ⚠ CAPACITY-GATED (apply ONLY after confirming node headroom)
**File:** `genesis/orchestrator/data/deployments.json` — matthew entry.
**Diff:**
```
-      "edgenodeMemoryRequest": "2Gi",
-      "edgenodeMemoryLimit": "8Gi",
+      "edgenodeMemoryRequest": "4Gi",
+      "edgenodeMemoryLimit": "16Gi",
```
**GATE (do not skip):** `ethosengine` already hosts matthew(8Gi)+james(8Gi)+jessica(4Gi)+doorway. **Confirm the node's allocatable RAM has headroom for a 16Gi matthew before applying** — a limit above node capacity makes matthew **Pending → alpha fully down** (worse than the 503). Verify node allocatable (operator: `kubectl describe node ethosengine` / capacity dashboard). If headroom is tight, do **Lever 2 (spread) first** so matthew isn't competing with james on the same box, then 16Gi fits.
**Rationale:** matthew = genesis anchor, can't leecher; arc lever dead; RAM is the honest lever (RCA theory 13: size the anchor for the job). matthew's working set sawtooths to its cgroup ceiling ~every 3h; 16Gi roughly doubles the OOM interval — a stopgap, not a fix (the real fix is the §4 memory instrumentation to find the actual driver, per `arc-shrink-ineffective-memory-soak.md`).

## Lever 2 — podAntiAffinity: spread matthew + james onto distinct nodes (RCA's better-supported fix)
Relieves BOTH the conductor OOM-crowding AND the doorway watchdog co-location park (RCA theory 3: matthew's doorway is killed because it's co-located with the two memory-climbing conductors; adam's doorway on a different node has 0 restarts with identical config).
**File:** `genesis/orchestrator/manifests/humans/_edgenode-consolidated.template.yaml` (the shared StatefulSet template) — add to the pod spec:
```yaml
      affinity:
        podAntiAffinity:
          requiredDuringSchedulingIgnoredDuringExecution:
            - labelSelector:
                matchExpressions:
                  - key: elohim-human
                    operator: In
                    values: ["matthew-manager", "james-son"]
              topologyKey: kubernetes.io/hostname
```
(Reconciled by the edge pipeline; never `kubectl` directly. Verify the pod label key matches the template's existing `elohim-human` label.) This also makes Lever 1's 16Gi fit (matthew alone on its node).

## Lever 3 — route alpha → adam/apex edge (FAST availability stopgap, unblocks Genesis NOW)
adam's doorway is **0 restarts, healthy**; apex/`elohim.host` is up. Routing `alpha.elohim.host` → adam's edge makes alpha reachable immediately while matthew is worked, which **unblocks the genesis pipeline** (it fails on `Verify Target Health` against the 503 alpha) and lets a seeder run light the resilience card.
**Surface:** ingress (operator-owned — the repo is not the live-ingress cleanup surface per CLAUDE.md). Document/apply the alpha ingress host rule to point at the adam edge service. **Caveat:** matthew↔adam island by construction (A/B edge islanding) and cohere only via DHT gossip — confirm adam's edge serves the alpha namespace's content (or accept apex-content during the stopgap).

---

## Sequencing
1. **Lever 3** (route→adam) — immediate availability + unblocks Genesis; lowest risk.
2. **Lever 2** (anti-affinity) — spread the anchors; relieves doorway park + makes RAM fit.
3. **Lever 1** (RAM 16Gi) — only after Lever 2 (or confirmed node headroom).

## After any lever lands → light the card
Once alpha is reachable: run the seeder (`seed-provide-rows.ts` heals `humans.agent_pub_key` + seeds `uhCAk` provide rows) → resilience card lights (`commitment_backed_collectives ≥ 1`, `stewarding_collectives ≥ 1`). The CID enforcement stack (commits `3d026f226`/`860c9e96b`/`8c217137c`, pushed to dev this shift) makes runtime provide rows join-correct too.

## Links
- Arc falsification + real memory lever: `arc-shrink-ineffective-memory-soak.md`
- CID enforcement rollout: `cid-enforcement-rollout.md`
- RCA (theories 3/9/13, staged experiments): `.claude/data/matthew-edge-resiliency-rca-fanout-2026-06-15.md`
- Shift journal: `.claude/shifts/2026-06-16T0357-alpha-conductor-oom-arc-leecher.journal.md`
