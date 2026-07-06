---
title: Genesis scenario failures — two in-repo code leads (seeder upsert + non-admin authz)
status: open
ci_status: open
severity: medium
discovered: 2026-07-06
discovered_by: shift/overnight-genesis-pipeline-stabilize
domain: content-pipeline / auth
pipelines: [elohim-genesis]
requires_env: alpha-cluster-6peer  # verification needs a genesis run against a HEALTHY alpha
---

## Context

The genesis API suite's UNSTABLE baseline (15 failed / 149, build #1258) is dominated by
substrate/seeding/recurring-flake classes (see the substrate-gated ceiling item). But two
failures look like genuine in-repo code defects worth a dedicated pass **once the alpha
substrate is healthy enough to give a trustworthy measurement** (blocked today by adam
arc-saturation → alpha.elohim.host 503).

## Lead 1 — seeder create-not-upsert on existing content

- **Symptom:** adam conductor logs show `create_content` returning `Content with id
  'scenario-value-scanner-...' already exists. Use update_content to modify existing
  entries.` (content_store/src/lib.rs:2361) during seeding — on the exact value-scanner
  content the affinity/stewardship test scans.
- **Hypothesis:** if the seeder calls create (not upsert) and treats "already exists" as a
  benign skip *without* applying an update, then a re-seed with corrected reach-grade /
  multi-steward allocations never takes effect → stale rows → the manifesto **reach-floor
  403** (commons must beat community; storage 403s anon reads on the downgraded row) and
  the **affinity ×3** failures (no multi-steward allocation in the scanned window;
  documented recurring flake genesis #1104/#1105) both persist across re-seeds.
- **Next:** trace the seeder's create→already-exists path (genesis/seeder or elohim-import);
  confirm whether it upserts. If not, make it update_content on already-exists. Verify a
  fresh seed corrects the manifesto reach and value-scanner allocations. NOT verifiable
  while alpha is degraded.

## Lead 2 — non-admin access not denied (conductor-visibility)

- **Symptom:** API scenario "non-admin access not denied" fails — an authorization gate is
  too permissive (a genuine logic defect, unrelated to seeding/substrate).
- **Next:** locate the conductor-visibility authorization check (doorway/storage), confirm
  it denies non-admin. This is a security-sensitive change — verify carefully; needs a
  genesis run to confirm the scenario flips green.

## Why blocked tonight

Both need a genesis run against a HEALTHY alpha to get a trustworthy pass/fail delta. Alpha
is currently substrate-degraded (adam arc-saturation → alpha.elohim.host 503), so any
verification is confounded. Pick up on a substrate-healthy shift after the arc-factor
ceiling item lands.
