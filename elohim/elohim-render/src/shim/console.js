// Minimal console binding to ops; shape compatible with what Angular SSR uses.
((globalThis) => {
  const fmt = (args) => args.map((a) => {
    if (typeof a === "string") return a;
    try { return JSON.stringify(a); } catch { return String(a); }
  }).join(" ");

  globalThis.console = {
    log:   (...a) => Deno.core.ops.op_console_log(fmt(a)),
    info:  (...a) => Deno.core.ops.op_console_log(fmt(a)),
    warn:  (...a) => Deno.core.ops.op_console_warn(fmt(a)),
    error: (...a) => Deno.core.ops.op_console_error(fmt(a)),
    debug: (...a) => Deno.core.ops.op_console_log(fmt(a)),
  };
})(globalThis);
