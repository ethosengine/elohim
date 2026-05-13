---
name: Env vars in tests need a static mutex (or refactor)
description: Process-global env var read on the hot path + one test that mutates it = parallel-test flake. Saw it in storage_proxy.rs BLOB_PANTRY_MAX_BYTES.
type: feedback
originSessionId: 872c2e1c-02fe-453a-93b3-e69dac1e54e3
---
When a function reads `std::env::var("X")` at call time and one test does `std::env::set_var("X", ...)`, parallel cargo tests racing against the same function fail intermittently. An 18-byte payload fails the "should cache" assertion because the oversized-blob test set the limit to 10 bytes a moment earlier.

**Why:** Env vars are process-global. `cargo test` runs tests on multiple threads in the same process by default. `set_var` in test A leaks into test B's read.

**How to apply:**
- If you're tempted to read env in a function that has unit tests: prefer threading the value as a parameter, or read once into a `OnceLock` at startup.
- If env-on-the-hot-path is already there and you can't refactor: add a `static MUTEX: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(())` in the test module and have every test that touches the function `lock().await` it. Cheap, robust, no new dependencies.
- Watch for symptoms: a test that passes alone but fails in the full suite, with an assertion that doesn't match the data the test set up. The miscompiled value comes from another test's env mutation.

Saw 2026-04-30 in `doorway/doorway-service/src/routes/storage_proxy.rs` (BLOB_PANTRY_MAX_BYTES). Fix shipped as a `BLOB_TEST_LOCK` static mutex around the 5 affected tests.
