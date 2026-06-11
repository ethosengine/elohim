---
id: "backlog-dna-mishpat-integrity-defense-in-depth-coverage"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Mishpat integrity defense-in-depth: acknowledges-reach-change arm missing + delegates-compute sweettest coverage"
slug: "dna-mishpat-integrity-defense-in-depth-coverage"
written: "2026-06-11"
author: "agentic-developer (EPR durability shift, coherence now-lane)"
status: "backlog"
priority: "medium"
jobs: [elohim-holochain]
tags: [dna, mishpat, integrity, commitments, sweettest, defense-in-depth]
cites:
  - elohim/holochain/dna/mishpat/zomes/mishpat_integrity/src/lib.rs
  - elohim/holochain/dna/mishpat/zomes/mishpat/src/commitments.rs
---

# Mishpat integrity defense-in-depth follow-ups

Context: the `delegates-compute` integrity arm landed (N5, this shift) —
`validate_commitment_entry` now guards the direct-source-chain bypass path
for delegates-compute payloads (substring-style, serde_json is dev-only in
the integrity zome; the coordinator does full validation). Two follow-ups:

1. **`acknowledges-reach-change` lacks the same integrity arm.** The
   coordinator validates it, but a direct `create_entry` bypass would pass
   the generic `starts_with('{')` check. Mirror the delegates-compute arm
   (same substring pattern + native round-trip tests).

2. **Sweettest for the delegates-compute arm** (zome-sweettest-sync rule):
   native tests prove the validator in isolation, not that it guards the
   DHT. Add to `elohim/holochain/tests/sweettest/`: (a) well-formed
   delegates-compute Commitment via `create_commitment` on conductor A
   gossips to B (read back via `record.entry().to_app_option::<Commitment>()`);
   (b) direct `create_entry` bypass of a malformed payload (empty recipient
   or bounds missing rate_per_hour) is REJECTED by B's validation — the only
   proof the arm guards the DHT. Use `two_agent_conductors` +
   `exchange_peer_info` + `await_consistency`. `#[ignore]` is fine locally
   but CI runs `--run-ignored all` — it must be a real passing test.

shift_objective: |
  Close both integrity defense-in-depth gaps: land the
  acknowledges-reach-change arm with native round-trip tests, and the
  delegates-compute two-conductor sweettest proving DHT-path rejection of
  malformed bounds.
