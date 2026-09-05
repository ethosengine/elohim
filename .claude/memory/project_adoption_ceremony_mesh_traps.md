---
name: project_adoption_ceremony_mesh_traps
title: "Rung-5 adoption ceremony: mesh traps + refusal map"
description: "Rung-5 adoption ceremony on the local mesh (2026-09-02): preconditions, candidate shape, typed-refusal → fix map, and the drivers' shapes — reach for it before any release/election measure on the mesh."
metadata:
  type: project
---

First full §10 receipt passed on the local mesh 2026-09-02 00:43Z (transcript
`genesis/a2o/reports/release-ceremony/2026-09-01/`; commits bd5d3984b 547c28d62 851ab2fae 2b02dd86f
6d36259f1 beeb38306). Numbers: publish→3/3 staged ≤19 s · canary hot-swap ~12 s after a sweep ·
promote→3/3 75 s · revert→3/3 31 s; conductor PIDs unchanged end to end.

**Preconditions that bit:** (1) doorway A (`:8888`) is the conductors' bootstrap+signal home — with it
down the conductors are islands (`NoPeersForLocation`), the iroh peer took 20 min to see a head; check
`/health` before any election measure. (2) the storage debug slot must be built `--features "p2p
p2p-iroh"` or `storage-restart` refuses dual/iroh peers. (3) the runtime-config watcher is OFF unless
`ELOHIM_RUNTIME_CONFIG_PATH` is in the peer's env — restart per peer with
`MESH_RESTART_ENV_OVERLAY="ELOHIM_RUNTIME_CONFIG_PATH=/tmp/elohim-local-mesh/<peer>/runtime-config.toml"`
(overlay is global per invocation). (4) never poll with an empty cid in the grep — a false 3/3.

**Candidate shape:** coordinator-only = byte-identical integrity wasm + different coordinator wasm.
The 2026-08-30 deployed bundle is CROSS-lineage vs a fresh workdir pack (integrity bytes differ) and the
vehicle refuses it `apply_failed / DNA lineage mismatch`. Mint with `COORD_BUILD_MARKER=<x> just build`
(zome `zome_build_info` knob) — currently blocked by the hc-rna cdylib wasm link atom; fallback used: a
wasm custom section appended in python, DNA hash unchanged.

**Typed refusal → meaning:** `threshold_unmet` = honest no-attestation-yet (`threshold_unchecked` =
count unreadable); `lineage_parent_mismatch` on a 2nd release = the zome's star chain (update_content
targets the root) — controller proves the declared parent by existence on the channel; `apply_failed`
with `DNA lineage mismatch` = wrong candidate, not a bug. Modes: `=canary` adopts staging (soaks),
`=apply` adopts earned only (`waiting/awaiting_promotion` on staging, no backoff), `=observe` reports.

**Driver shapes:** packager `--applies-to-from http://localhost:8090` (base URL, it appends /version);
a revert target is a NEW manifest for OLD bytes (`--applies-to-from` a peer running the current
release, `--lineage-parent <current>`, `--attestation-threshold 0` for bytes the fleet already ran) and
`release-ceremony.ts revert <channel> <manifest.json>` authors + declares earned in one act. A
refused channel sits in the backoff ladder (up to 1800 s) — a warm `storage-restart` clears it; a fresh
channel id costs one runtime-config reload. Related: [[project_local_mesh_binary_slot_and_restart]],
[[feedback_upgrade_propagation_north_star_wall_clock]], [[project_upgrade_authority_constitutional_elohim]].

**2026-09-04 overnight (rung 5 on the 0.7 mesh; Station 9 = second release on a long-lived channel):**
(5) the a2o fixture converges every peer to its pinned BASELINE pair before/after each run — if the pin
(`E2E_BASELINE_HAPP` + `E2E_BASELINE_COORDINATOR_WASM_HASH`) names older bytes than the mesh runs, the run silently
REVERTS earlier deliveries (it hot-swapped lamad back to the old content_store zome); pin to a bundle carrying the
mesh's current bytes for EVERY role (`genesis/a2o/reports/release-ceremony/2026-09-04/baseline-N/elohim-c3.happ`
shape: baseline lamad.dna with only the coordinator wasm swapped, other roles from the epic bundle). (6) a
registered channel in `apply` mode is a standing force: after the fixture moved bytes, the still-registered c3
channel re-asserted its earned head on the next sweep — de-register experiment channels (restore the
`runtime-config.toml` byte backup + reload) before a measure run. (7) an apply-mode row's `attestations` block is
evidence for the release THAT ROW RESOLVED (the winner); a candidate staged beneath an earned head shows its
attestation only on the canary's row (its resolvedHead IS the candidate) — read evidence there, or by cid via the
zome (`get_attestations_for_subject`; no HTTP route). (8) `release-ceremony.ts channel create` accepts ids the
release-manifest schema refuses (uppercase stamp) and authors an empty root before the packager rejects — lowercase
stamps; backlog row filed. (9) every NON-FIRST release on a channel must declare `--lineage-parent <head cid>` on
0.7 (the chain orders releases now; a null parent is `lineage_parent_mismatch`); first releases and fresh teardown
channels keep null. (10) `artifact_unavailable` on followers was structural until the peer-pull source (storage
35f0746ad + c8930c2f2, shard-manifest following); before it every non-packaging peer needed a hand `PUT /blob`.
