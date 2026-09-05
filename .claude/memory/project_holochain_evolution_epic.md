---
name: project_holochain_evolution_epic
title: Holochain Evolution Epic — start at the hub
description: "The hApp-lineage-migration epic: spec §11 is the hub every follow-up starts from; 2026-09-05 — Station 6 RED with PROVEN cause (sunset close + CapGrant → permanent blocks), Tasks 29–33 landed, mesh needs a rebuild then r40"
metadata:
  type: project
---

**Where to start:** `genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md` **§11**
(probe board · station board · operator decisions · dated ledger). The operator named it (2026-09-03):
not "rung 6", not a manifesto-tier epic — a spec/whitepaper-level epic; follow-up conversations begin there.

**The design in one breath:** a hApp version crossing is a release on rung 5's channel; refused at verify
unless it names what it migrates FROM and the elohim's `migrates-lineage` Mishpat commitment (k-of-n of an
earned roster under the `constitution_root` baked into the DNA properties) is already notarized; v2 installed
BESIDE v1 under the same agent key; every witnessed fact crosses as a `NotarizationWitness` (v1 action +
signature, re-verified by v2's own validation — entry hashes survive, only action hashes move); dual-cell
peers bridge, graded verified / authentic / foreign across many versions and branches (§5.1); free revert by
re-election until a separately notarized sunset closes the v1 chains. Lane B = conductor-fork spike toward
upstream's chain-continuation draft (one DHT across an integrity change).

**State 2026-09-04 (evening):** Probes A+B PASS on 0.7 in sweettest (`sweettest/src/tests/happ_lineage_migration.rs`,
re-run independently) — the notarization-carrying kernel is PROVEN: entry hash identical across the line, witness
accepted, flipped signature + foreign lineage refused, two cells one key, late open_chain accepted. FINDING: `close_chain`
is not a fence (author and single-conductor authority accept post-close writes; `ActionAfterChainClose` is only the
remote authority's rule) → the sunset's fence is OURS (v1 cell disabled + v2 refuses carried facts after the close);
Probe B2 PASS: the remote authority rejects only the first post-close action and warrants it (tail valid again, record fetchable) — the warrant is evidence, our fence stands. Landing rule DONE (`e233bb4f7`): the witness type rides cargo feature `lineage-witness`, default pack byte-identical
(v1 = `just build`, v2 = `just build-witness`). HASH DISCIPLINE: in an integrity zome ANY line shift moves the wasm
(file!/line! in panic paths) and the entry/link-type macros ignore #[cfg] on a variant — append gated blocks after the
last line, use two cfg-exclusive enums. Codex D2: the dual-cell storage
change is ADDITIVE (`HcClient` untouched, five touch points). Story READY at Stations 1–10 (Station 8 rewritten after B);
habit red, NOT active (operator HELD the WIP slot 2026-09-05); epic ACTIVE (operator accepted §4's posture 2026-09-05); execution subagent-driven, ledger .superpowers/sdd/2026-09-04-holochain-evolution-epic-mvp-plan/progress.md; valueflow sealed story→epic, habit→story. The dev sweettest
pool slot is warm on 0.7 (no recompile). 0.7 binaries persisted at `/projects/.claude-config/tools/{hc-0.7,iroh-relay-1.0.3}`;
the running mesh is still stock 0.6 — the 0.7 mesh is UP (baseline done); the MVP plan
`genesis/docs/superpowers/plans/2026-09-04-holochain-evolution-epic-mvp-plan.md` (15 tasks = 15 flow commitments) is the
execution surface — Tasks 1–11 MVP, 12–14 Stations 6–8, 15 Lane B spike.

**Why:** the 0.6→0.7 cutover was a wipe; production cannot wipe; nobody upstream ships authorship-preserving
migration (0.8 roadmap continues DNA migration, no date). **How to apply:** read §11 first; update its ledger
(dated, evidence-backed) rather than chat; never count a wipe as evidence; locally packed DNA hashes ≠ CI hashes.
See [[project_upgrade_authority_constitutional_elohim]], [[project_holochain_0_7_0_assessment]],
[[feedback_delegate_research_to_opus_sonnet_codex]].

**State 2026-09-05:** the MVP plan (`genesis/docs/superpowers/plans/2026-09-04-holochain-evolution-epic-mvp-plan.md`, SDD ledger `.superpowers/sdd/2026-09-04-holochain-evolution-epic-mvp-plan/progress.md`) landed Tasks 1–11: witness kernel + `export_records` + v2 `carry_from` (node-registry, default hash byte-identical), mishpat lineage arms + in-zome self-signing `create_lineage_commitment`, storage `HappLineage` verify/`verify_path`/`HappLineageVehicle`/`LineageRoles`/passport dual-cell view/`/admin/lineage/reset`, the a2o fixture. **Measured live (r15):** Stations 1, 2, 3, 4, 9 green in one run on the 0.7 household mesh — a notarized path, a dual-celled canary under one key, carry equality, forged witness refused. **Blocked on zome gaps:** Station 5 (held-carry unreachable — `export_records` is a local query; Task 12 bridge sweep) and Station 10 (no roster check anywhere; `constitution_root` never reaches the passport). Open: revocation invisible to non-authoring peers (Station 7). Remaining tasks 12–15 (bridge sweep, revert, sunset, Lane B spike). Live-mesh traps: installed DNA hash ≠ packed (modifier folding); coordswap bundle must carry the mesh's own DNAs ([[project_local_mesh_binary_slot_and_restart]]); stock 0.7 conductors grow ~1 GB/h ([[project_conductor_arc_resources]]).

**Update 2026-09-05 (later):** Tasks 16–18 + 17b landed (roster check, root written at install and verified installed-first, held-view export). Stated in the hub and to be repeated wherever Station 10 is discussed: the roster check is a coherence check against an author-MINTABLE electorate (mishpat integrity verifies no signatures; no arm binds a roster to the elohim's key or root) — not yet a trust boundary; the boundary is the integrity-side lineage arm (hash-moving) or storage re-verifying the roster's chain to the root. Process lesson: never two implementers in one crate (a required-field addition blocked a lane ~45 min); parallelism only across crates. The valueflow-authoring skills (another session) prescribe `epr flow claim` / note kind `ruling` that the binary gains only when that session installs it.

**Trust-boundary ledger (2026-09-05, epic §11.4):** G1 roster not bound to the elohim's key/root (author-mintable electorate) · G4 revocation visibility closed on the read path (Task 19: lifecycle from `CommitmentByState` links) · G6 cross-courier after-close fence not expressible on HDI 0.8 · G7 state links forgeable (tag-shape-only validation, public `create_commitment_state_link`). All four are integrity-side (hash-moving) and ride ONE sunset-hardening crossing; the rehearsal measures with them open and says so. Operating rule: mishpat coordinator hot-swap before the storage roll; re-notarize pre-swap lineage commitments.

**Update 2026-09-05 (late):** Tasks 12, 13a/13b, 14a, 19, 20 landed and reviewed (revert arm + `readopt_from`, seal_close with half-seal resume, DHT-visible lifecycle, carry idempotency, bridge sweep). Gaps G8 (quadratic export walk → Task 24), G9 (O(W²) after-close validation → Task 25, crossing), G10 (entry types matched by index across lineage ends → Task 26) filed; the risk discipline (`risk`-tagged backlog rows, inject rules, habit guards R1–R4) landed via the operator's fork. Remaining before the second live run: 14b + 13c (storage sunset + readopt wiring), Task 24, then coordswap + storage rebuild + Stations 5–10 live, then the whole-branch review.

**Overnight 2026-09-05:** Stations 1–10 ALL GREEN individually on the household mesh (Station 8 as narrowed); the ten-station receipt run in flight. Two sibling plans minted from the operator's mandate: `genesis/docs/superpowers/plans/2026-09-05-rung5-workspace-orchestration-plan.md` (packager reads the authoring cell; `POST /admin/runtime-config/follow`; alpha enrolment as data in deployments.json; workspace→alpha observe-mode measured — the ONE gap between "proven on three peers" and "propagates to the fleet" is that no API writes ELOHIM_RELEASE_CHANNELS) and `2026-09-05-dataplane-convergence-final-scenario-plan.md` (bind federation-deploy's @wip final scenario to the proven carried-election path; pain-points T7 pointer heal — the top red is unmeasured, not failing).

**Measured 2026-09-05 (Station 6):** on a partial-arc mesh peer, `get_agent_activity` with `GetOptions::local()` returns a frozen subset of a neighbour chain (212 of 322, unchanged across a conductor respawn); `network()` answered empty only in the single-conductor sweettest. Rule: reads of ANOTHER agent chain ask the authorities first and fall back to local only when the network answers empty, saying which view answered (Task 29). Whole-branch review verdict: mergeable-with-fixes (reset must not un-sunset; watch.rs split; story wording; fixture socket leak).

**State 2026-09-05 (after the overnight run):** Station 6's cause is PROVEN from the conductor dbs (Task 30
`hc-dbtool`, `hc-mesh.sh blocks <peer>`): Station 8's `seal_close` closed the household's real v1 chains and the next
`CapGrant` (every client's `authorizeSigningCredentials` — storage on reconnect, the a2o rail) was warranted by every
neighbour → the author's cell blocked forever (0.7, no unblock) → storageArc null, 0 gossip rounds, no authorities.
Landed: Task 29 (held page labels `authority|local-only|unreachable`, hot-swapped), Task 30 (tool + gate), Task 31
(fixture writes nothing after a close; poisoned peer refused at pre-flight), rung-5 Task 2 (`POST /admin/runtime-config/follow`),
convergence Task 1 GREEN live + Task 2 (T7 heal), mesh round deadline 60 s (86acd1926). Tasks 32 (durable close ledger, persisted credentials, partition probe measured live on all three peers — shapes
`timeouts` | `isolated`) and 33 (v1 binding with a reset latch; Station 8's block-list Then) landed and approved;
the fixture's run-scoped predecessor staging is a measured-only follow-up on the rebuilt mesh (recipe in task-33-report). The mesh's node-registry space
is poisoned until rebuilt (fresh cells); Station 6 unmeasurable until then. See
[[project_household_space_partition_blocks_and_round_deadline]].

