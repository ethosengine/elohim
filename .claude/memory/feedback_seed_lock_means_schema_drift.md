---
name: "Seed-time 'database is locked' often means schema drift, not concurrency"
description: When elohim-genesis Seed Database stage hits 'database is locked' errors clustered in specific content namespaces, the fix is usually to clear the database in the genesis stage, not patch SQLite pragmas or retry logic
type: feedback
originSessionId: cdffa1f9-7b63-4657-ae44-2cafff5156bf
---
When the Seed Database stage on elohim-genesis fails with `database is locked` errors, especially with errors clustered in specific content namespaces and shifting between builds, the underlying cause is usually **schema drift between the persisted DB and the current code's expectations**, not SQLite concurrency.

**Why:** Pragmas already include `journal_mode=WAL` + `busy_timeout=30000` (commit `0d72705f`, `elohim/elohim-storage/src/db/mod.rs:166-186`). User confirmed the operational pattern: when this happens, the fix is to clear the database in the genesis stage rather than chase pragmas or seeder-retry logic. A fresh DB applies all migrations cleanly and old rows don't carry stale shape that conflicts with new constraints/triggers.

**Signals that point at schema drift (vs real concurrency):**
- Errors clustered in a single content namespace within one build (e.g. all 100 errors in `preteen-scenarios-family-*`)
- The clustered namespace **shifts between builds** (caregiver-* in #954, preteen-* in #955)
- One anomalous batch with `0 inserted, 0 skipped` while neighbors succeed
- Existing data was already there (high `skipped` count), suggesting prior-run residue
- Pragmas already in place — pure concurrency would be more uniform

**How to apply:**
- First-pass fix lives in the genesis pipeline: clear the SQLite DB at the start of the Seed Database stage (or its predecessor). Do NOT default to patching elohim-storage pragmas or adding seeder retry logic.
- Investigate retry/concurrency paths only after a clean run reproduces the failure.
- Differentiate from legitimate concurrency: would show uniform/random errors across namespaces, would not correlate with deploy timing or schema changes.
