---
id: "backlog-dna-bridge-role-name-conformance"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Cross-DNA bridge role names are unverified against the happ manifest — OtherRole(\"elohim\") shipped dead in 12 sites"
slug: "dna-bridge-role-name-conformance"
written: "2026-06-12"
author: "delivery-trace session (EPR durability arc, wave-7)"
status: "backlog"
priority: "high"
jobs: [elohim-holochain]
tags: [dna, sweettest, cross-dna-bridge, role-names, conformance]
cites:
  - genesis/data/timeline/backlog/dna-health-attestation-ci-authz.md
  - genesis/a2o/features/delivery/happ-coordinator-delivery.feature
---

# Cross-DNA bridge role-name conformance guard

12 `CallTargetCell::OtherRole("elohim")` sites (imagodei ×8, infrastructure
×1, mishpat ×3) shipped targeting a role that does not exist in the happ
manifest — the consolidated content_store DNA packs as `lamad.dna` under
role `lamad`. Every bridge failed at runtime with
`Host("Role not found: elohim")`; the doorway-attestation lockout guard
masked the infrastructure site until 2026-06-12, when the coordinator
hot-swap delivered the re-registration fix and the next link surfaced.

**Why no test caught it:** attestation/governance sweettests install
SINGLE-DNA apps (`setup_app_for_agent("lamad-app", …, &[dna])`) and call
`content_store` functions directly — the cross-DNA `call` path is never
exercised, and role names in single-DNA test apps don't match the happ
manifest anyway.

**Fixed sites (2026-06-12):** all 12 now use a per-crate
`const LAMAD_ROLE: &str = "lamad"` (idiom mirrors content_store's
`IMAGODEI_ROLE`).

**Remaining work (this entry):**
1. **Conformance guard** — a non-`#[ignore]` sweettest-crate unit test (no
   conductor needed) that parses `dna/elohim/workdir/happ.yaml` role names
   and greps coordinator zome sources for `OtherRole("…")` string literals
   + role constants, asserting every target is a declared role. Pure
   fs/regex; would have caught all 12 sites at build time.
2. **One real bridge sweettest** — install infrastructure+lamad in ONE app
   with manifest-matching role names; register a doorway; drive
   `record_health_attestation` end-to-end to a `content_attestations`
   read-back. Guards the whole consolidation seam, not just the role name.
3. Constraint story already scaffolded:
   `genesis/a2o/features/delivery/happ-coordinator-delivery.feature`.
