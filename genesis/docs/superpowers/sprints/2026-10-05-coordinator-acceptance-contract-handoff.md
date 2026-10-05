---
title: "Handoff — coordinator acceptance contract: the mesh receipt is owed on shem"
id: coordinator-acceptance-contract-handoff-20261005
date: 2026-10-05
status: handoff
author: "claude-fable-5-1 (Che workspace), at the operator's direction"
habits: [runtime-upgrade-propagation]
cites:
  - genesis/data/timeline/backlog/coordinator-acceptance-tightening-contract.md
  - genesis/data/timeline/backlog/hosted-app-coordinator-coverage-gaps.md
  - genesis/data/timeline/backlog/hosted-authored-content-has-no-storage-row.md
---

# Handoff — coordinator acceptance contract: the mesh receipt is owed on shem

Branch `fix/coordinator-acceptance-contract`, based on `origin/dev` at `52727b6d1`. Not merged to
`dev`. Marks: **[V]** verified here, **[R]** reported and not re-checked, **[I]** inference.

## What is on the branch

1. `content_store` states its statement contract through the extern `statement_contract`, pinned
   by `zomes/content_store/statement-contract.lock.json`. The legacy head-delegation refusal is
   decided from that table and reads `issuer-behind: presented=<form> lowest-accepted=<form>`.
   Coordinator-only. **[V]** 136 zome tests, wasm builds, integrity zome untouched.
2. Storage sweeps the node's own app first and holds hosted apps back (`primary_not_healthy`,
   blocking) when it does not take the bundle cleanly; keeps the last applied bundle at
   `<storage_dir>/coordinators/last-applied.happ`; runs a standing pass (default 300 s, env
   `ELOHIM_COORDINATOR_STANDING_INTERVAL_SECS`) that sweeps app ids it has not read; publishes
   `elohim_coordinator_*` gauges and a `coordinators` object on conductor diagnostics, aggregates
   only. **[V]** `just gate elohim-storage`: 4298 passed, 0 failed.
3. A missing or malformed peer-policy file no longer disables the conductor signal subscribers
   (`PolicyConfig::load_or_builtin`). **[V]** same gate.

## Why it was not proven on a mesh here

`just mesh preflight` on the Che workspace REFUSES: the pinned fork conductor
(`hc-fork-5f4c16abe8cb`) is not built there. Shem ran the 2026-10-04 household proof on that fork.

## The receipt to take on shem

Household, built from this branch (storage binary with `--features "p2p p2p-iroh"`, every DNA's
wasm, bundle repacked by `just mesh start`):

1. `POST /admin/coordinators/sync?apply=false&allApps=true` on a peer: the report carries
   `statementContract` with six rows and `primaryHealthy` absent (dry run).
2. With hosted people cast, apply a variant bundle through the driver: the own app's row comes
   first in each peer's report; `pendingCount` 0 on re-check; `GET /db/p2p/conductor-diagnostics`
   shows `coordinators.pendingRoles: 0` and no app id.
3. Host one more person after the apply and wait one standing pass: their app is on the bundle
   (dry-run that app id: `drifted: false`). This is gap 1's probe.
4. Canary: apply a bundle the own app cannot take (a different DNA lineage is the cheap way).
   Expect every hosted app skipped as `primary_not_healthy`, each a blocking error, none changed.
5. Launch one storage with no peer-policy file: it logs the built-in policy warning and
   `ReaProjectionSignal subscriber registered`.

The `issuer-behind` refusal needs an app on a coordinator older than 2026-09-30 to issue a legacy
grant; unit tests cover it and no household step is asked for.

Write the outcome as a DELTA in `elohim/elohim-storage/.epr-meta/runtime-upgrade-propagation.habit.md`
and re-project. Land on `dev` after the receipt.

## Things to know before landing

- **Boot behaviour changed.** `sweep_other_apps_at_boot` now names the own app as primary; when
  the own app has any role error on an apply, no hosted app is swept at boot. On alpha, 32 hosted
  apps report `CellMissing` roles **[R]**; those are hosted apps, not the own app, so they do not
  trip the gate **[I]**. Confirm on the first fleet read.
- **Heartbeat now runs on nodes that had no policy file.** They run under the shipped example
  policy (nothing exposed externally).
- **A boot from an older image bundle re-records it as last applied.** Last applied wins by rule;
  the boot sweep already applied that bundle to the own app before this change.

## Side findings, not acted on

- `conductor_writes::is_unknown_function_error` may miss Holochain's `ZomeFnNotExists` text if it
  renders "doesn't exist" **[R]** (storage agent's reading; the conductor source was not at hand
  to confirm). It also decides the head-batch fallback. The new contract reader carries its own
  check.
- A declaration made while storage is down leaves no row and nothing retries it; recorded in
  `hosted-authored-content-has-no-storage-row.md`.
- `happ_manager.rs` is past its 3000-line soft ceiling.

## Che, for the record

Che's storage now runs with `genesis/local-dev/che/config/peer-policy.toml` and holds the row for
`fct-leg2-operator-root` (seeded 2026-10-05T01:57:58Z). Leg 2 itself waits on the operator's
hosted-app login; the token has expired.
