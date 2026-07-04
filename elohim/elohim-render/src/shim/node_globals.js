// elohim-render runtime globals: timers, `process`, `performance`, and `global`.
//
// The Angular server bundle is Node-targeted. Two facts force this shim:
//   1. zone.js touches `process.nextTick` at polyfills eval time, and the bundle
//      branches on `process.versions.node !== undefined` to select the "direct"
//      SSR render path (assigning `E = global`) -- so `process` and `global` must
//      exist and read as a Node-compatible environment.
//   2. The bundle uses setTimeout/setInterval/setImmediate/queueMicrotask heavily
//      (Angular's NgZone stability, RxJS scheduling). A bare deno_core runtime has
//      NONE of the timer globals (they live in Deno's web extension, not core).
//
// This is deliberately minimal, backed by deno_core's built-in timer/microtask
// facility -- `Deno.core.queueUserTimer` / `queueImmediate` / `cancelTimer` /
// `refTimer` / `unrefTimer`, whose fired callbacks the default event loop already
// dispatches via 01_core.js's `eventLoopTick`. `queueMicrotask` is ALREADY a core
// global (with error reporting), so we do not re-define it.
//
// Fidelity bounds (documented, never hidden):
//   - Timers are REAL-TIME: deno_core's WebTimers fire off the wall clock, so
//     setTimeout(fn, N) waits ~N ms of real time (bounded above by the render
//     wall-time budget in AngularRenderer). setTimeout(fn, 0) / setImmediate fire
//     on the next macrotask tick. There is no time fast-forwarding.
//   - process.nextTick is queueMicrotask-backed: it defers to the microtask
//     checkpoint but does NOT preempt Promise-job ordering the way Node's separate
//     nextTick queue does. Sufficient for zone.js's "defer past current sync" need.
//   - setImmediate is a delay-0 macrotask; Node's subtle immediate-vs-timeout
//     ordering is not preserved.
//   - Timer handles are Node-style objects (ref/unref/hasRef/refresh); the bundle
//     calls .unref()/.refresh() on them, so a raw numeric id would break. Symbol
//     .toPrimitive yields the numeric id for coercion / cross-realm clears.
//   - process.versions.node is a SENTINEL Node-compat version (the host emulates a
//     Node 20-class environment); process.versions.v8 is the REAL linked V8 version.
//     process.platform is the honest host OS. process.env is curated to
//     { NODE_ENV: "production" } (this IS a production build). Unknown process
//     properties read as `undefined` -- Node's own behavior for absent props, and
//     deliberately NOT loud stubs: the bundle feature-detects process.send / .type
//     / .browser / .__nwjs, and a truthy loud function would mis-route it into
//     child-process-IPC / electron / nw.js paths. Absence is the honest signal.
((globalThis) => {
  const core = Deno.core;

  // --- Timers -------------------------------------------------------------
  // deno_core stores the timer callback and eventLoopTick invokes it as
  // `f.call(globalThis)` with NO arguments, so extra args are bound here (Node's
  // setTimeout(cb, delay, ...args) semantics).
  function armTimer(callback, delayMs, args, repeat) {
    if (typeof callback !== "function") {
      throw new TypeError(
        "elohim-render timers: callback must be a function " +
          "(string-eval timer form is not supported in the SSR runtime)"
      );
    }
    const task =
      args.length > 0 ? () => callback.apply(undefined, args) : callback;
    const ms = Number(delayMs) || 0;
    // Depth tracks HTML-spec timer nesting; deno_core carries it through purely
    // to expose getTimerDepth() during dispatch (no min-delay clamping here).
    return core.queueUserTimer(core.getTimerDepth() + 1, !!repeat, ms, task);
  }

  // Node-style timer handle. The bundle calls .unref()/.refresh() on the result
  // of setInterval/setTimeout, so we cannot return a bare number.
  class Timeout {
    constructor(callback, delayMs, args, repeat) {
      this._callback = callback;
      this._delayMs = delayMs;
      this._args = args;
      this._repeat = repeat;
      this._id = armTimer(callback, delayMs, args, repeat);
    }
    ref() {
      core.refTimer(this._id);
      return this;
    }
    unref() {
      core.unrefTimer(this._id);
      return this;
    }
    hasRef() {
      return true;
    }
    refresh() {
      // Re-arm from now with the same parameters (Node's Timeout#refresh).
      core.cancelTimer(this._id);
      this._id = armTimer(
        this._callback,
        this._delayMs,
        this._args,
        this._repeat
      );
      return this;
    }
    // Numeric coercion so `+handle` / `String(handle)` yield the raw id, and a
    // handle passed where a number is expected still clears.
    [Symbol.toPrimitive]() {
      return this._id;
    }
  }

  // Accepts a Timeout handle OR a raw numeric id; both clear the timer. Uses
  // core.cancelTimer (not the raw op) so a clear issued from inside another
  // timer's callback in the same tick is honored.
  function clearTimer(handle) {
    if (handle === null || handle === undefined) return;
    const id = typeof handle === "object" ? handle._id : Number(handle);
    if (id === null || id === undefined || Number.isNaN(id)) return;
    core.cancelTimer(id);
  }

  if (typeof globalThis.setTimeout !== "function") {
    globalThis.setTimeout = (cb, delay, ...args) =>
      new Timeout(cb, delay, args, false);
  }
  if (typeof globalThis.setInterval !== "function") {
    globalThis.setInterval = (cb, delay, ...args) =>
      new Timeout(cb, delay, args, true);
  }
  if (typeof globalThis.clearTimeout !== "function") {
    globalThis.clearTimeout = clearTimer;
  }
  if (typeof globalThis.clearInterval !== "function") {
    globalThis.clearInterval = clearTimer;
  }
  if (typeof globalThis.setImmediate !== "function") {
    // Node's setImmediate ~ a delay-0 macrotask; returns a ref/unref handle.
    globalThis.setImmediate = (cb, ...args) => new Timeout(cb, 0, args, false);
  }
  if (typeof globalThis.clearImmediate !== "function") {
    globalThis.clearImmediate = clearTimer;
  }
  // queueMicrotask is already a core global (with error reporting) -- do not
  // re-define it; only backfill if a future deno_core drops it.
  if (typeof globalThis.queueMicrotask !== "function") {
    globalThis.queueMicrotask = (cb) => {
      if (typeof cb !== "function") {
        throw new TypeError(
          "elohim-render queueMicrotask: callback must be a function"
        );
      }
      core.ops.op_queue_microtask(cb);
    };
  }

  // --- process ------------------------------------------------------------
  // Sentinel: the SSR host emulates a Node 20-class environment. The version is
  // split(".") and compared by some libs, so it must be a valid semver.
  const NODE_COMPAT_VERSION = "20.11.1";
  const versions = {
    node: NODE_COMPAT_VERSION,
    v8: core.ops.op_elohim_v8_version(), // REAL linked V8 version (honest)
  };

  // stdout/stderr writes surface in the render trace via console (which routes to
  // Rust tracing) rather than being silently dropped.
  const writeVia = (level) => (chunk) => {
    const s = typeof chunk === "string" ? chunk : String(chunk);
    if (
      globalThis.console &&
      typeof globalThis.console[level] === "function"
    ) {
      globalThis.console[level](s);
    }
    return true;
  };

  const proc = {
    platform: "linux", // honest host OS
    arch: "x64", // host arch (sentinel)
    version: "v" + NODE_COMPAT_VERSION,
    versions,
    env: { NODE_ENV: "production" }, // curated: this is a production build
    argv: ["node", "elohim-render"], // sentinel argv; .slice(2) -> []
    release: { name: "node" }, // node-compat host
    nextTick(callback, ...args) {
      if (typeof callback !== "function") {
        throw new TypeError(
          "elohim-render process.nextTick: callback must be a function"
        );
      }
      globalThis.queueMicrotask(
        args.length > 0 ? () => callback.apply(undefined, args) : callback
      );
    },
    cwd() {
      return "/"; // sentinel root; the SSR host has no working-directory concept
    },
    chdir() {
      /* no-op */
    },
    emitWarning() {
      /* swallow: dev-path deprecation warnings are not actionable in SSR */
    },
    // Minimal EventEmitter surface: no-ops returning `proc` for chaining. A
    // single-shot SSR render never fires process lifecycle events.
    on() {
      return proc;
    },
    once() {
      return proc;
    },
    off() {
      return proc;
    },
    addListener() {
      return proc;
    },
    removeListener() {
      return proc;
    },
    removeAllListeners() {
      return proc;
    },
    emit() {
      return false;
    },
    stdout: { fd: 1, isTTY: false, write: writeVia("log") },
    stderr: { fd: 2, isTTY: false, write: writeVia("error") },
  };

  if (typeof globalThis.process === "undefined") {
    globalThis.process = proc;
  }

  // --- performance --------------------------------------------------------
  // The Web Performance API. `performance.now()` (used ~heavily by RxJS schedulers
  // and zone.js timing) is op-backed with a real monotonic clock. The User Timing
  // API (mark/measure/getEntriesByName/clear*) is a small real in-memory store --
  // NOT a no-op -- because the bundle reads `getEntriesByName(name)[0].duration`
  // after a measure, so an empty-array stub would throw on the deref.
  //
  // Fidelity bound: entries live only for the duration of one render (a fresh
  // isolate per worker); resource-timing is a no-op (no network entries to record
  // in SSR). now() is monotonic milliseconds since a process-global time origin.
  const perfMarks = new Map();
  const perfMeasures = [];

  function perfEntry(name, entryType, startTime, duration) {
    return { name, entryType, startTime, duration };
  }

  const performanceShim = {
    now() {
      return core.ops.op_elohim_perf_now();
    },
    // Epoch-ish origin so `performance.timeOrigin + performance.now()` approximates
    // wall-clock ms, matching the browser/Node contract.
    timeOrigin: Date.now() - core.ops.op_elohim_perf_now(),
    mark(name, options) {
      const startTime =
        options && typeof options.startTime === "number"
          ? options.startTime
          : core.ops.op_elohim_perf_now();
      perfMarks.set(String(name), startTime);
      return perfEntry(String(name), "mark", startTime, 0);
    },
    measure(name, startOrOptions, endMark) {
      const now = core.ops.op_elohim_perf_now();
      let start = 0;
      let end = now;
      if (typeof startOrOptions === "string") {
        start = perfMarks.has(startOrOptions)
          ? perfMarks.get(startOrOptions)
          : 0;
      } else if (startOrOptions && typeof startOrOptions === "object") {
        if (typeof startOrOptions.start === "number") start = startOrOptions.start;
        if (typeof startOrOptions.end === "number") end = startOrOptions.end;
      }
      if (typeof endMark === "string") {
        end = perfMarks.has(endMark) ? perfMarks.get(endMark) : now;
      }
      const duration = Math.max(0, end - start);
      const entry = perfEntry(String(name), "measure", start, duration);
      perfMeasures.push(entry);
      return entry;
    },
    getEntriesByName(name, type) {
      const key = String(name);
      const out = [];
      for (let i = 0; i < perfMeasures.length; i++) {
        if (
          perfMeasures[i].name === key &&
          (type === undefined || type === "measure")
        ) {
          out.push(perfMeasures[i]);
        }
      }
      if (
        (type === undefined || type === "mark") &&
        perfMarks.has(key)
      ) {
        out.push(perfEntry(key, "mark", perfMarks.get(key), 0));
      }
      return out;
    },
    getEntries() {
      return perfMeasures.slice();
    },
    getEntriesByType(type) {
      return type === "measure" ? perfMeasures.slice() : [];
    },
    clearMarks(name) {
      if (name === undefined) {
        perfMarks.clear();
      } else {
        perfMarks.delete(String(name));
      }
    },
    clearMeasures(name) {
      if (name === undefined) {
        perfMeasures.length = 0;
      } else {
        for (let i = perfMeasures.length - 1; i >= 0; i--) {
          if (perfMeasures[i].name === String(name)) perfMeasures.splice(i, 1);
        }
      }
    },
    markResourceTiming() {
      /* no-op: SSR records no network resource-timing entries */
    },
  };

  if (typeof globalThis.performance === "undefined") {
    globalThis.performance = performanceShim;
  }

  // --- global alias -------------------------------------------------------
  // Node code -- and the bundle's own runtime detection (`E = global` once
  // process.versions.node is seen) -- expects a `global` binding aliasing the
  // global object. They are the same object; expose the alias.
  if (typeof globalThis.global === "undefined") {
    globalThis.global = globalThis;
  }
})(globalThis);
