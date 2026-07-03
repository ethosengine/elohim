//! Runtime-globals shim: `process`, timers, and the `global` alias.
//!
//! The Angular server bundle is Node-targeted. Its remaining walls in a bare
//! deno_core runtime (after module loading + node-builtin linking) are RUNTIME
//! GLOBALS, not modules:
//!
//! - `process` -- zone.js touches `process.nextTick` at polyfills eval time, and
//!   the bundle selects the "direct" SSR render path off `process.versions.node`.
//! - Timers -- `setTimeout` / `setInterval` / `setImmediate` (and their clears)
//!   live in Deno's WEB extension, not in `deno_core`, so a bare runtime has none.
//!   Angular's NgZone stability tracking and RxJS scheduling need them.
//! - `global` -- Node's alias of the global object; the bundle assigns `E = global`
//!   once it detects Node.
//!
//! The JS half ([`node_globals.js`]) builds these on top of deno_core's built-in
//! timer/microtask facility (`Deno.core.queueUserTimer` / `queueImmediate` /
//! `cancelTimer` / `refTimer` / `unrefTimer`), which the default event loop already
//! dispatches via `01_core.js`'s `eventLoopTick`. The only Rust op this module adds
//! is [`op_elohim_v8_version`], which backs `process.versions.v8` with the REAL
//! linked V8 version rather than a fabricated one.
//!
//! Surface discipline mirrors [`crate::shim::node_builtins`]: implement only what a
//! proven render path needs, keep values honest (sentinel where there is no real
//! system fact), and prefer absence (`undefined`) over a fabricated value. The one
//! deliberate DIVERGENCE from node_builtins' loud-stub default: unknown `process`
//! properties read as `undefined`, NOT loud stubs. The bundle feature-detects
//! `process.send` / `.type` / `.browser` / `.__nwjs`; a truthy loud function there
//! would mis-route it into child-process-IPC / electron / nw.js code paths. For
//! `process`, absence is the honest and safe signal -- Node's own behavior for
//! properties it does not set.
//!
//! # WARNING: pure ASCII
//!
//! `node_globals.js` is served through the `js = [dir ...]` macro, which panics at
//! const-eval on any non-ASCII byte. Keep every literal ASCII.

use std::sync::OnceLock;
use std::time::Instant;

use deno_core::op2;

/// The real version of the V8 engine linked into this runtime.
///
/// Backs the JS shim's `process.versions.v8`. `V8::get_version()` returns a
/// compile-time constant string from the linked engine (no isolate needed), so
/// this is maximally honest -- unlike `process.versions.node`, which is a sentinel
/// (there is no real Node here).
#[op2]
#[string]
fn op_elohim_v8_version() -> String {
    deno_core::v8::V8::get_version().to_string()
}

/// The monotonic time origin for `performance.now()`.
///
/// Captured once, the first time any render reads the clock, and shared across
/// renders. `performance.now()` differences (durations) are what callers measure,
/// so a process-global origin is correct; only the absolute value is shared, which
/// is fine for SSR timing/profiling. `Instant` is monotonic (never goes backward),
/// unlike a `Date.now()`-based shim.
fn perf_origin() -> Instant {
    static ORIGIN: OnceLock<Instant> = OnceLock::new();
    *ORIGIN.get_or_init(Instant::now)
}

/// High-resolution monotonic milliseconds since the runtime's time origin.
///
/// Backs the JS shim's `performance.now()`. Sub-millisecond precision via
/// `as_secs_f64`, honest and monotonic (an `Instant` delta), so RxJS/zone.js
/// duration measurements are real rather than wall-clock approximations.
#[op2(fast)]
fn op_elohim_perf_now() -> f64 {
    perf_origin().elapsed().as_secs_f64() * 1000.0
}

deno_core::extension!(
    node_globals_ext,
    ops = [op_elohim_v8_version, op_elohim_perf_now],
    js = [dir "src/shim", "node_globals.js"],
);

#[cfg(test)]
mod tests {
    /// The V8 version string the op reports must be a non-empty, dotted version
    /// (e.g. "13.4.114.9"). Guards against the op backing `process.versions.v8`
    /// with an empty/garbage value that would make a UA-string template read as
    /// `v8/`.
    #[test]
    fn v8_version_is_a_nonempty_dotted_version() {
        let v = deno_core::v8::V8::get_version();
        assert!(!v.is_empty(), "V8 version must be non-empty");
        assert!(
            v.contains('.') && v.chars().any(|c| c.is_ascii_digit()),
            "V8 version looks like a dotted version: {v}"
        );
    }
}
