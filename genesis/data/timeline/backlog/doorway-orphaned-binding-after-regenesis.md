---
id: "backlog-doorway-orphaned-binding-after-regenesis"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "A re-genesis re-keys the alpha conductors but leaves doorway Mongo users rows naming the old agents, so login succeeds and /hc/connect answers 401; no live doorway invariant counts orphaned bindings and no procedure says how a re-genesis rebinds the rows"
slug: "doorway-orphaned-binding-after-regenesis"
written: "2026-11-09"
author: "artifact-store sprint 2 orchestrator (claude-opus-5-5, shem)"
status: "open"
priority: "high"
jobs: [doorway, genesis, alpha]
cluster: "regenesis-store-coherence"
tags: [doorway, mongo, regenesis, key-binding, orphaned-binding, live-probe, operator-procedure]
---

**Observed 2026-11-08/09.** Regression class 12 in
`genesis/docs/superpowers/sprints/2026-11-05-regenesis-1/regression-classes.md`. The alpha
re-genesis re-keyed every conductor, but the doorway's Mongo `users` collection was not reset.
Genesis #1755 (`elohim-genesis/dev`, log lines ~3025–3049) failed four humans with
`ORPHANED-BINDING … doorway=alpha-elohim-host reason=Session is missing its installed app binding`:
Matthew (doorway phase, the operator) and Ezra, Maria and Ronald (hosted). Node and device humans
passed (`[=] projection row exists`). The seeder check that names the failure landed in 7fe827e28
(`genesis/seeder/src/seed-humans.ts` `orphanedBindingLine()`; tests in
`genesis/seeder/src/__tests__/seed-humans-binding.test.ts`).

**Operator repair, 2026-11-09 (targeted, not a full reset).** Confirmed from `doorway-alpha.users`:
- Matthew's row held the pre-re-genesis key (`hCAkJH75…`). Conductor-1's elohim key is now
  `hCAkQC1z…`, and no row held it.
- Ezra and Ronald pointed at conductor-11, the old emma pod, which is scaled to 0 and whose
  registry rows date from June. Maria's key had no registry row at all.

Acts:
1. The four original rows were backed up to a session scratch file on the operator's machine.
2. Matthew's row was rebound to the conductor-1 key. The update was conditional on his identifier
   and old key both matching and the new key being unheld, and it changed one row. His ADMIN level,
   steward flag and `stewardship_at` are unchanged. The key was checked against the doorway's own
   registry.
3. Ezra's, Maria's and Ronald's rows were deleted, matching on identifier and old key. The next
   genesis run registers them fresh, and the doorway provisions their hosted apps.

A full wipe was rejected. `users` holds 4,039 rows, and a wipe would have dropped the passing node
and device rows, Matthew's steward record and 23 accounts created after the re-genesis.

**Still owed:**
1. **Proof.** The next `elohim-genesis/dev` run lists all four as created or bound, with no `[X]`.
   Record the build number here, then close item 1.
2. **Live backstop.** Build a doorway invariant reporting orphaned bindings = 0: a chaperoned
   (`hosted` or `doorway` phase) account whose key no reached conductor hosts. Today only
   `store_verdict` exists, and it covers the opposite case (store lost, conductors intact). Until
   this lands, only the seeder catches the class. It belongs in `doorway/doorway-service`, beside
   `store_verdict`, surfaced in Dataplane Validation.
3. **Procedure.** A re-genesis rebinds or deletes the doorway user rows whose keys it retires, as
   in the repair above, and never wipes the store. Write it into the re-genesis runbook next to
   `DNA_MIGRATION_INTENT` and the PVC clear. The household's `MESH_RESET` already removes doorway
   identity state with the conductors; alpha needs the targeted form.
4. **Low priority:**
   - A few `oauth_sessions` rows still reference Matthew and Maria. They are harmless and expire.
   - conductor-7 to conductor-13 hold about 220 registry rows from earlier runs that nothing
     prunes. This predates the re-genesis.
