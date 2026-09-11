---
id: "backlog-doorway-hosted-cell-grant-minted-before-row-insert"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Hosted register mints the delegates-compute grant BEFORE the user row is inserted — an insert failure leaks a live notarized grant for an agent that never existed at the doorway"
slug: "doorway-hosted-cell-grant-minted-before-row-insert-2026-09-11"
written: "2026-09-11"
author: "doorway-federation sprint — Task 17c (rust-architect, live-mesh diagnosis)"
status: "open"
priority: "medium"
tags: [doorway-service, hosted-cell, delegates-compute, ordering, idempotency, C6b, D8]
relatedNodeIds: []
cites:
  - doorway/doorway-service/src/routes/auth_routes.rs
  - doorway/doorway-service/src/routes/hosted_cell.rs
  - genesis/docs/superpowers/plans/2026-09-10-doorway-federation-three-reds-to-green-plan.md
---

# Grant minted before the row lands

## Observed (2026-09-11, household mesh, doorway log 04:15:16–04:15:24)

Re-registering a CLOSED identifier: the duplicate pre-check passed (`find_one` already filters `is_deleted`), a new
agent was provisioned, `Hosted: the cell is a notarized promise` was logged (grant `uhCEk64woTR-…` issued on the pool
peer), and then the `UserDoc` insert died on the `identifier_unique` Mongo index — no `Registered new user` line. Net
effect: a live, notarized `delegates-compute` commitment naming an agent key that has no account row anywhere, and a
provisioned cell nobody can reach. The immediate cause (closed rows still holding the identifier) is cured by
`closed_identifier_tombstone` (eff30a245); the ORDERING that made the leak possible is not.

## Why it matters

C6b (idempotent effect): a notarized side effect must not precede the local commit that makes it meaningful. Any future
insert failure (index, Mongo outage, validation) reproduces the leak — and the leaked grant is real capacity on the pool
peer (rate_per_hour, rotation_ttl) with no revoke path, because revoke hangs off close-account, which needs the row.

## Shape of the fix (bounded)

In the hosted register tail (`auth_routes.rs`, the block after `provision_agent` / before `UserDoc::new_with_custodial_key`):
insert the row FIRST with `hosted_cell_grant_cid: None`, then issue the grant, then `update_one` the cid + provider +
valid_until onto the row; on grant failure the row stands (registration succeeds without a promise — the tested
`hosted_register_without_pool_compute_config_still_registers` shape). If the row insert fails after provisioning,
deprovision the cell (already the close path's `deprovision_agent`) so no orphan cell remains either. Contract test: a
forced insert failure leaves zero grants and zero cells. Seam row update on `hosted_cell_grant_body` / a new
`register_side_effect_order` predicate.
