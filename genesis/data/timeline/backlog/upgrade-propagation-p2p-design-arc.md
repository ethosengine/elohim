---
id: "backlog-upgrade-propagation-p2p-design-arc"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Design arc (operator course-set 2026-08-31): p2p hApp upgrade/revert propagation — mixed-version peers keep communicating, no big-bang fleet rolls; the crux before inviting app developers onto the SDK"
slug: "upgrade-propagation-p2p-design-arc"
written: "2026-08-31"
author: "shift 2026-08-31T02-40-fleet-carried-election-convergence"
status: "open"
priority: "high"
jobs: [elohim-edge, elohim-holochain]
cluster: "arch-dataplane-refactor-backlog"
relatedNodeIds:
  - "backlog-late-joiner-peer-discovery-boot-only-board"
  - "backlog-mesh-fixture-fidelity-regimes"
  - "habit:dataplane-convergence"
tags: [upgrade, rollback, dna-lineage, sdk, dataplane, brainstorm-class]
---

Operator course-set (2026-08-31, verbatim essence): *"Full support for upgrade
propagation revert/upgrade over p2p is another engineering feat to achieve
before I'd feel confident for people to start trying to use it. Holochain
encrypts data to ONE hApp bundle — this seam cannot be upgraded by any
external push; the network itself must agree on how it evolves."* The
multi-hour roll-per-change dev cycle is the wall-clock this arc structurally
retires.

Existing inventory (all proven live 2026-08-31, the carried-election shift):
- **Coordinator hot-swap over one admin call** — running conductor, no
  re-key, no reboot (admin `update_coordinators`, applied to W2 at 14:00Z).
- **Mixed-version wire discipline** — additive serde(default) fields with
  byte-identity pins; old and new peers conversed through a rolling window.
- **Same-hash happ splice** — old integrity + new coordinator wasm repacked,
  DNA hash preserved (manual instance of the version-lineage crossing).
- **Elected canonical heads carried peer-to-peer with local wasm
  verification** — structurally the mechanism for network-agreed evolution:
  "which bundle is canonical" is a head election over an artifact the storage
  plane already replicates.
- The genuinely hard remainder: DATA REBINDING across DNA lineage
  (HC 0.6 gates `lineage:` behind unstable-migration; we own the conductor
  fork). Read `2026-06-11-dna-upgrade-governance.md` first.

## The velocity ladder — a DEBT SNOWBALL (operator-set 2026-08-31)

**The ordering rule is a debt snowball, and the direction is deliberate:
smallest atomic-discipline debts are paid FIRST; the most complex,
highest-risk change — full upgrade/revert propagation over p2p, and beneath
it DNA lineage — is deliberately LAST.** It carries the highest cost of
proof (a LOT of CI/CD cycles to shake out), so every rung paid off before it
raises iterations-per-day and therefore the odds of landing it. The snowball
IS the launch program: atomic disciplines are how the moon landing survives
contact.

**Cycle-time baseline (measured 2026-08-31 — RE-MEASURE after each rung
lands and record the delta in sprint planning; cycle-time is this arc's own
measure):**

| Change class | Cost per iteration TODAY | Anatomy |
|---|---|---|
| content / app EPR | ~minutes | already atomic (author→declare→converge) |
| coordinator zome | **~2-4 h** | bundled into the pod-image roll (the mechanism itself is minutes) |
| config flip | **~2-4 h** | env read once at boot ⇒ full roll |
| storage/doorway binary | **~2-4 h** | roll 30-65 min + 1-3 h conductor catch-up before measurements are trustworthy |
| DNA / integrity | roll + governance | rare by design |

**Structural insight (2026-08-31):** pod recreation restarts the conductor
regardless of container layout — the catch-up tax dies only when the
conductor has its OWN lifecycle. The external-conductor mode that does this
ALREADY EXISTS (`--admin-url`; the workspace peer W2 ran a detached
conductor through the whole arc — storage restarted 4x, zero conductor
churn).

**The snowball, smallest debt first:**

| # | Payment | Effort | What it frees | Snowball effect |
|---|---------|--------|---------------|-----------------|
| 1 | **Fleet coordinator hot-swap vehicle** — a script/CI step calling admin `update_coordinators` across the pods (done by hand on W2 in one call, 2026-08-31). No roll, no re-key, no churn. | hours | zome-logic iterations: 2-4 h → minutes — the highest-frequency change class in dataplane work | every later rung that touches wasm iterates for free |
| 2 | **Split the conductor into its own workload** — separate StatefulSet; storage attaches via the proven `--admin-url` mode. Storage/doorway rolls stop touching conductors: no arc reset, no catch-up regime. | days (manifest surgery + one careful migration roll) | storage/doorway iterations: 2-4 h → ~40-70 min; also deletes the fleet-wide no-authorities regime | every later rung's native-code iterations |
| 3 | **Staggered rolls** for the remaining conductor-touching cases (fold in backlog `staggered-conductor-fleet-restarts`). | ~day (deploy-loop ordering) | conductor rolls: fleet-wide 2.5-3 h churn → bounded per-peer windows | keeps later-rung experiments measurable while they run |
| 4 | **Config as runtime surface** — watched config first; protocol-native form later (config as declared EPRs: a flag flip = a head declaration that converges). | days | flag flips: 2-4 h → seconds; unblocks operator-runtime-surface verbs (habit #2) | later adoption switches become runtime acts, not deploys |
| 5 | **THE MOON SHOT (deliberately last): artifacts as ELECTED CONTENT + upgrade/revert propagation over p2p** — a content-addressed artifact with an earned canonical head; peers verify locally and adopt at their own pace; mixed versions ride the additive-wire discipline; revert = the election moving back. The carried-election machinery is the proven kernel. Attempted ONLY on top of rungs 1-4's velocity — it is the change that needs the most CI/CD proof, so it gets the cheapest possible iterations. | design arc (/brainstorm, p2p-design-gate) | retires the CI roll as the delivery path for everything above the DNA line; the SDK confidence bar | — |
| 6 | **DNA lineage migration** — the irreducible network-agreement seam under the moon shot (hash = network identity; data rebinds across lineage). Rare, deliberate, constitutional. | the hard kernel | the last big-bang class, governed | — |

**Rung 1 LANDED 2026-08-31 (local-mesh proven: upgrade → revert → upgrade,
3 peers, ~40s/peer/pass, conductor PIDs unchanged throughout).** Vehicle =
`POST /admin/coordinators/sync` (per-role DNA-lineage guard; embedded AND
external conductors — also cures the T3 external-attach hotswap gap) +
`scripts/ci/fleet-coordswap.sh` (rolling driver) + a warn-only DNA-pipeline
stage. **Trajectory note (operator concern, 2026-08-31): the CI push and the
service-DNS roster are the k8s-shaped SCAFFOLD half of this rung — clean
toward the dataplane, never extend. The server-side machinery (peer applies
to its own conductor, lineage-guarded, verified) is the durable half rung 5
re-drives from an elected bundle head: push inverts to pull, roster inverts
to election, and delivery moves THROUGH the p2p network.** Cycle-time
delta to record at next sprint planning: coordinator class 2-4h → ~2min
(mesh, measured); fleet expected minutes after the one edge roll that ships
the endpoint.

Cross-cutting lesson (2026-08-31): even atomic changes paid big-bang prices
because everything ships in ONE vehicle (the pod image). **Separate the
delivery vehicle by change class** — and pay the small vehicles off first.

Brainstorm-class: route through /brainstorm (p2p-design-gate applies —
upgrade artifacts, lineage records, and adoption elections are data
entities). Rung 1 may be pulled forward as bounded shift work ahead of the
full design arc. Not to be ground as unplanned shift iterations.
