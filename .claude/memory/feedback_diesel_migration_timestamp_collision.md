---
name: Diesel migration timestamp collisions silently drop migrations
description: Two diesel migration directories with same YYYY-MM-DD-HHMMSS prefix cause embed_migrations! to silently keep only one
type: feedback
originSessionId: 53d58b9b-be66-4db2-bfb9-75f7f377aed9
---
When two directories under `elohim/elohim-storage/migrations/` share the same `YYYY-MM-DD-HHMMSS` prefix, `diesel::embed_migrations!()` orders them by directory name and silently keeps only one — the other is dropped at runtime. Symptom: `no such table: <name>` errors at runtime, but `cargo build` and `diesel migration run` both succeed.

Phase 11 hit this: M5 added `2026-04-25-010000_create_portal_hosts` and EPR 2B added `2026-04-25-010000_projector_cursor` in parallel branches. After both merged to dev, `portal_hosts` was silently dropped, breaking 9 tests (8 portal_hosts CRUD + 1 reconcile controller roundtrip). Resolution: rename one directory to a non-colliding timestamp (e.g., `015000`).

**Why:** parallel branches commonly land migrations on the same day. Default timestamps come from minute-resolution clocks; collisions are likely when two engineers (or sessions) work in the same hour.

**How to apply:**
- Pick distinct minute-level timestamps for new migrations even within the same hour. `0100`, `0130`, `0200` not all `0100`.
- After merging two branches with migrations from the same day, run `ls elohim/elohim-storage/migrations/ | sort | uniq -d -w 17` (count of files with same first 17 chars) to detect collisions before committing.
- If collision is found post-merge, rename ONE directory (`git mv old new`) — both `up.sql` and `down.sql` survive.
- Don't trust `cargo build` to catch this — the failure is runtime-only.
