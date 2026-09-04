---
title: "Holochain 0.7.0 fleet cutover — operator runbook (F5 of the upgrade guide)"
id: holochain-0-7-fleet-cutover-runbook
status: Active
class: substrate
domain: substrate (alpha fleet re-genesis · relay plane · conductor line change)
sprint: 2026-09-03 (executes after F4 lands the 0.7 images; owner = the operator, integrator on call)
serves:
  - runtime-upgrade-propagation
  - dataplane-convergence
cites:
  - "holochain-0-7-upgrade-guide | Holochain 0.7.0 Upgrade Guide | sha256:8819a0f70c1d72d3 | path: genesis/docs/superpowers/plans/2026-09-02-holochain-0-7-upgrade-guide.md"
  - "substrate-trust-contract-runbook | The Substrate Trust Contract | sha256:e47d962ca7259c79 | path: genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md"
  - genesis/data/timeline/backlog/iroh-cross-relay-preflight-fails-closed.md
memory_anchors:
  - project_alpha_dna_migration_2026_09_02
  - project_holochain_0_7_0_assessment
  - project_alpha_topology_bootstrap_pair
  - project_devspace_recovery
---

# Holochain 0.7.0 fleet cutover — operator runbook

This is the sequenced, operator-owned half of Lane F in the 0.7 upgrade guide (F5 + F5a + E10).
The integrator's half (F1–F4: the single push of `upgrade/holochain-0.7`, the conductor image,
the DNA hash guard, the edge roll) must be finished before step 1 below; nothing here is a
substitute for it. Cluster state is read through Jenkins and the repo manifests, never
`kubectl` from the dev environment — the operator runs the cluster, this runbook tells them the
order and the probes.

## Authorizations already given (2026-09-03)

- **Wipe the WHOLE fleet clean.** Every alpha conductor's `databases/` tree and keystore, and
  each peer's storage state (diesel DB, blob stores, iroh/libp2p keys). The 0.7 fleet is born
  from one clean genesis and one re-seed; nothing 0.6-era is carried. Holochain 0.7 has **no
  data migration** from 0.6 and the DNA hashes move, so this is not optional — it is the only
  path.
- **Per-doorway relays stay** (operator ruling D2). Doorways are different operators; a
  human's conductor homes to their primary doorway's relay and the fork's cross-relay preflight
  fallback lets peers homed on different relays talk. Two relays, one network, on purpose.

## Preconditions (all must be true before step 1)

- [ ] The 0.7 push has landed on `dev` and the orchestrator has dispatched **conductor → dna →
      edge** in that order (do not hand-trigger). The `elohim-conductor` job published
      `elohim-edgenode:conductor-<hc12>` where `<hc12>` is the first 12 chars of the
      `elohim/holochain-conductor` gitlink on `dev` (branch `elohim-0.7`).
- [ ] The edge build's DNA Hash Guard printed `DNA-HASH <role> <hash>` for all five roles and they
      match `elohim/holochain/dna/dna-hashes.baseline` on `dev` (the push carried `[dna:migrate]`;
      if the guard printed hashes the baseline does not have, the integrator updates the baseline
      from those CI lines and re-pushes before you continue — local `hc dna hash` values differ from
      CI's and must never be pasted into the baseline).
- [ ] The storage image for this roll embeds that conductor tag (`scripts/ci/build-storage-image.sh`
      logs `CONDUCTOR_SOURCE_IMAGE`).
- [ ] **Relay 1.0.3 is in the same wave.** The edge pipeline built and pushed
      `harbor.ethosengine.com/ethosengine/iroh-relay:1.0.3-dev-latest`, and both relay
      Deployments (`genesis/orchestrator/manifests/doorway/alpha.yaml`, `alpha-b.yaml`) pin that
      tag. The 0.7 conductor resolves stock `iroh 1.0.3` / `iroh-relay 1.0.3` as its relay client;
      kitsune2 0.5 dials only peers whose advertised relay matches its own exactly. **A 0.7
      conductor against a 0.95.1 relay is a booted conductor with 0 connections, and it is
      silent** — no crash, no error, just no peers.
- [ ] `DNA_MIGRATION_INTENT` is staged for every node with the five baseline hashes (see step 3).
- [ ] Both doorways answer `/health` today, so a post-roll failure is attributable to the roll.

## Sequence

1. **Roll first, wipe second.** Confirm the edge roll carrying the 0.7 hApp and the storage image
   embedding `conductor-<hc12>` has reached every alpha StatefulSet — all 7 active peers (4 shem +
   3 household; `genesis/orchestrator/data/deployments.json` `suspended` is the roster) and the
   conductor workloads — and that both relay Deployments are on `1.0.3-dev-latest`. Nothing below
   starts until every pod in the wave is on the new images.
**Observed on the first roll (edge #1426, 2026-09-03 21:0x–21:5xZ) — read before step 2.** The storage phase rolled
all seven peers (`elohim-{adam,matthew,jessica,james,gertrude,susan,eve}-alpha` "rolling update complete"), the alpha
doorway rolled, and the alpha-b relay Deployment was reconfigured to `iroh-relay:1.0.3-dev-latest`. The CONDUCTOR
phase did not: it rolls the conductor StatefulSets one peer at a time and waits `rollout status --timeout=600s`
for readiness, jessica's 0.7 conductor cannot become ready on 0.6 databases, and after the first failure the phase
HALTS ("NOT rolling elohim-<peer>-alpha-conductor — an earlier peer's conductor rollout failed"). So the fleet now
runs 0.7 storage against six 0.6 conductors plus one unready 0.7 conductor (jessica). Consequence for the sequence:
the pipeline's readiness gate means a conductor cannot be rolled onto 0.6 data at all — for the conductor phase the
wipe comes FIRST. Adjusted order: (a) scale every `*-alpha-conductor` StatefulSet to 0 (or delete the pods and let
the StatefulSet hold them down) and clear the conductor `databases/` + keystore on every peer, bootstrap pair
together; (b) stage `DNA_MIGRATION_INTENT` (step 3); (c) re-dispatch the edge roll (`[build:edge]` on dev, ~70 min;
storage re-rolls idempotently, the conductor phase then finds every peer ready on fresh genesis); (d) continue at
step 4. Do NOT push anything else to dev while that edge run is deploying — a superseding orchestrator run cancels
the calling pipeline and marks the roll aborted mid-`rollout status` (that is what cut #1426's conductor phase).

2. **Clear the conductor state — bootstrap pair together.** ONLY THEN clear each conductor's full
   `databases/` tree and keystore. 0.7 cannot read 0.6 databases and the lair/agent re-key is
   expected. Clearing earlier makes the still-running 0.6 storage re-install the 0.6 hashes and
   re-key twice. **adam and matthew (the bootstrap pair) are cleared in the same window** — if only
   one of them comes up on the 0.7 hashes the fleet splits into two DHTs.
3. **First boot with intent.** The five packed hashes, from elohim-holochain #1424 (2026-09-03, the same values now in
   `elohim/holochain/dna/dna-hashes.baseline`):

   ```
   DNA_MIGRATION_INTENT=uhC0ka1Tpt-_mtrGKILPnCAWTINwdtoRlYreoQM9GWsdVnbDgshr9,uhC0kRTnoJojlGSinY8Ko3BWUG55YHQYODF00tZahrxwqJ1cuz9r7,uhC0k5a385O0UxmKsi0DinnaLOOirNrV0B0CPQG0f-X-_n-PDHnFb,uhC0k3wIqzPHIBDyUp_UdTq7BsvnYT9h8ZA9-igXlQ_V7tQRj9xb6,uhC0kJOs8Qf5Vs1WDNqj2AWVZRqPbAKzZL9kwrPL4AR3JMF3OaOGD
   ```

   FORMAT (elohim/elohim-storage/src/happ_manager.rs): a comma-separated list of BUNDLE DNA hashes only — no role names.
   It is read by the `elohim-storage` process (the one that installs the hApp into the conductor), so it goes on the
   STORAGE container's env, not the conductor's. **With the full wipe of this ceremony it is NOT needed**: a conductor whose
   `/var/local/lib/holochain` was cleared has no app, and storage first-installs the 0.7 hApp with no flag at all. Set it only
   on a node whose chains you deliberately keep.
   `DNA_MIGRATION_INTENT=<the five baseline hashes>` on every node for
   the first boot, so the storage supervisor installs the 0.7 hApp deliberately instead of reading a
   "not stale" role structure and doing nothing.
4. **Watch the supervisor, not the pod status.** A dead conductor child is reported with its exit
   status and a stderr tail. `CellWithoutGenesis` on boot means step 2 ran before step 1 on that
   node — stop, finish the roll there, clear again. Coordinator-only changes never move a DNA hash;
   if a shipped fix "did not land", first check which zome class the diff touched.
5. **Verify with the trust-contract probes** (the substrate trust-contract runbook is the
   authority when this text and live behaviour disagree):
   - every peer reports `caughtUp` on the doorway `/p2p/status`;
   - `GET /db/p2p/conductor-diagnostics` on each storage peer shows agent infos for the whole
     fleet AND **connections > 0**. The D2 measure is explicit: a peer homed on `relay.alpha`
     connected to a peer homed on `relay.elohim.host`. If `transportStats` reads
     `{"serializeError": …}` (a known 0.7 view bug, hash-neutral follow-up), read the admin
     interface's `dump_network_stats` directly — `connections` is the field that matters;
   - the deploy prints `✓ canonical head propagated`.
   Restart churn is roughly 20 minutes and fresh actions need publish time; do not read a red
   probe as a failed cutover inside that window.
6. **Re-seed once.** Content seeds via the operator's pipeline for the fleet
   (`[build:genesis]`; the pre-cutover re-seed was deliberately held). For the household this
   host owns, `just seed apply mesh content` per the seed workflow. Seed exactly once; the seeder's
   post-flight now waits for the reconcile sweep to anchor bulk rows before it reads them back.
7. **Do not roll conductors alone afterwards until the storage handle fix ships.** A storage peer
   whose conductor restarts re-mints its registry client correctly, but the reanchor and
   head-adoption sweeps keep the boot-time handle and fail until storage restarts (branch backlog
   `558c43f08`). Until that lands, a conductor-only roll must be followed by a storage restart on
   the same node.

## Manifest strictness (F5a — why a "small" conductor change can take the fleet down)

0.7's `AppManifestV0` restores `deny_unknown_fields`, and `NetworkConfig` dropped `enable_mdns`
along with every tx5 key (`signal_url`, `webrtc_config`). A conductor-fork manifest field the
storage client does not know takes the admin seam down fleet-wide — the 2026-08 dev.23 incident
class. Rule: any conductor-side manifest addition and the storage `holochain_types` pin land in
the SAME batch; `tests/happ_manifest_relay_url_compat.rs` is the tripwire. Unknown conductor-config
keys hard-fail startup: `scripts/ci/validate-conductor-config.sh` refuses the tx5 keys and requires
`relay_url`.

## Rollback

There is no in-place rollback across the line: 0.6 cannot read 0.7 state either. Rolling back
means re-deploying the last 0.6 images AND clearing state again AND re-seeding — the same
ceremony in reverse. Decide on the connections>0 probe in step 5, not on the a2o lane; the
lane's baseline carries pre-existing reds.

## Evidence to bank when green

One-line DELTA in `elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md` and in
the dataplane-convergence habit atom (fleet on 0.7 on fresh chains, receipt id), then
`.claude/scripts/habits-project.py`; the Wave 3 outcome line in the convergence campaign plan; the
"LANDED" line in memory `project_holochain_0_7_0_assessment`; a cycle-time row for "conductor line
change" in the upgrade-propagation arc table.
