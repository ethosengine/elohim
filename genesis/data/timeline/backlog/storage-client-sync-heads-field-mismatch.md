---
id: "backlog-storage-client-sync-heads-field-mismatch"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "storage-client-ts reads `new_heads`; storage sends `newHeads` — the first sync re-sends the whole document"
slug: "storage-client-sync-heads-field-mismatch"
written: "2026-10-04"
author: "claude (local-first design pass, 2026-10-04)"
status: "resolved"
resolved: "2026-10-04"
priority: "medium"
tags: [storage-client-ts, automerge, sync, wire-format, camelcase, perf-bytes]
cites:
  - elohim/sdk/storage-client-ts/src/sync.ts
  - elohim/sdk/storage-client-ts/src/types.ts
  - elohim/elohim-storage/src/http.rs
  - "working-version-sdk-standard-design | The working-version standard | sha256:b323e209c972831d | path: genesis/docs/superpowers/specs/2026-10-04-working-version-sdk-standard-design.md"
---

# storage-client-ts reads `new_heads`; storage sends `newHeads`

## Symptom (verified by reading source, 2026-10-04; not run)

`AutomergeSync` stores the heads it last saw from `response.new_heads`
(`elohim/sdk/storage-client-ts/src/sync.ts:97,132,134`), typed in `types.ts:82,94`. The storage
handlers answer with `"newHeads"` (`elohim/elohim-storage/src/http.rs:6994,7053`). The same types
file declares `doc_id`, `doc_type`, `change_count` and `last_modified` where the server is
reported to send `docId`, `docType`, `changeCount` and `lastModified`.

## Consequence

After `load()` or `save()` the known heads are undefined and fall back to empty, so the first
`sync()` sends every change in the document instead of the changes since the last exchange. The
one consumer today only reads, so the cost is hidden. Any client that writes pays it on every
first sync.

## Fix shape

The types are hand-written snake_case against a camelCase wire, which the Rust-to-TypeScript rule
forbids. Generate these response types from the Rust side as the other views are, and add a test
that round-trips a save and asserts the next sync sends nothing.

## Resolved 2026-10-04

- TypeScript: the response types and `AutomergeSync` now read the camelCase wire names.
  `tests/sync-heads-wire.test.ts` fails on the old name (107 bytes re-sent where 0 were expected)
  and passes with the fix; the package's tests and typecheck pass.
- Rust: `crates/elohim-storage-client/src/types.rs` had the same defect and worse. It declared
  `app_id`, `doc_id` and `new_heads` with no rename, so it could not parse a reply at all. The six
  response types now carry `rename_all = "camelCase"` with `hAppId` for the app id, and three
  tests parse the server's shapes.
- Still open: these types are hand-written on both sides. Generating them from one source is the
  durable fix.

## Why it mattered

The working-version standard makes the client a writer. This is listed in its §9.
