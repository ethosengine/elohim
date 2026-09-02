---
id: "backlog-alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "ESCALATION — all seven alpha conductors have crash-looped since 09:1xZ 2026-09-02 (64 restarts in 2 h after 28 h of none): holochain saturates its SQLite read pool, runs out of blocking threads, its admin listener dies, and the container supervisor exits after 120 s — so both doorways lost every conductor (headless DNS gone), storage's reconcile circuit opens each sweep, every SPA head declare sheds, and alpha serves an index whose assets 404; separately, the wave-3/4 hApp's integrity DNA hash moved for ALL FIVE roles"
slug: "alpha-conductor-crash-loop-after-wave4-roll-and-moved-dna-hashes"
written: "2026-09-02"
author: "shift 2026-09-02T02-20-land-rung5-batch"
status: "open"
priority: "critical"
ceiling: "operator"
relatedNodeIds: []
tags: [escalation, alpha, conductor, crash-loop, sqlite-read-pool, out-of-threads, process-manager, readiness-window, doorway, headless-dns, projection-reconcile, canonical-head, dna-hash-moved, integrity-wasm, mixed-version, north-star]
cites:
  - elohim/elohim-storage/src/conductor/process_manager.rs
  - elohim/elohim-storage/src/happ_manager.rs
  - genesis/orchestrator/manifests/humans/_edgenode-conductor.template.yaml
  - genesis/docs/content/elohim-protocol/architecture/2026-07-12-substrate-trust-contract-runbook.md
  - genesis/data/timeline/backlog/conductor-websocket-flap-breaks-deploy-write-path.md
  - genesis/data/timeline/backlog/ci-deploy-reads-storage-backpressure-as-failure.md
  - genesis/data/timeline/backlog/lamad-dna-workspace-hc-rna-cdylib-link-breaks-coordinator-build.md
---

## Measured (all read-only: Prometheus + Loki via the observability MCP; no kubectl)

**Conductor plane.** `kube_pod_status_ready{condition="true"}` = 0 for every
`elohim-<human>-alpha-conductor-0` (adam, matthew, jessica, james, eve, susan, gertrude).
`kube_pod_container_status_restarts_total` 3–14 per pod, last-terminated reason `Error` (not
OOMKilled). `increase(...restarts[2h])` = 0 for every 2 h bucket of the prior 28 h, then **64** in
08:35–10:35Z; at 15 min resolution the first restart lands in **09:06–09:21Z** and the rate ramps
to ~20/15 min by 10:30Z. CPU on the conductor containers is ~0.01–0.06 cores (limits 2 / 4),
working set ~12 MB — they die before they ever get warm.

**What the conductor says before it dies** (matthew, container instance `0.log`, 10:23Z):
`holochain_sqlite::db::access: Database read connection is saturated. Util 1837.50%` (Cache DB,
also Authored 300–587 %), then `Failed to claim a thread to run the database read transaction.
It's likely that the program is out of threads`, then `queue_consumer: publish_dht_ops_consumer /
integrate_dht_ops_consumer err=DatabaseError(Timeout(Elapsed))`, then `websocket: Websocket closed:
No connection` / `Admin listener finished`. The container's supervisor
(`elohim_storage::conductor::process_manager`) then logs `Conductor not ready yet, retrying in 2s`
×60 with `Connection refused (os error 111)` and `Conductor failed to become ready, attempts: 60`
→ exit → kubelet restart → same storm.

**Downstream, all consistent with "no conductor":**
- doorway-alpha (`intel-nuc`) and doorway-alpha-b (`shem`): `ZomeCaller connecting to FALLBACK
  conductor (primary unavailable)` … `DNS resolve for 'elohim-<human>-alpha-conductor-0.elohim-
  <human>-alpha-conductor-headless.elohim-alpha.svc.cluster.local:8444' failed: Name or service
  not known` for every human — a headless Service publishes no A record for a NotReady pod, so
  the doorway has had 0 conductors (the "0/4 workers" ceiling seen all shift).
- storage `elohim-matthew-alpha-0`: `projection-reconcile[content]: OPENED the
  unresponsive-conductor circuit on a CALL-level failure` every sweep (`healed 0`,
  `to_resolve 847`); `/p2p/status` on both doorways: `projectionReconcile {pending 847,
  completed 0, sweeps 14, caughtUp false}` → the doorway write gate answers **503 catching-up**
  to every deploy write; `election-obey: ELECTION READ FAILED … deadline has elapsed` (conductor
  DB read-permit timeout) and `Websocket closed: No connection`.
- `elohim/dev` #1684 and #1686 (the `[build:app]` redeploy): 12/16 legs, both browser-variant
  canonical-head declares failed on both doorways (alpha 503 catching-up ×3 re-offers; elohim.host
  502 deadline / 503 shedding-writes ×9) — the 665d5b69d ladder re-offered correctly; the
  substrate never came back. **Live effect:** `https://alpha.elohim.host/` serves an index that
  references `main-R7523XGF.js` / `styles-C4EOU6MY.css`, both 404, while the built bundle is
  `main-FILTX4GA.js` / `styles-7XLYMW2X.css`. The atom home (elohim-4a's habit) cannot render.
- `elohim-genesis/dev` red (seed preflight) is this same ceiling.

**Second finding — the DNA hash moved for every role.** storage on adam, 08:48Z (and every
retry since): `DNA lineage mismatch — REFUSING coordinator hot-swap for role (integrity change
needs reinstall/migration, not update_coordinators)` for **lamad, imagodei, infrastructure,
node_registry, mishpat** — e.g. lamad installed `uhC0kkLdCTgRohBd10TDia…` vs bundle
`uhC0kJ1dmjFufe4DNmkVuhQ…`. The hApp baked into `elohim-storage-iroh:1.0.0-dev-8181d60a`
(holochain #1416/#1418, built with the 2026-09-02 DNA-workspace build fix d5fd9642b/03f331f21
— `build.rs --import-undefined` for the hc-rna cdylib link) has different **integrity** wasm bytes
from the installed DNAs. The operator's stated risk at delivery ("the integrity build scripts are
hash-neutral only because CI's linker already imports undefined symbols") did not hold: CI's
integrity bytes moved. The refusal is the CORRECT behaviour (that is what the gate is for) and it
is why the coordinator zome fix 0d572a455 has NOT reached the fleet by hot-swap. Whether the move
is the linker flag or something else in that build is unverified — compare the integrity wasm
sha256 between holochain #1415 (pre-fix) and #1416 artifacts.

## CORRECTED DIAGNOSIS (k8s operator, on-cluster, 2026-09-02 ~11:15Z) — supersedes the mechanism above

Not load, not threads, not the 120 s window. Each conductor **panics 2.3 s into startup**:
`Could not initialize Conductor from configuration: InternalCellError(CellWithoutGenesis(CellId(
DnaHash(uhC0kkLdCTgRoh…), AgentPubKey(…))))` (`crates/holochain/src/bin/holochain/main.rs:201`),
identical on all seven. The 120 s is the supervisor's patience on an already-dead child.

What happened, per node, ~10 min apart along the wave-4 roll (jessica 09:28 → james 09:38 →
gertrude 09:49 → susan 09:59 → eve 10:09 → adam 10:19 → matthew 10:28): first boot after the
roll, `happ_manager` saw `DNA drift detected for role` (bundle integrity hashes ≠ installed) and,
because **`ALLOW_DNA_REINSTALL=true` is set on all 14 alpha StatefulSets**, took
`DNA content drift vs bundle — reinstalling` → `uninstall_app`. Holochain **deleted the five
authored (source-chain) DBs** and began purging DNAs; the conductor-DB transaction then hit a
30 s lock timeout under the post-restart read storm (`uninstall_app failed: …
DatabaseError(Timeout(Elapsed))`). The uninstall is not atomic: files gone, conductor state
still lists app `elohim` Enabled with five cells → every boot tries to create cells with no
genesis → panic. On disk: all five authored DBs are exactly 139264 bytes (empty schema), mtime =
first post-restart boot; `dht/` still holds 393–891 MB per DNA. **The source chains for the old
agent keys are irrecoverable**; re-genesis under the same key on an empty chain is a fork, so the
seven rejoin as NEW agents. The SQLite saturation is real but it is what tore the uninstall, not
what kills the pod. The 120 s readiness window is NOT the fix (the startup probe already allows
600 s; a longer window only slows the loop).

**Decisions (orchestrator, 2026-09-02 11:2xZ):**
1. **Pin the integrity hashes back** — the hc-rna `--import-undefined` link flag (03f331f21) is
   gated to the local toolchain only, and the DNA pipeline prints + guards each DNA hash against
   a committed baseline (`[dna:migrate]` to move it). The ~2.5 GB/node of DHT data belongs to
   the old hashes; the reinstall must land on those DNAs.
2. **`happ_manager` must never treat a standing env flag as migration intent**: drift on a node
   holding data reinstalls only under an explicit per-roll `DNA_MIGRATION_INTENT=<bundle hashes>`;
   `uninstall_app` failure is fatal-and-abort for that boot, never retried into a half-state; a
   saturated conductor DB refuses to begin a non-atomic uninstall.
3. **The supervisor distinguishes a dead child from a slow one** (`try_wait` each attempt; exit
   status + last stderr lines reported immediately) — the death-witness MVP core.
4. **Recovery (operator, destructive, after 1 lands and rolls):** clear `databases/conductor/` per
   node (keeps DHT + cache, fastest rejoin) — not the full `databases/` tree. Consequence to
   record: seven new agent keys → every surface that names those agents by key (hosted-agent
   bindings, custody commitments, fixture humans' `agentPubKey`, attributions) sees a lineage
   break; the 2026-08-30 native-sync recipe's identity binding (station 3) is the place that
   should absorb it, and prod would need migration/lineage, not this.
5. Noted, unintentional: conductor containers run `elohim-storage-iroh:1.0.0-dev-4a81a749` while
   node containers run `…-7513654f` — the conductor template's image tag is not moved by the edge
   deploy; the process_manager fix in 3 will not reach the conductor pods until that is.

## THE HASH QUESTION, ANSWERED (2026-09-02 16:00Z — archived-artifact walk, `hc dna hash` per build)

| Build | Date | lamad | imagodei | infrastructure | mishpat |
|---|---|---|---|---|---|
| 1363–1380 | Jul 18 – Jul 29 | moved | moved | moved | moved |
| 1390–1414 | Aug 2 – Sep 2 02:17 | moved (`8x1UsBQj`) | moved (`RA0NgxsL`) | **MATCH** | moved (`Rnr6SB8c`) |
| 1416–1419 | Sep 2 07:47–14:47 | moved (incident) | moved (incident) | moved (incident) | moved (incident) |
| **1420** | Sep 2 15:20 | = 1414 | = 1414 | **MATCH** | = 1414 |

- The 2 September incident (03f331f21's build scripts) is **bounded and fixed**: #1420 is
  byte-identical to #1414 on every role (189061c6d removed the build scripts; the local link
  need rides RUSTFLAGS in the per-DNA justfiles). The guard works.
- **The fleet's installed lamad / imagodei / mishpat DNAs match NO CI build since at least
  2026-07-18** (node_registry is unrecoverable before #1419 and moved in both recoverable builds).
  The fleet has been running pre-July integrity zomes while every CI build since shipped newer
  ones that never installed — the "DNA changes don't redeploy by default" trap, six weeks deep.
  What the drift probe saw on the wave-4 roll was not the link-flag move; it was that backlog.
- **Pin-back is not viable**: it would mean reverting six weeks of integrity work (lamad T1b
  9b3979d3b, seam-contracts election extraction ffdd7369d, …). The DHT data on the PVCs belongs
  to DNA hashes the code no longer produces.

**Recommendation to the operator: MIGRATE.** (1) Update `elohim/holochain/dna/dna-hashes.baseline`
to #1420's five hashes in a commit tagged `[dna:migrate]` (the guard then passes; the baseline
becomes the fleet contract going forward). (2) Clear the full `databases/` tree per conductor
(not just `databases/conductor/` — the DHT/cache data is orphaned either way), let the edge roll
install the current hApp, nodes re-genesis as new agents on the current DNAs. (3) Re-seed alpha
(genesis pipeline + app bundle stage + `just seed apply`) — the fixture humans' `agentPubKey`
and every key-named binding are re-minted; record the lineage break. Going forward the guard
makes a silent integrity move impossible, and `happ_manager` (867e4bf9b) makes a reinstall on a
node holding data impossible without `DNA_MIGRATION_INTENT` naming these exact hashes — set it
on the alpha StatefulSets for this one roll, then remove it.

## RECOVERY PROCEDURE (operator-owned; written 2026-09-02 17:4xZ for the k8s dev; grounded in the manifests)

**Preconditions (met at 17:28Z):** all seven `elohim-<human>-alpha-0` storage pods run
`elohim-storage-iroh:1.0.0-dev-7d909626` (the happ_manager intent gate + supervisor fix), the
hApp baked into that image is holochain #1420's, and #1421's DNA Hash Guard read MATCH ×5 against
the `[dna:migrate]` baseline. Because the storage code is new, a cleared conductor presents NO app
and the runtime performs a first install — no `DNA_MIGRATION_INTENT`, no `FORCE_DNA_REINSTALL`.

**What is being cleared, exactly.** Each conductor pod mounts one PVC:
`holochain-data-elohim-<human>-alpha-0` at `/var/local/lib/holochain` (= `CONDUCTOR_DATA_DIR` =
the conductor config's `data_root_path`). Under it holochain keeps `databases/` (conductor state,
authored chains, DHT, cache) and the lair keystore `ks/`. Clear **the entire contents of that
mount** — both `databases/` and `ks/`. This is byte-for-byte what the runtime's own node-repair
primitive does (`ConductorManager::clear_conductor_state` = `remove_dir_all(data_dir)` +
recreate). Clearing `ks/` is required: keeping the old lair key on an empty chain would re-genesis
the SAME agent key on a new chain — a fork, worse than a new key.

**What must NOT be touched.**
- The PVC objects themselves (`holochain-data-elohim-<human>-alpha-0`) and their PVs — delete
  contents, never the claim (openebs-hostpath node affinity keeps each on its node).
- The storage pods `elohim-<human>-alpha-0` (container `elohim-node`) and their `/data` PVC
  (`content.db`, blobs) — untouched.
- The conductor pod's `/data` mount — that is a scratch emptyDir, irrelevant.
- No StatefulSet spec edits, no env changes, no image changes.

**The seven, and where they live.** Namespace `elohim-alpha`. StatefulSet
`elohim-<human>-alpha-conductor`, pod `elohim-<human>-alpha-conductor-0`, containers
`elohim-conductor` + `ws-proxy`. matthew, jessica, james on node `ethosengine`; adam, eve, susan,
gertrude on node `shem`.

**Steps, per human (do adam and matthew FIRST — they are the bootstrap pair — confirm both Ready,
then the other five in any order):**
1. `kubectl -n elohim-alpha scale statefulset elohim-<human>-alpha-conductor --replicas=0` and wait
   for `elohim-<human>-alpha-conductor-0` to be gone (nothing may hold the SQLite files).
2. Mount the PVC from a throwaway pod on the same node (a busybox/alpine pod with
   `persistentVolumeClaim.claimName: holochain-data-elohim-<human>-alpha-0` at
   `/var/local/lib/holochain`; the PV's node affinity places it), then inside it:
   `ls -la /var/local/lib/holochain` (expect `databases/` and `ks/`; note sizes), then
   `rm -rf /var/local/lib/holochain/databases /var/local/lib/holochain/ks`, then `ls -la` again
   (expect empty). Delete the throwaway pod.
3. `kubectl -n elohim-alpha scale statefulset elohim-<human>-alpha-conductor --replicas=1`.
4. Verify, in order: the conductor pod reaches Ready with 0 restarts (startup probe allows 600 s;
   first install compiles WASM ribosomes — minutes); the storage pod `elohim-<human>-alpha-0` logs
   a first install (`Installing hApp` / `App already installed … Enabled` on the next tick) and NO
   `DNA drift detected` / `Stale hApp detected` lines; `GET /version` via the doorway that fronts
   that human shows lamad `coordinatorWasmHashes` present and the DNA hash `uhC0k8x1UsBQ…`.
5. Only after all seven are Ready: the elohim operator runs the re-seed (genesis pipeline; app
   `[build:app]` to redeclare the SPA heads) — the shift asks before pushing that.

**Abort conditions.** A conductor that panics again after the clear with `CellWithoutGenesis` means
the mount was not fully cleared (a leftover `databases/conductor/`). A storage log line
`DNA drift detected … keeping the installed cells` means the pod is NOT on the wave-8 image (check
`kube_pod_container_info`); stop and say so — do not set any force flag.

## Why the operator had to act first (original framing, kept for the trail)

The fleet's conductors are in a restart livelock on a persistent PVC; every edge roll (#1413,
#1414 in flight) re-enters it. Nothing in the repo manifests changed the conductor pod spec in
waves 4–5 (verify: `git log --since=2026-09-01 -- genesis/orchestrator/manifests/humans/
_edgenode-conductor.template.yaml`). Recovery needs a hand on the pods (stop the roll, let one
conductor come up with the read storm quenched, or raise its budget), which is cluster-owned.

## Repo-side candidates (after the fleet is stable — do not push during the livelock)

1. `process_manager`: the 60 × 2 s readiness window (120 s) is shorter than a conductor's
   post-restart integration storm on a 1000+-record DHT with 7 gossiping peers. A supervisor that
   exits kills a conductor that was still alive → the restart loop is partly self-inflicted.
   Keep waiting while the child process is alive (bounded by a much larger ceiling), and log the
   child's exit status when it does die.
2. Conductor budget: `Failed to claim a thread … out of threads` under 2 CPU. The
   `CONDUCTOR_ARC_FACTOR` / tuning knobs (memory `project_conductor_arc_resources`) and the
   read-pool size are the levers; the storm is sys-validation + integration after restart.
3. Storage's reconcile/adopt arms hammer a conductor that is already saturated (`fanout 2,
   batch 8` every sweep, election reads per row): the circuit opens per sweep but the sweep
   cadence does not back off — the unresponsive-conductor circuit should hold for minutes, not
   one leg.
4. The DNA-hash move: decide whether the integrity change is intended (then it is a **migration**
   — `ALLOW_DNA_REINSTALL` with lineage, not a blind reinstall) or a build regression (then pin
   the integrity bytes back and add a byte-identity check to the DNA pipeline: the integrity
   wasm sha256 must not move when only coordinator sources changed). Evidence so far
   (elohim-4a, 2026-09-02): `content_store_integrity.wasm` was byte-identical (sha256
   `d1c4e709…`) across the two coordinator-only commits 872bf5789 and 0d572a455, so the move
   points at the hc-rna cdylib/rlib + `build.rs --import-undefined` commits (03f331f21,
   d5fd9642b) compiling differently under holonix. The DNA pipeline console prints no DNA
   hashes (searchBuildLog on #1416 matches nothing), so the which-build question needs the
   packed artifacts. Low-cost follow-up: have the pack step print `hc dna hash` per packed DNA
   so a moved integrity hash is readable from the console.

## Done when

All seven conductor pods Ready with 0 restarts over 2 h; both doorways resolve their primary
conductor; `projectionReconcile.caughtUp true`; a `[build:app]` run reports `✓ canonical head
propagated` for elohim-host-landing and lamad-spa on both doorways and alpha's index assets
return 200; and the integrity-hash question in §4 has an answer recorded on this atom.
