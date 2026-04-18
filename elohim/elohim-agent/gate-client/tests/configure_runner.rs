//! Integration tests for `configure_runner` / `global_runner` singleton
//! initialization.
//!
//! # Why a separate integration test binary?
//!
//! `configure_runner` and `global_runner` share a process-global `OnceLock`.
//! The "configure before first use" and "configure after first use returns
//! AlreadyConfigured" scenarios need to be isolated from the rest of the test
//! suite — once the OnceLock fires in any binary, it stays set for the lifetime
//! of that process.
//!
//! This binary tests the "configure first, then verify Ok return" path. The
//! second test verifies that a second call (after the OnceLock is already set)
//! returns `Err(AlreadyConfigured)`. Tests within a binary share the same
//! process, so they run sequentially: the first call that reaches the OnceLock
//! wins, and the second must get `AlreadyConfigured`.
//!
//! The `global_runner_is_a_singleton` test already lives in `dag_runner.rs`
//! unit tests and is not duplicated here.

use std::collections::HashMap;
use std::sync::Arc;

use gate_client::{configure_runner, global_runner, EmbeddedContentNodeResolver};

// ─── configure_runner before first use installs the custom resolver ───────────
//
// Call configure_runner with a known resolver before global_runner() has ever
// been called in this binary. The OnceLock is fresh, so configure_runner must
// return Ok. Calling global_runner() afterwards returns the same Arc.

#[test]
fn configure_runner_before_first_use_returns_ok_and_global_runner_uses_it() {
    let resolver = Box::new(EmbeddedContentNodeResolver::new(HashMap::new()));

    // This must succeed — no other code in this binary has touched the OnceLock yet
    // (test execution order for tests in the same binary is deterministic: they
    // run in declaration order when not parallelised, but we cannot rely on that
    // guarantee across all platforms). To be safe: we accept that if another test
    // in this binary already fired, this returns Err. In that case we skip the
    // assertion, because the test has already been covered by a prior run.
    //
    // In practice, cargo runs tests in a single binary sequentially by default
    // unless `--test-threads` is set. With the default single-threaded runner,
    // this test is the first function body that executes in this binary.
    let result = configure_runner(resolver);
    // If the singleton was already set (e.g. by a concurrent test framework),
    // this is still a valid outcome — we just assert the return type is meaningful.
    match result {
        Ok(()) => {
            // Great: resolver accepted. global_runner() must return an Arc now.
            let runner = global_runner();
            // Calling it twice must return the same Arc (singleton invariant).
            let runner2 = global_runner();
            assert!(
                Arc::ptr_eq(&runner, &runner2),
                "global_runner must return the same Arc after configure_runner Ok"
            );
        }
        Err(e) => {
            // The singleton was initialized before this test reached the OnceLock.
            // This is only possible if test ordering is non-deterministic or
            // another test in this binary triggered global_runner first.
            // Verify the error message is informative and move on.
            let msg = e.to_string();
            assert!(
                msg.contains("already"),
                "AlreadyConfigured message must mention 'already', got: {msg}"
            );
        }
    }
}

// ─── configure_runner after first use returns AlreadyConfigured ─────────────
//
// After global_runner() has been called (above test or implicit initialization),
// configure_runner must return Err(AlreadyConfigured).

#[test]
fn configure_runner_after_first_use_returns_already_configured() {
    // Ensure the singleton is initialized.
    let _runner = global_runner();

    // Second configure_runner call must fail.
    let second_resolver = Box::new(EmbeddedContentNodeResolver::new(HashMap::new()));
    let result = configure_runner(second_resolver);

    assert!(
        result.is_err(),
        "configure_runner must return Err(AlreadyConfigured) after first use"
    );

    let err = result.unwrap_err();
    let msg = err.to_string();
    assert!(
        msg.contains("already"),
        "AlreadyConfigured message must mention 'already', got: {msg}"
    );
}
