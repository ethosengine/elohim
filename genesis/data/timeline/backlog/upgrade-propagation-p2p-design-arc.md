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

Operator course-set (2026-09-01, steering the rung-5 design): **upgrade
stewardship is a domain of the NETWORK itself.** Networks and protocols are by
nature monopoly power, and that apex power is deliberately vested one degree
removed from humans — in the elohim, at the constitutional level. The
convenience-and-security power of automatic updates (the Chrome-OTA class) is
wielded THERE to keep the whole coherent and compatible: peers may diverge in
DECLARED patterns with reconciliation pathways and maps back (enabling the
global orchestra), but silent staleness is a harm class, not a freedom — the
household member who never updates is exposed, not sovereign. Consent is
exercised through the governance system (meaningfully heard, decisions
explained, intimate context weighed with the whole context in good faith),
NOT as a per-node veto on staying current. The protocol stewards its own
protocol: a values-first non-negotiable of the values-first contract.

And the complement (same course-set, second breath): compatible branches,
low-reach runtime experiments, and A/B variants are NOT a deficit conceded —
they are the viable-system-model ecology exercising itself. Robustness
requires necessary diversity between peers; what works informs the whole,
flowing up through hubs and back down to regions for what fits particular
contexts. Variety lives ABOVE the compatibility envelope; unity is enforced
AT it. Day one needs only the MVP architecture — enough to accelerate our own
development and demonstrate on the test fleet how the future will be grown,
in love — not the complete ecology.

And the closing breath (operator, 2026-09-01, "Go"): the unity is not
arbitrary — because we are all created in the imago dei, we all share
something, so we all have to agree SOMEWHERE. The compatibility boundary —
how robustly it can support, extend, and afford diversity, and bring
reconciliation back, as exercised by its own evolution for good or ill — is
where this system closes. This decision was deliberately pushed as late as
it has been; it is the right and only approach we can possibly support.

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

**Rung 2 IMPLEMENTED IN-TREE 2026-08-31 (repo-only; CI-unverified until the
operator's next edge pipeline reconciles it).** Every human is now TWO
StatefulSets sharing one image artifact: `<resourcePrefix>` (elohim-storage in
external-conductor mode) and `<resourcePrefix>-conductor` (the same binary in
embedded-conductor mode, storage features off, plus a socat ws-proxy sidecar).

The five load-bearing decisions, so a later reader can re-derive them:

1. **PVC continuity via explicit `claimName`, no renames.** The storage
   StatefulSet KEEPS its name — its Service DNS, `storage-data` PVC, doorway
   config and coordswap roster are all byte-identical. The conductor
   StatefulSet has NO `volumeClaimTemplates`; it mounts the already-minted
   `holochain-data-<prefix>-0` PVC by explicit `persistentVolumeClaim.claimName`.
   That PVC is Retained and its openebs-hostpath PV carries node affinity, so
   the conductor pod is co-scheduled onto the node that already holds the
   bytes. Zero re-genesis (agent keys survive), zero reseed, zero DNS churn.
   Retiring the storage STS's `holochain-data` template is a Forbidden
   StatefulSet spec update — handled by the orphan-delete + reapply fallback
   that already lives in `deployHumanManifest` and retains PVCs by name.

2. **One image, two pin cadences.** Storage keeps `STORAGE_IMAGE_PLACEHOLDER`
   (moves every edge build). The conductor gets its own
   `CONDUCTOR_WORKLOAD_IMAGE_PLACEHOLDER`, resolved by
   `resolveConductorWorkloadImage` to the image the LIVE conductor StatefulSet
   already runs — the cluster is the durable record, and an absent StatefulSet
   is the first-rollout signal. It advances only on a conductor/tx5 submodule
   bump (`scripts/ci/conductor-workload-pin.sh`, the same derivation
   `build-storage-image.sh` uses), an operator `[conductor-roll]` /
   `CONDUCTOR_ROLL`, or a hApp digest move (which flows through the conductor
   pod template's `elohim.host/happ-digest` annotation). Without this the split
   would be cosmetic: every storage roll would still roll conductors.
   Corollary, enforced by construction: NOTHING that changes on an ordinary
   commit may appear in the conductor pod template — `app.kubernetes.io/version`
   is on object metadata only, and the recorded pin is an object annotation.

3. **BUDGET NEUTRALITY — the split re-partitions, it never adds.** The first
   cut gave each pod the full `edgenode*` budget and the pre-commit
   `epr:validator-test-bench-aggregate-capacity` refused it. That refusal was
   right for a reason worth recording: the validator sums
   `edgenode{Cpu,Memory}{Request,Limit}` across active humans and knows nothing
   about per-pod fields, so two pods each carrying the full budget would have
   DOUBLED the portfolio's real footprint while the portfolio arithmetic showed
   no change at all. `edgenode*` now means "this human's TOTAL envelope,
   partitioned across two pods": `conductorShare`/`storageShare` in the
   Jenkinsfile split it (memory 5/8 conductor — RSS ∝ corpus at full arc,
   ~2.5 GB observed on alpha, vs a few hundred MB for storage alone; CPU 1/2 —
   no measurement yet discriminates the sides, so the neutral halving is the
   honest default), rounding DOWN with any remainder falling to the storage
   side, and `validate-deployments.ts` check (3) asserts the two sides sum back
   to the record. **Re-size only with measured evidence PER SIDE, and do it by
   moving `edgenode*` — the field the portfolio validator actually holds to
   account — never by inflating one pod's share behind the ledger's back.**
   Aggregate requests/limits are byte-identical to pre-split (active humans:
   requests 4250m / 9472Mi, limits 25000m / 37888Mi), so the standing
   `$computeEnvelopeRatification` covers the portfolio unchanged.

4. **Ordering is the single-writer guarantee.** Per human:
   storage apply → rollout → Ready (this is what kills the embedded conductor
   and releases the chain PVC), THEN conductor apply → rollout status. The
   conductor path never issues `kubectl rollout restart` — a steady-state
   deploy reports it `unchanged` and restarts nothing.

5. **Rollback is symmetric and data-free.** Delete the `<prefix>-conductor`
   StatefulSets and revert the manifests (storage back to
   `EMBEDDED_CONDUCTOR=true` with its `holochain-data` volumeClaimTemplate).
   That is exactly today's pre-rung-2 shape and it re-adopts the same PVCs by
   the same names. PVCs are untouched in both directions.

Why socat rather than elohim-storage's own forwarder: the Rust forwarder is
only reachable through main.rs's `if let Some(admin_url)` branch, which also
starts the HcClientRegistry, the bridge supervisor, the signal subscribers and
the PeerStatus heartbeat. Giving the conductor pod an `admin_url` would run a
SECOND heartbeat per human, and its `DefaultProbe` measures the disk under its
own blob path — an emptyDir — so it would publish wrong capacity over the
storage pod's real numbers every 60s. The conductor pod therefore has no
`admin_url` at all and uses the legacy ws-proxy shape (still live in
`genesis/orchestrator/manifests/edgenode/alpha.yaml`) on the same 8444/8445
ports the whole fleet already speaks.

**Named preconditions carried INTO the migration roll (deliberately not fixed
here):** (a) the PeerStatus heartbeat's own `HcClient` sits outside the bridge
supervisor and never re-mints its app-interface token, so a conductor restart
under a running storage still flaps `/health conductor.zomePath` — this rung
makes independent conductor restarts routine rather than rare, so that defect
will fire more often (backlog
`storage-stale-app-interface-token-after-conductor-restart`); (b) doorway's
`ServingHealth` does not read storage's `/health/serving`; (c) **adam's conductor read pool halves.** His 8000m single-pod CPU limit was
chosen 2026-07-07 so the conductor detected 8 cores and its SQLite read pool
`max(2*cpus, 8)` reached 16 readers; at a neutral 4000m share it detects 4 and
floors at 8. Restoring 16 is arithmetically incompatible with budget neutrality
(it needs the entire 8000m on one pod), so it is a RAISE of adam's `edgenode*`
envelope — a portfolio decision `test-bench-aggregate-capacity` must ratify, not
something to smuggle in as a `conductorCpuLimit` override. Countervailing
evidence before raising: pre-split the conductor was `nice`d to 10 to LOSE the
CPU race to storage inside the shared cgroup, so its 4000m is now uncontended
where the 8000m never was. Measure first.

Cycle-time delta to record after the first post-split storage-only deploy:
storage/doorway class 2-4 h → the storage rollout window alone, with
`statefulset.apps/<prefix>-conductor unchanged` in the build log as the proof.

**Rungs 2-4 LANDED overnight 2026-09-01 (velocity-rungs shift):**
- **Rung 2 (conductor split)**: applied to alpha on edge #1405 after two
  FAIL-SAFE attempts (CPS sandbox rejected Double.parseDouble then DGM
  toBigDecimal at RUNTIME — arithmetic now lives in
  scripts/ci/conductor-split-budget.sh; museum lesson: pipeline arithmetic
  belongs in shell, and catchError + old-STS retention turned both failures
  into log lines instead of incidents). All 7 <prefix>-conductor STSs
  created, existing holochain-data PVCs adopted by claimName, zero
  re-genesis (both doorways kept serving declared content throughout),
  budget-neutral splits enforced by validate-deployments check (3).
- **Rung 3 (staggered rolls)**: conductor deploys drained sequentially from
  a synchronized queue after ALL storage rollouts reach Ready — genesis
  pair last, matthew final, soak-on-change only (CONDUCTOR_STAGGER_SOAK_SECS).
- **Rung 4 (config as runtime surface, watched half)**: proven live on the
  mesh — flag flip applied to a RUNNING storage node in seconds (same PID
  through activation, two WARN-logged flips, and boot-value restore);
  fleet wiring = per-human runtime-config ConfigMap -> mounted file ->
  in-process watcher; GET /admin/runtime-config reports effective values,
  provenance, and the deliberately boot-only knobs with reasons.

**Rung 5 DESIGN LANDED 2026-09-01** (five-lens adversarially verified):
spec `genesis/docs/superpowers/specs/2026-09-01-runtime-artifacts-elected-content-design.md`
+ the six-task implementation family (`task-release-manifest-schema-packager` ·
`task-release-channel-ceremony-driver` · `task-release-adoption-controller-observe`
· `task-release-apply-vehicles` · `task-release-soak-attestation-rail` ·
`task-runtime-upgrade-a2o-receipt`) — discrete, disjoint, dependency-edged,
claimable. Ladder: local → hybrid → mesh → cluster; end-state names Jenkins an
external observer and retires k8s as the operations plane.

**Cycle-time deltas (record in sprint planning):**

| Change class | Was | Now (measured) |
|---|---|---|
| coordinator zome | 2-4 h | ~2 min mesh (proven x3: upgrade/revert/upgrade + post-refactor rerun); fleet = DNA-pipeline auto-stage (first live run pending) |
| config flip | 2-4 h | SECONDS on a running node (mesh-proven); fleet ≈ push+apply, no roll |
| storage/doorway binary | 2-4 h (incl. conductor churn + catch-up) | roll only storage pods — conductors keep arcs (first storage-only roll measuring on edge #1406) |
| conductor roll | fleet-wide 2.5-3 h churn | staggered per-peer windows, genesis anchor last |
| coordinator zome — **rung 5, elected (mesh receipt 2026-09-02)** | 2-4 h | publish→3/3 staged ≤19 s · canary hot-swap ~12 s after sweep · soak attest 30 s → observers read 1/1 · promote→3/3 applied 75 s · revert→3/3 restored 31 s; conductor PIDs unchanged end-to-end; five typed refusals became five controller fixes on the way (transcript: `genesis/a2o/reports/release-ceremony/2026-09-01/`) |

Cross-cutting lesson (2026-08-31): even atomic changes paid big-bang prices
because everything ships in ONE vehicle (the pod image). **Separate the
delivery vehicle by change class** — and pay the small vehicles off first.

Brainstorm-class: route through /brainstorm (p2p-design-gate applies —
upgrade artifacts, lineage records, and adoption elections are data
entities). Rung 1 may be pulled forward as bounded shift work ahead of the
full design arc. Not to be ground as unplanned shift iterations.

## Rung 6 — lineage-crossing migration (opened 2026-09-03 by the 0.7 cutover)

The 0.6→0.7 line change was a wipe + re-genesis, not propagation: conductor data layer, integrity hashes and the
kitsune2 wire all moved and no vehicle carries data across that. Stations, entry-type questions and the evidence are
in `2026-09-03-lineage-crossing-migration-rna.md` (this arc's next rung; RNA = old+new conductors under the ark, the
storage P2P plane as the cross-line bridge, a migrate-from recipe in the release manifest before data is held, a
Mishpat commitment with revert). Cycle-time row for the big-bang line change: pushed 16:5xZ → conductors on 0.7 with
fresh genesis on the wiped fleet (fill in when edge ≥#1428 lands); seven CI rounds on dispatch defects, zero on the substrate.
