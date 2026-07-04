// Golden fixture: the `buffer` module surface. `import { Buffer } from "buffer"`
// must resolve to the SAME constructor as the global `Buffer` (one source of
// truth: node_buffer_ext installs the global; node_builtins re-exports it), so
// cross-surface `instanceof` is coherent and the real codec surface works
// through the module binding.
import { Buffer as ModuleBuffer } from "buffer";

export function render(url) {
  const parts = [];

  // The module export is literally the global constructor.
  parts.push("same-class:" + (ModuleBuffer === globalThis.Buffer));

  // A Buffer made via the GLOBAL is instanceof the MODULE binding (same class).
  const viaGlobal = globalThis.Buffer.from("hi", "utf8");
  parts.push("cross-instanceof:" + (viaGlobal instanceof ModuleBuffer));

  // The real codec surface works through the module binding.
  const b64 = ModuleBuffer.from("hello", "utf8").toString("base64");
  parts.push("hello-b64:" + b64);

  return `<main data-url="${url}">${parts.join("|")}</main>`;
}
