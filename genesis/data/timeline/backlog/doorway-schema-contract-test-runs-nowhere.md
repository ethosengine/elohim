---
id: "backlog-doorway-schema-contract-test-runs-nowhere"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "doorway-service's integration test tests/schema_contract.rs (incl. the AuthDiscovery view contract) runs in NO lane — the justfile gate and the CI check target both run `cargo test --lib --bins`"
slug: "doorway-schema-contract-test-runs-nowhere"
written: "2026-08-29"
author: "portal-lane mapping"
status: "open"
priority: "medium"
jobs: [elohim-edge]
cluster: "arch-dataplane-refactor-backlog"
tags: [doorway, gate, schema-contract, auth-discovery, ci]
---

`doorway/doorway-service/justfile` `gate: fmt-check clippy test` → `cargo test --lib --bins`; the CI
`Quality Gate: Doorway` builds the Dockerfile `check` target with the same flags (non-blocking, UNSTABLE).
`tests/schema_contract.rs:636` (AuthDiscovery view ↔ `elohim/sdk/schemas/v1/views/`) is therefore never
executed. Cure: `cargo test --lib --bins --tests` in both places (verify the integration tests need no
running conductor; if any do, gate them behind a feature). Companion: the root App Jenkinsfile
"E2E Testing - Alpha Validation" stage is a Cypress ghost (no cypress dir/dep, catchError, prints ✅) —
ratchet rung D3.
