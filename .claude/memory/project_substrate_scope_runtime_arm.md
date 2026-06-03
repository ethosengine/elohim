---
id: project-substrate-scope-runtime-arm
name: project_substrate_scope_runtime_arm
description: "The substrate-scope reconciler is a cybernetic loop with TWO arms over one @requires:<cap> vocabulary — planning (held/ move) + runtime (a2o Before-hook skip); generic over any cluster-state cap, not shem-specific"
metadata:
  node_type: memory
  type: project
  originSessionId: 1c4e3007-6490-4eaf-b8ef-9d240eaafa58
cites:
  - genesis/a2o/src/framework/fixtures/substrate-scope.ts
  - genesis/a2o/steps/common.steps.ts
  - genesis/manifests/cluster-state.yaml
  - .claude/scripts/memory-kit/scope-reconcile.py
---

The substrate-scope mechanism is a **cybernetic reconciliation loop over the agentic-memory corpus**, and it has **two arms driven by one vocabulary** (`@requires:<cap>` where `cap` ∈ `cluster-state.yaml` resources). `cluster-state.yaml` is the SENSOR (declared substrate reality), the tag is the SETPOINT, the mover + the runtime gate are the actuators, the SessionStart `scope:` gate is the feedback. **shem is just one cap** — the whole machine is generic over `alpha-cluster-6peer`, `harbor-registry`, a future `iroh`, anything declared.

**Two arms (added the runtime arm 2026-06-03):**
- **Planning arm** (pre-existing, `scope-reconcile.py`): a FEATURE-level `@requires:<cap>` → when `<cap>` is unavailable the whole `.feature` is `git mv`'d to `held/`, out of the cucumber glob AND out of agentic-search/planning scope. See [[project_gap_granular_substrate_scope]].
- **Runtime arm** (new): a SCENARIO-level `@requires:<cap>` on a MIXED feature → a `Before` hook in `genesis/a2o/steps/common.steps.ts` returns `'skipped'` (cucumber 11) when any required cap is unavailable. The feature stays live; only its capability-bound scenarios are held. **This closed the seam** (handoff defect #4) where a shem scenario that didn't happen to name a remote-only persona ran against down pods and FAILED, masking the real signal.

**The primitive:** `genesis/a2o/src/framework/fixtures/substrate-scope.ts` — cap-generic over cluster-state: precedence is `env override (ELOHIM_REMOTE_COMPUTE_STATUS for shem; ELOHIM_CAP_<CAP>_STATUS generic) → durable home (only \`available: true\` counts) → fail-open ONLY when the cap is undeclared/unreadable`. Caps NOT in cluster-state (`@requires:doorway`, `@requires:seeded-content`) are fixture preconditions, never substrate gates. `humans.ts`'s shem-specific RemoteCompute API now delegates its parse to this module (one parser for all caps).

**`shem` ≠ `multi-node`** (the precision discipline): the household (matthew/jessica/james) is itself a 3-node cluster, so a cross-node scenario among them is household-testable — do NOT hold it. Only the remote multi-tenant canvas (the 11 `nodeTypes:['remote']` personas, or >3 independent peers) needs `shem`. First exercise classified 91 a2o features → 13 HOLD_WHOLE (feature-tag → held), 13 MIXED (scenario-tag → runtime-skip), 65 stay live.

**CI twin:** `genesis/Jenkinsfile probeRemotePoolStatus()` now reconciles a blind kubectl probe to `cluster-state.yaml` instead of failing OPEN; the seeder's genesis set drops remote-only peers (adam) when remote compute is down (see [[project_alpha_topology_bootstrap_pair]]). The three homes — cluster-state.yaml, `ELOHIM_REMOTE_COMPUTE_STATUS`, the held/ tree — cannot disagree ([[project_ci_reconciles_to_substrate_signal]]).

**Why:** so an agentic developer can `--set <cap>=off`, drive a focused shift on the verifiable household subset without bailing on in-scope artifacts or failing on out-of-scope ones, then `--set <cap>=on` and resume with the enabled context intact. Stories + architecture follow the substrate.

**How to apply:** to focus the plate on any dependency point, declare it in `cluster-state.yaml`, tag the dependent features/scenarios `@requires:<cap>`, and toggle with `scope-reconcile.py --set <cap>=off|on --apply`. **Footgun:** `--set` WITHOUT `--apply` still writes the durable home (only the *move* is dry-run) — a "preview" flip leaves cluster-state changed; always pair the flip you intend with `--apply`, and re-`--set <cap>=off --apply` to restore if you only meant to preview. Relates to [[feedback_shem_down_peers_are_held_not_failed]]. Spec §9: `genesis/docs/superpowers/specs/2026-06-02-scope-tree-reconciler-design.md`.
