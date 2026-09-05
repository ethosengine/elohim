---
name: project_tevah_compute_envelope_canonized
title: Tevah compute envelope canonized
description: "Tevah/ark: S0 + station 2 LANDED (2026-09-03) — witnesses custodied via custody-spool with zero DNA change; next station 3b (reach gate on the shard replication plane); traps for running it"
metadata: 
  node_type: memory
  title: Tevah compute envelope canonized
  type: project
  originSessionId: 08dda108-5eac-4580-8178-d1bade78f0ab
  modified: 2026-09-02T14:17:25.259Z
---

Spec: `genesis/docs/superpowers/specs/2026-09-02-compute-envelope-tevah-design.md` (decision register §12,
review disposition §16). Habit `runtime-death-witnessed` born red in `elohim/elohim-storage/.epr-meta/`;
scenario `genesis/a2o/features/resilience/death-witness.feature` (four @wip stations). Nothing built.

**Do not re-derive:** liveness after readiness exists nowhere (`/health` stays 200 by design); on the mesh
the conductor is never storage's child; a new `runtime:*` content type moves the DNA hash (ride
`issue-report`/`node-context` + `metadata_json.kind`); custody commitments are per exact blob hash;
storage holds no signing key (transport keypair authors at `private`); the supervisor must never share an
OOM group with the child it witnesses; `tokio::process` consumes the reap and loses `rusage`.

**S0** = the envelope as its own crate family + binary launching the mesh's conductors (`hc-mesh.sh`
direct mode → `tevah run`), never a witness grown inside `process_manager`.

**Operator decisions 2026-09-02 (spec §12 items 20–24):** branding split — **tevah** in prose/titles/
stories, **`ark`** in code (`elohim/ark/`, `ark-core`, `ark-supervisor`, `ark` binary); never mix in one
surface. WIP fence: `runtime-death-witnessed` ACTIVE (took operator-runtime-surface's slot). The
constitutional DNA batch waits for S1's measured shape. Delivery priority: working code, built through
the update-propagation loop (seed refs are closure-CID/channel-head from the first commit; S0 resolves
pinned-local, S1 resolves a channel head) — accelerate → new habit → dogfood → master → new habit.

**Units (2026-09-02 thread):** `RuntimeManifest` (Manifest-kind EPR, `runtime-manifest`) — never "seed";
`Berth` = per-blade half; a manifest may include a manifest. **Tiers:** hardware = cattle, ark+berth die
with the blade (passport is the berth's), household footprint = the pet held across berths by
commitments; a **berth offer** is a REA intent that renegotiates under-held commitments; loss recurses
(blade→household→commons) and is healed by the same flow. Spec §3.1, register 25–29.

**Guide-star, not a slice:** the `epr-pvc` k8s StorageClass bridge — peers offering external actors a
collective agreement for persistent volumes backed by this runtime. End of the valueflow chain; floated,
unminted; `RuntimeInstance.data_root` reserved as a volume head. Check each slice: does it preclude it?

**Why:** the operator's north star — hyperscaler understanding as a lens for primitives that make a
durable, observable substrate possible over diverse peers on a hostile network.
**How to apply:** design from the register; §1's corrections outrank the two backlog atoms' original
text; do not add a third active habit.

Related: [[feedback_upgrade_propagation_north_star_wall_clock]] [[project_upgrade_authority_constitutional_elohim]]
[[feedback_k8s_is_not_the_architecture]]

**Mesh-run budget is DISK WRITES, not RAM (2026-09-03, from the 0.7 cutover session):** a 0.7 mesh + prologue seed
drove the shared NVMe to 130–270 MB/s, dqlite lease writes hit 3–9.7 s, the devworkspace controller lost leader election
and re-rendered the Deployment WITHOUT its secrets (no ANTHROPIC_API_KEY/GH_TOKEN, sccache unauthenticated, MCPs dead) —
a pod SWAP, not a crash. Before a station-3b/4 mesh run: sample `/sys/fs/cgroup/io.stat` every 30 s, stop the seed if
sustained writes pass ~100 MB/s, never run `just mesh prologue` beside a cargo build, announce "mesh taken"/"mesh free"
to the peer sessions. See [[project_devspace_recovery]].

**Station 3b traps (2026-09-03):** (1) running death-witness stations in SEQUENCE exposes a restarted conductor's
passport reading `ready: false` for seconds — the Background waits ≤90 s now (8446d1dc6); a scoped single-station run
never showed it. (2) After the Act I lane's conductor restarts, storage's `lamad` and `node_registry` role bridges stay
`zomePath: dead` (imagodei/infrastructure re-mint) on 0.6 AND 0.7 — standing bug, backlog
`storage-per-role-bridge-stuck-dead-after-conductor-restart`; custodians cannot author custody through a dead lamad
bridge, so `just mesh storage-restart <custodians>` before a death-witness rerun. (3) The a2o `node_modules` vanished
with a workspace cycle — `just test mesh` exits 127 and STILL writes a sprint-report with `refused 1` (a bogus receipt;
move it aside). (4) `pkill -f <script>` kills your own shell (exit 144) — kill by pgrep pid list excluding `$$`.
