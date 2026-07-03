//! Behavior tests for the runtime-globals shim (`process`, timers, `global`).
//!
//! These drive real V8 through the same `with_shims` runtime the production path
//! uses, so they exercise the injected globals end-to-end (ESM/op wiring + the
//! deno_core event loop that dispatches fired timers), not just the JS in
//! isolation.

use elohim_render::runtime::JsRuntime;

/// `process.nextTick` is queueMicrotask-backed: its callback runs AFTER the
/// current synchronous frame, on the microtask queue, in FIFO order with respect
/// to a Promise job queued after it. This pins the documented fidelity bound.
#[tokio::test]
async fn process_nexttick_runs_as_microtask_after_sync() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(async () => {
                const order = [];
                process.nextTick(() => order.push("nextTick"));
                Promise.resolve().then(() => order.push("promise"));
                order.push("sync");
                // Drain the microtask queue.
                await Promise.resolve();
                await Promise.resolve();
                return order.join(",");
            })()"#,
        )
        .await
        .expect("nextTick ordering script runs");
    // sync is synchronous (first); nextTick was queued before the Promise job, so
    // it drains first among the microtasks.
    assert_eq!(
        out, "sync,nextTick,promise",
        "nextTick defers to the microtask checkpoint, FIFO with promises"
    );
}

/// `process.nextTick(cb, ...args)` forwards extra arguments (Node semantics).
#[tokio::test]
async fn process_nexttick_forwards_args() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(async () => {
                let seen = null;
                process.nextTick((a, b) => { seen = a + ":" + b; }, "x", "y");
                await Promise.resolve();
                return String(seen);
            })()"#,
        )
        .await
        .expect("nextTick args script runs");
    assert_eq!(
        out, "x:y",
        "nextTick forwards trailing args to the callback"
    );
}

/// `process.env` is safe to read for unset keys (undefined, never throws), is
/// curated to NODE_ENV=production, and is a mutable plain object (assign/delete).
#[tokio::test]
async fn process_env_access_is_safe_and_curated_and_mutable() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const before = process.env.SOME_UNSET_VAR; // must be undefined, no throw
                process.env.ELOHIM_TEST = "bar";
                const set = process.env.ELOHIM_TEST;
                delete process.env.ELOHIM_TEST;
                const after = process.env.ELOHIM_TEST;
                return JSON.stringify({
                    before: before === undefined,
                    nodeEnv: process.env.NODE_ENV,
                    set,
                    after: after === undefined,
                });
            })()"#,
        )
        .await
        .expect("env access script runs without throwing");
    assert!(
        out.contains("\"before\":true"),
        "unset key reads undefined: {out}"
    );
    assert!(
        out.contains("\"nodeEnv\":\"production\""),
        "NODE_ENV curated: {out}"
    );
    assert!(out.contains("\"set\":\"bar\""), "env is assignable: {out}");
    assert!(out.contains("\"after\":true"), "env supports delete: {out}");
}

/// The load-bearing guarantee: runtime-detection properties the bundle
/// feature-detects (`send` / `type` / `browser` / `__nwjs`) read as `undefined`,
/// NOT as truthy loud stubs -- a truthy value there mis-routes the bundle into
/// child-process-IPC / electron / nw.js paths.
#[tokio::test]
async fn feature_detect_absent_process_members_are_undefined() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => JSON.stringify({
                send: typeof process.send,
                type: typeof process.type,
                browser: typeof process.browser,
                nwjs: typeof process.__nwjs,
            }))()"#,
        )
        .await
        .expect("feature-detect script runs");
    assert_eq!(
        out, r#"{"send":"undefined","type":"undefined","browser":"undefined","nwjs":"undefined"}"#,
        "absent process members must be undefined, not loud stubs: {out}"
    );
}

/// `process.versions.node` is a valid dotted version the bundle can `split(".")`
/// and read as "this is Node", and `process.versions.v8` is a real, non-empty
/// version string.
#[tokio::test]
async fn process_versions_node_is_parseable_and_v8_is_real() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                const v = process.versions;
                const parts = v.node.split(".").map(Number);
                return JSON.stringify({
                    nodeDefined: v.node !== undefined,
                    nodeParses: parts.length >= 2 && parts.every((n) => !Number.isNaN(n)),
                    v8Ok: typeof v.v8 === "string" && v.v8.length > 0,
                    platform: process.platform,
                });
            })()"#,
        )
        .await
        .expect("versions script runs");
    assert!(
        out.contains("\"nodeDefined\":true"),
        "versions.node defined: {out}"
    );
    assert!(
        out.contains("\"nodeParses\":true"),
        "versions.node is dotted: {out}"
    );
    assert!(
        out.contains("\"v8Ok\":true"),
        "versions.v8 is a real string: {out}"
    );
    assert!(
        out.contains("\"platform\":\"linux\""),
        "platform is honest host OS: {out}"
    );
}

/// A setTimeout(0) fires when the event loop is driven, and clearTimeout on the
/// returned handle prevents a pending timer from firing -- the set/clear roundtrip.
#[tokio::test]
async fn timer_set_fires_and_clear_prevents_fire() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(async () => {
                let firedA = false;
                let firedB = false;
                setTimeout(() => { firedA = true; }, 0);          // should fire
                const h = setTimeout(() => { firedB = true; }, 5); // cleared below
                clearTimeout(h);
                // Await a real timer so the event loop drains pending timers.
                await new Promise((res) => setTimeout(res, 20));
                return JSON.stringify({ firedA, firedB });
            })()"#,
        )
        .await
        .expect("timer roundtrip script runs");
    assert_eq!(
        out, r#"{"firedA":true,"firedB":false}"#,
        "setTimeout(0) fires; a cleared timer does not: {out}"
    );
}

/// setTimeout forwards trailing args to the callback (Node semantics), since
/// deno_core invokes the stored callback with no arguments.
#[tokio::test]
async fn set_timeout_forwards_args() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(async () => {
                let seen = null;
                setTimeout((a, b) => { seen = a + b; }, 0, 2, 3);
                await new Promise((res) => setTimeout(res, 15));
                return String(seen);
            })()"#,
        )
        .await
        .expect("timer args script runs");
    assert_eq!(out, "5", "setTimeout forwards extra args to the callback");
}

/// setInterval fires repeatedly and clearInterval stops it; the returned handle
/// supports Node's .unref()/.refresh() without throwing (the bundle calls these).
#[tokio::test]
async fn interval_fires_repeatedly_and_handle_supports_unref() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(async () => {
                let count = 0;
                const h = setInterval(() => { count++; }, 3);
                const isHandle =
                    typeof h.unref === "function" &&
                    typeof h.refresh === "function" &&
                    h.unref() === h; // unref returns the handle, does not throw
                await new Promise((res) => setTimeout(res, 25));
                clearInterval(h);
                const afterClear = count;
                await new Promise((res) => setTimeout(res, 15));
                // No further increments after clearInterval.
                return JSON.stringify({
                    fired: count > 0,
                    isHandle,
                    stable: count === afterClear,
                });
            })()"#,
        )
        .await
        .expect("interval script runs");
    assert!(
        out.contains("\"fired\":true"),
        "interval fires at least once: {out}"
    );
    assert!(
        out.contains("\"isHandle\":true"),
        "handle has ref/unref/refresh: {out}"
    );
    assert!(
        out.contains("\"stable\":true"),
        "clearInterval stops further fires: {out}"
    );
}

/// setImmediate fires on the next macrotask tick and clearImmediate cancels it.
#[tokio::test]
async fn set_immediate_fires_and_clears() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(async () => {
                let a = false;
                let b = false;
                setImmediate(() => { a = true; });
                const h = setImmediate(() => { b = true; });
                clearImmediate(h);
                await new Promise((res) => setTimeout(res, 15));
                return JSON.stringify({ a, b });
            })()"#,
        )
        .await
        .expect("setImmediate script runs");
    assert_eq!(
        out, r#"{"a":true,"b":false}"#,
        "setImmediate fires; a cleared immediate does not: {out}"
    );
}

/// `performance.now()` is monotonic (never decreases) and op-backed -- the clock
/// RxJS schedulers and zone.js timing read.
#[tokio::test]
async fn performance_now_is_monotonic() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(async () => {
                const a = performance.now();
                await new Promise((res) => setTimeout(res, 5));
                const b = performance.now();
                return JSON.stringify({
                    numbers: typeof a === "number" && typeof b === "number",
                    monotonic: b >= a,
                    advanced: b > a,
                });
            })()"#,
        )
        .await
        .expect("performance.now script runs");
    assert!(
        out.contains("\"numbers\":true"),
        "now() returns numbers: {out}"
    );
    assert!(
        out.contains("\"monotonic\":true"),
        "now() is monotonic: {out}"
    );
    assert!(
        out.contains("\"advanced\":true"),
        "now() advances over a 5ms wait: {out}"
    );
}

/// The User Timing API is a real store, not a no-op: a mark/measure roundtrip
/// yields a shaped entry via getEntriesByName, so the bundle's
/// `getEntriesByName(name)[0].duration` deref does not throw.
#[tokio::test]
async fn performance_user_timing_mark_measure_roundtrip() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => {
                performance.mark("start");
                performance.mark("end");
                performance.measure("span", "start", "end");
                const entry = performance.getEntriesByName("span")[0];
                const hasEntry = entry !== undefined && entry.entryType === "measure";
                const durationOk = typeof entry.duration === "number" && entry.duration >= 0;
                performance.clearMarks();
                performance.clearMeasures();
                const clearedOk = performance.getEntriesByName("span").length === 0;
                return JSON.stringify({ hasEntry, durationOk, clearedOk });
            })()"#,
        )
        .await
        .expect("user timing script runs");
    assert!(
        out.contains("\"hasEntry\":true"),
        "measure yields a shaped entry: {out}"
    );
    assert!(
        out.contains("\"durationOk\":true"),
        "entry.duration is a number: {out}"
    );
    assert!(
        out.contains("\"clearedOk\":true"),
        "clear* empties the store: {out}"
    );
}

/// `global` is an alias of the global object, carrying `process` -- the binding
/// the bundle assigns via `E = global` on the Node-detection path.
#[tokio::test]
async fn global_aliases_globalthis() {
    let mut rt = JsRuntime::with_shims();
    let out = rt
        .eval_string(
            r#"(() => String(
                global === globalThis && typeof global.process === "object"
            ))()"#,
        )
        .await
        .expect("global alias script runs");
    assert_eq!(out, "true", "global aliases globalThis and carries process");
}
