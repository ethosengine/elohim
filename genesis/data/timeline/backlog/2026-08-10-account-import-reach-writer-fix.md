---
id: "backlog-account-import-reach-writer-fix"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Fix /account/import writing viewer-relative package assignments into peer-global content.reach (root cause of the 1,987 familiar-reach divergence) — writer fix only, no data repair"
slug: "account-import-reach-writer-fix"
written: "2026-08-10"
author: "batch-3 integration session (follow-up from familiar-reach archaeology)"
status: "resolved"
priority: "high"
tags: [dataplane, reach, account-import, bounded-code-fix, codex-claimable]
cites:
  - genesis/data/timeline/backlog/2026-08-10-familiar-reach-origin-archaeology.md
---

# Stop the writer first — /account/import poisons peer-global reach

Archaeology verdict (see cited item): the 1,987 familiar rows on matthew
exactly match Susan's familiar account-package assignments; `/account/import`
incorrectly writes VIEWER-RELATIVE assignments into the PEER-GLOBAL
`content.reach` column, creating last-writer-wins divergence across pods
(matthew=familiar vs adam=community).

Scope — the WRITER only:
1. Locate the `/account/import` path that mutates `content.reach` and stop it
   from writing viewer-relative reach into the global column (the assignment
   belongs on a viewer/agent-scoped surface, not on the content row).
2. Regression test proven red-on-old-code: an import carrying a familiar
   package assignment must leave a community row's `content.reach` untouched.
3. Do NOT attempt the data repair here — re-projection vs governed canonical
   re-declaration is the archaeology item's recommendation thread and stays a
   separate, operator-visible decision.

Constraints: disjoint from head_adoption.rs / contest_backoff.rs (in-flight
elsewhere). Acceptance: full `cargo test` on elohim-storage green + the new
red-first test; fmt/clippy clean on touched files.

## Resolution — 2026-08-10

`POST /account/import` now treats `package.content` as viewer-relative package
metadata and does not write it into peer-global `content.reach`. The existing
`contentUpdated` response field remains wire-compatible and reports `0`; the
other import phases are unchanged.

The handler-level regression
`account_import_keeps_peer_global_content_reach_unchanged` seeds a `community`
row, imports a `familiar` assignment, and reads the row back through the normal
storage path. It failed on the old writer with `familiar != community`, then
passed after the writer was removed.

Evidence:

- red-first targeted test: failed on the old writer at the reach assertion;
- fixed targeted test: `1 passed; 0 failed`;
- `cargo fmt --check`: green;
- `cargo clippy -- -D warnings`: green;
- full unit and integration inventory via `cargo test --tests --quiet`: exit 0
  (`2626 passed; 2 ignored` in the library target, all subsequent test targets
  green).

No existing content rows were changed and no data-repair path was added. The
separate governed repair decision remains with the archaeology follow-up.
