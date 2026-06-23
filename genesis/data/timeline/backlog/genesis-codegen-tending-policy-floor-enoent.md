---
id: "backlog-genesis-codegen-tending-policy-floor-enoent"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "genesis Generate-Schema-Types codegen ENOENT on epr:schema:manifest:tending-policy-floor — the schema file EXISTS; manifest-dir-scan resolution gap"
slug: "genesis-codegen-tending-policy-floor-enoent"
written: "2026-06-22"
author: "shift 2026-06-22T1550-dev-buildall-shakeout-doorway-503 ([build:all] shakeout)"
status: "backlog"
priority: "medium"
ci_status: in-progress
jobs: [elohim-genesis]
tags: [ci, genesis, codegen, schema, manifest-dir-scan, shakeout]
cites:
  - elohim/sdk/schemas/v1/manifest/tending-policy-floor.schema.json
  - elohim/elohim-storage/tests/schema_contract.rs
---

# genesis codegen ENOENT on an EXISTING schema (tending-policy-floor)

## The failure (shakeout #1294, [build:all])
`elohim-genesis/dev` **Generate Schema Types** stage failed: codegen `ENOENT` resolving
`epr:schema:manifest:tending-policy-floor` (error_class APP_BUILD).

## Why it's not a missing-file fix
The schema file **exists**: `elohim/sdk/schemas/v1/manifest/tending-policy-floor.schema.json`
(also referenced by `schema_contract.rs` + the content_store_integrity manifest). So the
codegen's `epr:schema:manifest:<name>` → `manifest/<name>.schema.json` **resolution** isn't
finding a file that's present — a scan-config / path-resolution gap, not a missing schema.
Lands near the recent `manifest-dir scan` work (`6e13c540c fix(schema): epr-publish event
property + manifest-dir scan — schema_contract 4->0`). NOT obviously introduced by the
2026-06-22 feat integration; surfaced by running the full [build:all] shakeout.

## Next (for ci-triage / a focused codegen pass)
1. Read the genesis codegen scan logic (which dir/list it walks for `manifest:` ids; is
   `manifest/` included, or only `manifest-payloads/`? — note BOTH dirs exist:
   `manifest/tending-policy-floor.schema.json` AND `manifest-payloads/tending-policy.schema.json`).
2. Confirm the `epr:schema:manifest:` prefix maps to the `manifest/` dir; fix the scan/registry
   so the existing file resolves.
3. Re-run genesis codegen (`pnpm run schema:codegen:ts` / the genesis Generate-Schema-Types
   stage) to confirm 0 ENOENT.

## Severity
Medium — blocks genesis a2o codegen (the genesis pipeline was already red/unstable on the
floor). Not the production-outage blocker (that's the cluster Pending-pods issue, operator-owned).
Deterministic ci-triage owns closure (disappearance on a green genesis streak).
