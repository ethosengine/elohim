---
epr-habit-version: 1
id: happ-lineage-migration
invariant: >
  A hApp version crossing (an integrity change, a new DNA hash) is carried by the network
  itself: refused at verify unless the release names what it migrates from and the elohim's
  migration commitment for that path is already notarized (never a per-node or household consent); adopted beside v1 under the SAME agent key; every witnessed fact
  crosses with its original v1 action + signature, re-verified by v2's own validation; dual-cell
  peers bridge the window; revert is free by re-election until a separately ratified sunset
  closes the v1 chains. No wipe, no re-seed, no re-key, ever again.
status: red
active: false
checks:
  - "a2o @concern:happ-lineage-migration (genesis/a2o/features/delivery/happ-lineage-migration.feature — Stations 1-10, @wip; steps not yet written; rehearsal = node_registry v1 → v1+NotarizationWitness on the household mesh, 0.7)"
  - "spike verdict (§9 Lane B, conductor fork): must_get_record_from_lineage host fn measured in elohim/holochain-conductor before B2 is decided — recorded as a dated delta below"
refs:
  - "spec: genesis/docs/superpowers/specs/2026-09-03-holochain-evolution-epic-design.md"
  - "backlog home (the epic's backlog item): genesis/data/timeline/backlog/governance-native-dna-upgrade-path.md"
  - "arc: genesis/data/timeline/backlog/upgrade-propagation-p2p-design-arc.md (ladder row 6)"
  - "upstream kernel: elohim/holochain-conductor/crates/holochain/tests/tests/migration.rs (chain-switch, #5842)"
retire-when: >
  when three consecutive integrity changes reach the alpha fleet through their release channel
  with every witness verified and zero records re-seeded from outside, each with its cycle-time
  row in the arc doc — the wipe is then not a delivery path for any class and the register
  describes a product, not a practice.
---
DELTA 2026-09-03 (birth, RED): declared after the 0.6→0.7 cutover proved the class has no vehicle
(the fleet was wiped; rung 5 passed 5/5 on 0.7 but carries only coordinator bytes on a fixed line).
Grounded by four readers (Sonnet ×2, Opus, Codex): the v1 action signature carries no DNA hash and
verifies under v2; `must_get_*` is same-DNA only so the proof is embedded; upstream ships a
chain-switch test (#5842) validating a signed summary against DNA-properties signers — authorship
preserved, notarization not, which is the gap the NotarizationWitness closes. Rehearsal DNA:
node_registry (6 entry types, 1 storage call site, own bundle, not health-supervised). First
runnable red: Station 1 (verify's DnaLineageMismatch gains its positive branch). Not active: the
WIP fence is full (dataplane-convergence, runtime-death-witnessed); promotion is the operator's call.

DELTA 2026-09-04 (Probes A+B, sweettest on the 0.7 line, `sweettest/src/tests/happ_lineage_migration.rs`, EXIT=0, 2 passed;
re-run independently by the chief, 164 s): the KERNEL is green — v1 record re-created on v2 keeps its entry hash; the
NotarizationWitness carrying the v1 action + signature is accepted by v2's own validation; a flipped signature and a
foreign lineage hash are refused with typed messages; two cells under one agent key; late open_chain accepted.
FINDING: close_chain is not a fence (author and single-conductor authority both accept post-close writes) — Station 8
rewritten (the fence is ours: v1 cell disabled, v2 refuses carried facts after the close); Probe B2 (two conductors)
opened. Status stays RED: the habit measures the STORY on the household mesh (0 of 10 stations), not the kernel.
Landing rule: the witness type rides cargo feature `lineage-witness` so the default pack stays on `dna-hashes.baseline`.
