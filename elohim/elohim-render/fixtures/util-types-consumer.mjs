import { types } from "node:util";
import { createRequire } from "node:module";

// Match ws's module-evaluation destructuring, which previously prevented boot.
const { types: { isUint8Array } } = createRequire(import.meta.url)("util");

export function render() {
  const positive = [new Uint8Array(2), Buffer.from("hello")];
  const negative = [null, undefined, 1, "bytes", {}, [], new Uint16Array(2),
    new Uint8ClampedArray(2), new DataView(new ArrayBuffer(2)),
    { [Symbol.toStringTag]: "Uint8Array" }, Object.create(Uint8Array.prototype)];
  for (const check of [isUint8Array, types.isUint8Array]) {
    if (!positive.every(check) || negative.some(check)) throw new Error("wrong byte-view brand");
    const disguised = new Uint8Array(2);
    Object.defineProperty(disguised, Symbol.toStringTag, { value: "Uint16Array" });
    if (!check(disguised)) throw new Error("spoofed tag hid byte-view brand");
  }
  return "<main>util byte-view brands verified</main>";
}
