---
id: artifact-store-brief-b2-seeder-deferred-retry
status: active
---

# Brief B2 — the seeder retries a "deferred" 400 with backoff (Codex sol + Sonnet test)

Worktree `as-b2`, branch `artifact-store/b2`. Read `common.md`.

**Problem (evidence):** a household prologue's MUST-SUCCEED leg failed on one write: HTTP 400 `REA projection changed during authority read; deferred` for the elohim-host-landing projection commitment; the retry an hour later succeeded (regenesis-1 `STATE.md` 11:45Z and 12:05Z).

**Write-set:** `genesis/seeder/src/seed-commitments.ts` and the client it calls, plus a unit test beside the seeder's existing tests.

**Deliverables:** a 400 whose body contains `deferred` is retried up to 3 times with 1 s, 2 s, 4 s backoff, each attempt logged; any other 400 stays fatal at once; the final failure names the attempts. Locate the exact storage message first (`grep -rn "deferred" elohim/elohim-storage/src`) so the match is on the real text.

**Tests:** a mocked client returning deferred-400 twice then 200 succeeds; deferred-400 four times fails naming 4 attempts; a non-deferred 400 fails on the first attempt.

**Gate:** `just gate genesis`. **Reviewer:** Sonnet reads the diff and reruns the test.
