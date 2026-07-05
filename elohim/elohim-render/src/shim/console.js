// Minimal console binding to ops; shape compatible with what Angular SSR uses.
((globalThis) => {
  const fmt = (args) => args.map((a) => {
    if (typeof a === "string") return a;
    // Error's `message`/`stack` are non-enumerable own properties, so
    // JSON.stringify(someError) yields "{}" -- discarding the one thing we
    // actually want to see. Special-case Error (and error-like values from
    // another realm, where `instanceof Error` can be false but `.stack`/
    // `.message` are still strings) before the JSON branch below.
    const isErrorLike = a instanceof Error ||
      (a && typeof a === "object" && (typeof a.stack === "string" || typeof a.message === "string"));
    if (isErrorLike) {
      return a.stack || `${a.name || "Error"}: ${a.message || ""}`;
    }
    try {
      const s = JSON.stringify(a);
      // JSON.stringify returns undefined for bare `undefined`/functions/etc;
      // fall back to String(a) so the value isn't silently dropped.
      return s === undefined ? String(a) : s;
    } catch { return String(a); }
  }).join(" ");

  globalThis.console = {
    log:   (...a) => Deno.core.ops.op_console_log(fmt(a)),
    info:  (...a) => Deno.core.ops.op_console_log(fmt(a)),
    warn:  (...a) => Deno.core.ops.op_console_warn(fmt(a)),
    error: (...a) => Deno.core.ops.op_console_error(fmt(a)),
    debug: (...a) => Deno.core.ops.op_console_log(fmt(a)),
  };
})(globalThis);
