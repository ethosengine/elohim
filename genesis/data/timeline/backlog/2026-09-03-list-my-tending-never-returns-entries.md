---
id: "backlog-list-my-tending-never-returns-entries"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "list_my_tending queries the chain without include_entries — the list is always empty (CI-invisible: tests #[ignore] + quarantined)"
slug: "list-my-tending-never-returns-entries"
written: "2026-09-03"
author: "holochain 0.7 upgrade, Lane B (found while running sweettest --include-ignored)"
status: "open"
priority: "medium"
tags: [dna, content_store, attention-tending, coordinator-only, hash-neutral, sweettest, codex-claimable]
cites:
  - genesis/docs/superpowers/plans/2026-09-02-holochain-0-7-upgrade-guide.md
---

# `list_my_tending` never returns entries (pre-existing, not 0.7)

**Site:** `elohim/holochain/dna/elohim/zomes/content_store/src/attention_tending.rs:~236` —
`ChainQueryFilter::new().entry_type(UnitEntryTypes::AttentionTending…)` then
`record.entry().to_app_option::<AttentionTending>()`. `include_entries` defaults to `false`
(`holochain_zome_types` 0.6.0:254, 0.6.3:254, 0.7.0:253) and the conductor honours it
(`holochain_state/src/source_chain.rs`), so every record's entry is `NotStored` and the
`if let Ok(Some(..))` drops all of them. The list has been empty on every line we have run.

**Why CI never saw it:** the four tests in `tests/sweettest/src/tests/attention_tending.rs` are
`#[ignore]` and excluded by `tests/sweettest/scripts/build-nextest-filter.sh`; the last dev build ran
only `attention_tending_via_route`. Locally under `--include-ignored` (2026-09-03, holochain 0.7.0):
`create_and_list_succeeds` and `refresh_ttl_appends_timestamp` FAIL (`expected 1 record … left: 0`);
the two that pass do so *because* the list is empty.

**Fix (coordinator-only, DNA hash unchanged):** `ChainQueryFilter::new().entry_type(..).include_entries(true)`.
Then un-quarantine the four tests in the nextest filter (or move them to the route-level file's
scope) so the behaviour is CI-observed. Land AFTER the 0.7 family move is on `dev` — it is a
behaviour fix and does not belong inside the mechanical migration commit.

**DoD:** the four `attention_tending` tests pass locally with `--include-ignored`; they are no longer
quarantined; `hc dna hash` for the elohim DNA is unchanged by the commit (coordinator-only).
