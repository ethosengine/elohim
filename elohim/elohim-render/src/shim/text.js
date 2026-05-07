// TextEncoder and TextDecoder shims built on deno_core's op_encode / op_decode
// builtins (exposed as Deno.core.encode and Deno.core.decode).
((globalThis) => {
  class TextEncoder {
    get encoding() { return "utf-8"; }
    encode(input) {
      return Deno.core.encode(String(input == null ? "" : input));
    }
    encodeInto(input, dest) {
      const buf = this.encode(input);
      const len = Math.min(buf.length, dest.length);
      dest.set(buf.subarray(0, len));
      return { read: len, written: len };
    }
  }

  class TextDecoder {
    constructor(encoding, options) {
      this._encoding = (encoding || "utf-8").toLowerCase();
      this._fatal = !!(options && options.fatal);
      this._ignoreBOM = !!(options && options.ignoreBOM);
    }
    get encoding() { return this._encoding; }
    get fatal() { return this._fatal; }
    get ignoreBOM() { return this._ignoreBOM; }
    decode(input) {
      if (input == null) return "";
      let buf;
      if (input instanceof Uint8Array) {
        buf = input;
      } else if (ArrayBuffer.isView(input)) {
        buf = new Uint8Array(input.buffer, input.byteOffset, input.byteLength);
      } else if (input instanceof ArrayBuffer) {
        buf = new Uint8Array(input);
      } else {
        buf = new Uint8Array(0);
      }
      return Deno.core.decode(buf);
    }
  }

  globalThis.TextEncoder = TextEncoder;
  globalThis.TextDecoder = TextDecoder;
})(globalThis);
