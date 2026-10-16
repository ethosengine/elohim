---
id: "backlog-doorway-hosted-capacity-bound-from-pod-memory"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Hosted capacity is a declared count of 50 against a conductor that holds about five — the bound, the tie-break and the orphaned install"
slug: "doorway-hosted-capacity-bound-from-pod-memory"
written: "2026-10-16"
author: "claude-opus-5-5 (operator-directed cataloguing pass)"
status: "backlog"
priority: "high"
relatedNodeIds:
  - "habit:doorway-failover"
  - "backlog:doorway-registry-prune-matches-app-id-only"
  - "backlog:arch-scale-risk-backlog"
cites:
  - genesis/data/timeline/CONVENTIONS.md
  - genesis/data/timeline/backlog/doorway-registry-prune-matches-app-id-only.md
  - genesis/data/timeline/backlog/arch-scale-risk-backlog.md
  - doorway/doorway-service/src/conductor/pool_map.rs
  - doorway/doorway-service/src/conductor/registry.rs
  - doorway/doorway-service/src/routes/auth_routes.rs
  - genesis/orchestrator/manifests/doorway/alpha-b.yaml
shift_objective: |
  Make a hosting conductor's admission bound a number the pod can actually carry. Derive the
  per-conductor hosted-agent bound from the conductor's memory limit (or declare it per human in
  deployments.json beside edgenodeMemoryLimit), break placement ties deterministically, and
  uninstall the app when create_human fails so a failed registration neither holds heap nor skews
  the count. Done when three consecutive genesis runs register their hosted humans on alpha without
  a create_human timeout or a conductor OOMKill, and the registry's capacity_used equals the
  conductor's installed-app count after each run.
tags: [doorway, conductor, hosted, capacity, alpha, operator-decision, risk, performance, perf-memory, perf-scale, perf-queue, trustful-self, friction-mechanical, custody-everyone, plane-custody, unit-agent, phase-steady, phase-growth, lane-interactive]
---

# Hosted capacity is a declared count against a heap that cannot carry it

Found by genesis #1649 (2026-10-16, against 89c41e696): all three hosted sign-up scenarios failed
with `create_human` timing out at 10 s on conductor-0 (adam). The registry prune
([doorway-registry-prune-matches-app-id-only](epr:doorway-registry-prune-matches-app-id-only),
"the third fact") fixed the counts and so exposed three defects underneath. This entry holds them
as one performance concern because they compound into a single law: **a hosting conductor reaches
its memory limit about every second genesis run.**

## What the code and the fleet say (read 2026-10-16)

**1. The bound is a declaration with no derivation.** `DEFAULT_MAX_AGENTS_PER_CONDUCTOR = 50`
(`pool_map.rs:45`). Alpha A's doorway leaves `DOORWAY_MAX_AGENTS_PER_CONDUCTOR` unset, so it uses
50. Alpha B sets 32 (`manifests/doorway/alpha-b.yaml:395`). Nothing reads the pod's memory.

**2. The pod's memory is RAM, and it holds about five.** `deployments.json` gives adam and matthew
`edgenodeMemoryLimit: 8Gi`. The edge Jenkinsfile splits that budget 5/8 to the conductor and 3/8 to
storage (`elohim/holochain/Jenkinsfile` ~1488–1505, `scripts/ci/conductor-split-budget.sh`), so the
conductor container runs at 5120 Mi (`manifests/humans/adam-firstman-conductor.yaml:397`). A
conductor idles at about 1–1.5 GiB. Each hosted install adds about 0.7 GiB that is not released
until the conductor restarts ([arch-scale-risk](epr:arch-scale-risk-backlog) row 7, measured by
smaps 2026-10-08: ~650–765 MB per hosted human). adam went 1.17 → 3.18 GiB for three installs;
matthew's conductor was OOMKilled at the limit on 2026-10-15 19:52:53Z (the genesis #1648
`ConnectionClosed`). Arithmetic: (5.0 − 1.5) / 0.7 ≈ 5 hosted humans per conductor, not 50.

**3. Ties go to whichever entry the hash map yields first.** `find_least_loaded_excluding`
(`registry.rs:361–365`) takes `max_by_key(capacity_max − capacity_used)` over a DashMap. After the
prune both hosting conductors seed at 14/50, and the iteration order picked adam, the pod already
logging 96–220 `get_links … response channel dropped` and read-permit timeouts an hour.

**4. A failed `create_human` leaves its app installed and uncounted.** The provisioner uninstalls
on its own failures: enable, genesis timeout, registration (`provisioner.rs` ~263, ~324, ~354).
The hosted registration branch then calls `create_human` with the default 10 s zome-call deadline
(`zome_caller.rs:99`, `DOORWAY_ZOME_CALL_TIMEOUT_MS`). On failure it returns 503
`IDENTITY_CREATION_FAILED` (`auth_routes.rs` ~1745) without deprovisioning. The installed cells
keep their heap, and the registry's count does not include them, so the tie persists into the next
registration.

## Why it compounds

The four combine into one loop. A declared 50 admits registrations. Ties send them to the
saturated pod. The saturated pod times out `create_human`. The failed install keeps its 0.7 GiB.
The count does not move, so the next registration goes to the same pod, until it OOMs and restarts.
A restart then costs roughly 20 minutes of churn on the fleet (substrate trust contract) and up to
hours of space rejoin.

## The fix shape

Smallest first:

- **Uninstall on `create_human` failure.** Call `provisioner.deprovision_agent` in the error arm,
  with the same idempotency note the provisioner already carries (an install whose socket died may
  have landed). Bounded doorway change, unit-testable. Check first against the unmerged branch
  `fix/hosted-provisioning-stop-damage` (2026-10-04, five commits, about 10k lines across
  `auth_routes.rs`, `zome_caller.rs` and a new `human_binding.rs`): it rewrites this registration
  path around an intent log and a has-a-Human check, and already deprovisions in one recovery arm.
  It is 12 days old and not on dev. Land the cleanup there or reconcile with it, not beside it.
- **Deterministic tie-break.** Among equal headroom, prefer the conductor with lower observed
  pressure (admission occupancy or RSS from conductor-diagnostics), falling back to a stable
  order by id. A pod already shedding should never win a tie.
- **A derived bound.** Either compute `floor((conductor_limit − idle_baseline) / per_install)` at
  render time from the same split arithmetic, or declare `hostedAgentMax` per human in
  `deployments.json` and render it into the doorway env. The second keeps the number in the
  portfolio where the capacity check can see it. The per-install figure is the row 7 measurement
  and should be re-read once the module-cache lead
  ([arch-scale-risk](epr:arch-scale-risk-backlog) row 14) is tested, because it may fall by a
  quarter.
- **Count installs, not agents.** The registry already re-seeds from distinct installs
  (`registry.rs:284–296`). The bound should be compared against that, including failed installs
  until they are removed.

## Needs the operator

- **The number.** About 5–6 per 5 GiB conductor today. Raising `edgenodeMemoryLimit` and letting the
  split follow raises the ceiling, but installs still never release memory, so the bound still has
  to be real.
- **Whether adam keeps hosting.** adam is the saturated genesis-pair anchor. Setting its bound to 0
  leaves matthew as the only hosting conductor on alpha A.

## Done when

Three consecutive genesis runs register their hosted humans on alpha with no `create_human`
timeout and no conductor OOMKill, and after each run the registry's `capacity_used` for each
hosting conductor equals its installed hosted-app count (`GET /admin/conductors/conductor-N/agents`
against the conductor's `list_apps`).

The longer cure is to stop paying 0.7 GiB per registered human at all. That is the design question
in [arch-scale-risk](epr:arch-scale-risk-backlog) rows 14–17 (what a hosted human's conductor must
actually run). This entry is the guard rail until then.
