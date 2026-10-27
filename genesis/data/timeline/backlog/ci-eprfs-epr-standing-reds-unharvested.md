---
id: "backlog-ci-eprfs-epr-standing-reds-unharvested"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "elohim-eprfs and elohim-epr on dev have been red for days, and ci-harvest cannot see why"
slug: "ci-eprfs-epr-standing-reds-unharvested"
written: "2026-10-27"
author: "claude-opus-5-5"
status: "open"
priority: "high"
area: "ci / eprfs / epr / ci-harvest"
domain: "D5"
ci_status: "triaged"
relatedNodeIds:
  - "habit:devices-share-one-berth"
tags: [ci, eprfs, epr, ci-harvest, environment]
---

# eprfs and epr: standing reds, unharvested

`elohim-eprfs/dev` has been FAILURE since at least #72 (#76 aborted), and `elohim-epr/dev` since #50.
`.claude/data/ci-findings.jsonl` holds no fingerprint for any of the failing tests. eprfs publishes
no JUnit (`getTestResults` returns empty), so ci-harvest has nothing to fingerprint and the reds
have no owner.

Read from `elohim-eprfs/dev #77` (dev fd1620978), the 2026-10-27 overnight shift:

| Test | Cause | State |
|---|---|---|
| `bin_store::tests::brit_build_tries_offline_then_anonymous_then_credentials`, `explicit_offline_mode_never_attempts_network_build_or_download` | `build_target` reads the host's `CARGO_TARGET_DIR`. CI exports none, and the fixture's fake runner had no answer for `cargo-pool key`, so the build refused before any `cargo build` (left 0) | fixed on dev by f13cbdd33 (9d1f0e8cd), which superseded ba22674a7 |
| `session_envelope_native_facets_and_hook_pin_regressions` (3 Python cases) | CI devShell lacks PyYAML (`capability-tier-gate: skipped`) and `just` (exit 127) | devShell fix on dev by f13cbdd33 (9d1f0e8cd); confirm on the next eprfs run |
| `judge_needs_no_session_at_all` (`epr-cli/tests/flow_memory_recall_sample.rs:282`) | the judge refused `invalid arguments: session-invalid: ` (empty session id), exit 2, expected 0. Likely an empty `ELOHIM_SESSION_ID` or a session resolver reading CI's environment | fixed on dev by f13cbdd33 (an empty --session stays None) |
| `elohim-epr/dev #53` | `script returned exit code 1` after the TS tests stage. The observer inferred the stage from log order with low confidence | addressed on dev by the elohim-epr safe.directory fix (9d1f0e8cd); confirm on the next epr run |

## Done when

- Both jobs are green on dev for 3 consecutive builds.
- eprfs (and epr) emit JUnit, or ci-harvest reads cargo's `test … FAILED` lines, so the next standing
  red is fingerprinted on its first build instead of being found by hand.
