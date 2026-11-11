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

**Proof run, genesis #1756 (2026-11-09, dispatched by 683b10ce3 `[build:genesis]`): half proven.**
- Matthew is fixed: `[=] Matthew … doorway household=household-dowell (projection row exists — key
  bound at registration)`.
- Ezra, Maria and Ronald still fail, with the same `[X] … ORPHANED-BINDING` (log ~3040–3050). The
  seeder reaches that line only on the `exists` path: register answered 409, or 503 "Agent
  already has a Human profile", and the login that followed succeeded with no `installedAppId`.
  A fresh registration would print `[+]`. So when #1756 seeded, an account row for each of them
  existed and accepted the seed password, although the operator's delete reported success.
- The same run's hosted reclaim freed nothing:
  `reclaim-ephemeral-hosted: listed=132 candidates=78 freed=0 skipped-active=60 failed=18`. Each
  failure was `DELETE /admin/hosted-users/<key> -> HTTP 500`. With 60 active hosted agents against
  the doorway's default ceiling of 50 per conductor (`DEFAULT_MAX_AGENTS_PER_CONDUCTOR`), a fresh
  hosted registration may have no conductor to provision on. In that case
  `registration_agent_key` stores the custodial key with no hosted app, the same symptom.

What decides between these is in `doorway-alpha.users` and the provisioning records, which
operators read: whether the three rows exist now, their `created_at` against the delete, their
`agent_pub_key`, whether more than one row matched each identifier (the delete matched
identifier + old key only), and whether a provisioning record exists for them.

**Correction, 2026-11-09 ~03:45Z: the first repair deleted the wrong rows (cause 1, not capacity).**
The seeder sends and prints `ezra@test.elohim.host`. The doorway re-qualifies every identifier to
its own gateway domain (`normalize_identifier`, `doorway/doorway-service/src/routes/auth_routes.rs`
~:1513, `localpart@gateway`), so the row it reads is `ezra@alpha.elohim.host`. The doorway log for
#1756 shows `Login successful: ezra@alpha.elohim.host` at 03:25:30, and the same for Maria and
Ronald. The `@test` rows deleted first were unused leftovers. The rows actually in use:

| row | created_at | agent_pub_key | rows per id | registry rows | provisioning intent |
|---|---|---|---|---|---|
| ezra@alpha | 2026-06-23 | uhCAkcWXXr5k… | 1 | 0 | none |
| maria@alpha | 2026-06-23 | uhCAkvMaCT1Z… | 1 | 0 | none |
| ronald@alpha | 2026-06-23 | uhCAkmihQ8a6… | 1 | 0 | none |

They predate the re-genesis and name conductor-4 or conductor-5, now gertrude's and susan's pods,
which do not host. So registration got a 409, the seed password still logged in, and there was no
app to bind. Capacity is not the cause. The doorway counts it from `conductor_registry` by
distinct agent, and the two hosting conductors (adam, matthew) hold 1 and 2 rows. The operator
backed up the three `@alpha` rows and deleted each by identifier plus its exact old key
(`deletedCount: 1` each; none remain).

**Proof run, genesis #1757 (2026-11-09 ~04:00Z, dispatched by 88f95922c): proven.**
`[=] Matthew … doorway household=household-dowell (projection row exists — key bound at registration)`;
`[+] Ezra`, `[+] Maria` and `[+] Ronald` (hosted) registered fresh; no `Failed humans` block, no
ORPHANED-BINDING line. All four sign in with an installed app binding again. Item 1 below is closed.

**Still owed:**
0. **Fixed 2026-11-09 (artifact-store/seeder-identifier): the seeder reported an identifier the doorway does not store.** Each result
   now carries the identifier from the doorway's auth response (`storedIdentifier`), and the ORPHANED-BINDING line names it
   (`identifier=…`). Tests: `seed-humans-binding.test.ts`. Original note: It prints what it sent
   (`creds.identifier`, `genesis/seeder/src/seed-humans.ts` ~:216 and ~:719), not the doorway's
   normalized identifier. That sent the first repair to the wrong rows. Print the identifier the
   doorway answers with, or apply the same normalization. A re-genesis reset must target rows by
   the doorway's own identifiers.
1. **Proof: done.** Matthew by #1756; Ezra, Maria and Ronald by #1757 (above).
2. **Live backstop.** Build a doorway invariant reporting orphaned bindings = 0: a chaperoned
   (`hosted` or `doorway` phase) account whose key no reached conductor hosts. Today only
   `store_verdict` exists, and it covers the opposite case (store lost, conductors intact). Until
   this lands, only the seeder catches the class. It belongs in `doorway/doorway-service`, beside
   `store_verdict`, surfaced in Dataplane Validation.
3. **Procedure.** A re-genesis rebinds or deletes the doorway user rows whose keys it retires, as
   in the repair above, and never wipes the store. Write it into the re-genesis runbook next to
   `DNA_MIGRATION_INTENT` and the PVC clear. The household's `MESH_RESET` already removes doorway
   identity state with the conductors; alpha needs the targeted form.
4. **The hosted reclaim frees nothing.** In #1756, `reclaim-ephemeral-hosted` hit 18 HTTP 500s on
   `DELETE /admin/hosted-users/<key>` and freed 0 of 78 candidates. It did not cause this failure,
   but ephemeral e2e hosted agents accumulate.
5. **Low priority:**
   - A few `oauth_sessions` rows still reference Matthew and Maria. They are harmless and expire.
   - conductor-7 to conductor-13 hold about 220 registry rows from earlier runs that nothing
     prunes. This predates the re-genesis.
