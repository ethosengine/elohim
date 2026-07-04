// elohim-render Node `Buffer` shim: a REAL Uint8Array subclass installed as the
// global `Buffer`.
//
// Why a global (not just a module): the Angular server bundle's bundled `ws`
// evaluates `EMPTY_BUFFER = Buffer.alloc(0)` and `Buffer.from([0,0,255,255])` as
// TOP-LEVEL consts -- at module-eval time, during the SSR render's module-graph
// load. A bare deno_core runtime has no `Buffer` global, so this was the render
// wall (`ReferenceError: Buffer is not defined`). Extension `js` files run at
// isolate init (before any bundle module body), so installing the global here is
// what clears that wall. The `buffer`/`node:buffer` MODULE (served by
// node_builtins) re-exports THIS global, so `import { Buffer }` and the global
// are the same constructor and `x instanceof Buffer` is coherent.
//
// Correctness: `Buffer extends Uint8Array`, so `instanceof Uint8Array` holds and
// `.length`/`.byteOffset`/`.buffer`/`.set` are inherited-correct. utf8 rides
// deno_core's op_encode/op_decode; latin1 rides op_encode_binary_string; base64
// and hex ride the byte-exact Rust ops in node_buffer.rs. See that file's module
// doc for the full used-surface table and the loud-stub rationale.
//
// WARNING: pure ASCII (served through the `js = [dir ...]` macro).
((globalThis) => {
  const core = Deno.core;

  const b64enc = (bytes, urlSafe) =>
    core.ops.op_elohim_base64_encode(bytes, !!urlSafe);
  const b64dec = (s) => core.ops.op_elohim_base64_decode(s);
  const hexenc = (bytes) => core.ops.op_elohim_hex_encode(bytes);
  const hexdec = (s) => core.ops.op_elohim_hex_decode(s);

  // Normalize an encoding label to a canonical supported key, or null if this
  // shim does not implement it (ucs2/utf16le are intentionally absent -- not in
  // the bundle's used surface). Callers throw a LOUD named error on null rather
  // than guessing bytes.
  function normalizeEncoding(enc) {
    if (enc === undefined || enc === null) return "utf8";
    switch (String(enc).toLowerCase()) {
      case "utf8":
      case "utf-8":
        return "utf8";
      case "hex":
        return "hex";
      case "base64":
        return "base64";
      case "base64url":
        return "base64url";
      case "latin1":
      case "binary":
        return "latin1";
      case "ascii":
        return "ascii";
      default:
        return null;
    }
  }

  function unsupportedEncoding(enc) {
    return new TypeError(
      "elohim-render buffer shim: encoding " +
        JSON.stringify(String(enc)) +
        " is not implemented (SSR runtime shim, " +
        "elohim/elohim-render/src/shim/node_buffer.rs). Supported: " +
        "utf8, hex, base64, base64url, latin1/binary, ascii."
    );
  }

  // string -> a plain Uint8Array of the encoded bytes.
  function stringToBytes(str, enc) {
    const key = normalizeEncoding(enc);
    if (key === null) throw unsupportedEncoding(enc);
    const s = String(str);
    switch (key) {
      case "utf8":
        return core.encode(s);
      case "hex":
        return hexdec(s);
      case "base64":
      case "base64url":
        return b64dec(s);
      case "latin1": {
        const out = new Uint8Array(s.length);
        for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0xff;
        return out;
      }
      case "ascii": {
        const out = new Uint8Array(s.length);
        for (let i = 0; i < s.length; i++) out[i] = s.charCodeAt(i) & 0x7f;
        return out;
      }
      default:
        throw unsupportedEncoding(enc);
    }
  }

  // Build a JS string from raw bytes, masking each byte with `mask` (0xff for
  // latin1, 0x7f for ascii). Chunked so a very large buffer does not blow the
  // String.fromCharCode argument-count limit.
  function bytesToCharString(view, mask) {
    let out = "";
    const CHUNK = 8192;
    for (let i = 0; i < view.length; i += CHUNK) {
      const end = Math.min(i + CHUNK, view.length);
      const codes = new Array(end - i);
      for (let j = i; j < end; j++) codes[j - i] = view[j] & mask;
      out += String.fromCharCode.apply(null, codes);
    }
    return out;
  }

  // bytes (a Uint8Array view) -> a string in `enc`.
  function bytesToString(view, enc) {
    const key = normalizeEncoding(enc);
    if (key === null) throw unsupportedEncoding(enc);
    switch (key) {
      case "utf8":
        return core.decode(view);
      case "hex":
        return hexenc(view);
      case "base64":
        return b64enc(view, false);
      case "base64url":
        return b64enc(view, true);
      case "latin1":
        return core.encodeBinaryString(view);
      case "ascii":
        return bytesToCharString(view, 0x7f);
      default:
        throw unsupportedEncoding(enc);
    }
  }

  function isSharedArrayBuffer(v) {
    return (
      typeof SharedArrayBuffer !== "undefined" && v instanceof SharedArrayBuffer
    );
  }

  class Buffer extends Uint8Array {
    // Delegates to Uint8Array: `new Buffer(n)` allocates n zero-filled bytes;
    // `new Buffer(arrayBuffer, off, len)` shares memory (used by species-driven
    // subarray/slice); `new Buffer(uint8arrayOrArray)` copies + octet-truncates.
    // The bundle never calls `new Buffer(...)` directly (Buffer.from/alloc only);
    // this is the internal + @@species constructor.
    constructor(...args) {
      super(...args);
    }

    // --- static: construction ---------------------------------------------

    static from(value, encodingOrOffset, length) {
      if (value === null || value === undefined) {
        throw new TypeError(
          "elohim-render buffer shim: Buffer.from(null/undefined) is invalid"
        );
      }
      if (typeof value === "string") {
        return new Buffer(stringToBytes(value, encodingOrOffset));
      }
      if (value instanceof ArrayBuffer || isSharedArrayBuffer(value)) {
        // Node shares memory with the ArrayBuffer (zero-copy).
        const off = encodingOrOffset === undefined ? 0 : encodingOrOffset >>> 0;
        const len =
          length === undefined ? value.byteLength - off : length >>> 0;
        return new Buffer(value, off, len);
      }
      if (ArrayBuffer.isView(value)) {
        // Copy the BYTES (Node copies from a Buffer/TypedArray). A non-Uint8Array
        // view (Uint16Array, DataView, ...) is byte-reinterpreted, not element-
        // copied, so build a byte view first.
        const bytes =
          value instanceof Uint8Array
            ? value
            : new Uint8Array(value.buffer, value.byteOffset, value.byteLength);
        return new Buffer(bytes);
      }
      if (Array.isArray(value) || typeof value.length === "number") {
        // Octet array-like: super octet-truncates each element (matches Node's
        // `list[i] & 255`).
        return new Buffer(value);
      }
      throw new TypeError(
        "elohim-render buffer shim: Buffer.from(...) unsupported argument type"
      );
    }

    static of(...items) {
      return Buffer.from(items);
    }

    static alloc(size, fill, encoding) {
      const b = new Buffer(size >>> 0); // zero-filled by Uint8Array
      if (fill !== undefined && !(typeof fill === "number" && fill === 0)) {
        b.fill(fill, 0, b.length, encoding);
      }
      return b;
    }

    // Node's allocUnsafe returns UNinitialized memory; a Uint8Array is always
    // zero-filled. Zero-filling is a safe superset (never wrong, only slower),
    // so we intentionally return zeroed memory -- documented divergence.
    static allocUnsafe(size) {
      return new Buffer(size >>> 0);
    }

    static allocUnsafeSlow(size) {
      return new Buffer(size >>> 0);
    }

    static concat(list, totalLength) {
      if (!Array.isArray(list) && typeof list[Symbol.iterator] !== "function") {
        throw new TypeError(
          "elohim-render buffer shim: Buffer.concat expects a list of buffers"
        );
      }
      const items = Array.from(list);
      if (totalLength === undefined) {
        totalLength = 0;
        for (let i = 0; i < items.length; i++) totalLength += items[i].length;
      }
      totalLength = totalLength >>> 0;
      const out = new Buffer(totalLength);
      let pos = 0;
      for (let i = 0; i < items.length && pos < totalLength; i++) {
        let src = items[i];
        const room = totalLength - pos;
        if (src.length > room) src = src.subarray(0, room);
        out.set(src, pos);
        pos += src.length;
      }
      return out;
    }

    static isBuffer(x) {
      return x instanceof Buffer;
    }

    static isEncoding(enc) {
      return typeof enc === "string" && normalizeEncoding(enc) !== null;
    }

    static byteLength(input, encoding) {
      if (typeof input !== "string") {
        if (ArrayBuffer.isView(input)) return input.byteLength;
        if (input instanceof ArrayBuffer || isSharedArrayBuffer(input)) {
          return input.byteLength;
        }
        input = String(input);
      }
      return stringToBytes(input, encoding).length;
    }

    // --- instance ----------------------------------------------------------

    toString(encoding, start, end) {
      const len = this.length;
      let s = start === undefined ? 0 : start | 0;
      let e = end === undefined ? len : end | 0;
      if (s < 0) s = 0;
      if (e > len) e = len;
      if (e < s) e = s;
      const view = s === 0 && e === len ? this : this.subarray(s, e);
      return bytesToString(view, encoding);
    }

    // Node's Buffer.slice SHARES memory (an alias of subarray) -- unlike
    // Uint8Array.prototype.slice, which COPIES. Override to the sharing form so a
    // mutation-through-slice matches Node.
    slice(start, end) {
      return this.subarray(start, end);
    }

    equals(other) {
      if (this === other) return true;
      if (!(other instanceof Uint8Array)) return false;
      if (this.length !== other.length) return false;
      for (let i = 0; i < this.length; i++) {
        if (this[i] !== other[i]) return false;
      }
      return true;
    }

    copy(target, targetStart, sourceStart, sourceEnd) {
      targetStart = targetStart === undefined ? 0 : targetStart | 0;
      sourceStart = sourceStart === undefined ? 0 : sourceStart | 0;
      sourceEnd = sourceEnd === undefined ? this.length : sourceEnd | 0;
      if (sourceEnd < sourceStart) sourceEnd = sourceStart;
      let src = this.subarray(sourceStart, sourceEnd);
      const room = target.length - targetStart;
      if (src.length > room) src = src.subarray(0, Math.max(0, room));
      target.set(src, targetStart);
      return src.length;
    }

    fill(value, start, end, encoding) {
      // Node signatures: fill(value[, offset[, end]][, encoding]).
      if (typeof start === "string") {
        encoding = start;
        start = 0;
        end = this.length;
      } else if (typeof end === "string") {
        encoding = end;
        end = this.length;
      }
      start = start === undefined ? 0 : start | 0;
      end = end === undefined ? this.length : end | 0;
      if (start < 0) start = 0;
      if (end > this.length) end = this.length;
      if (end <= start) return this;

      if (typeof value === "number") {
        Uint8Array.prototype.fill.call(this, value & 0xff, start, end);
        return this;
      }
      let bytes;
      if (typeof value === "string") {
        bytes = stringToBytes(value, encoding);
      } else if (value instanceof Uint8Array) {
        bytes = value;
      } else {
        Uint8Array.prototype.fill.call(this, 0, start, end);
        return this;
      }
      if (bytes.length === 0) {
        Uint8Array.prototype.fill.call(this, 0, start, end);
        return this;
      }
      for (let i = start, j = 0; i < end; i++) {
        this[i] = bytes[j];
        j = j + 1 === bytes.length ? 0 : j + 1;
      }
      return this;
    }

    write(string, offset, length, encoding) {
      // Node signatures: write(string[, offset[, length]][, encoding]).
      if (offset === undefined) {
        offset = 0;
        length = this.length;
      } else if (typeof offset === "string") {
        encoding = offset;
        offset = 0;
        length = this.length;
      } else {
        offset = offset | 0;
        if (typeof length === "string") {
          encoding = length;
          length = this.length - offset;
        } else if (length === undefined) {
          length = this.length - offset;
        } else {
          length = length | 0;
        }
      }
      const bytes = stringToBytes(String(string), encoding);
      const n = Math.min(length, bytes.length, this.length - offset);
      if (n <= 0) return 0;
      this.set(bytes.subarray(0, n), offset);
      return n;
    }

    toJSON() {
      return { type: "Buffer", data: Array.prototype.slice.call(this) };
    }
  }

  // Off-boot-path binary wire-protocol read/write family: LOUD, NAMED stubs on
  // Buffer.prototype (see node_buffer.rs module doc). Used by `ws` frame framing
  // and protobuf varint at CONNECTION time, never during a single-shot SSR
  // render. A guessed offset/return impl would be silent-wrong on a live wire, so
  // the honest bound is a throw naming the method + this file. Installed on the
  // prototype so a call on a real Buffer instance is loud; a call on a non-Buffer
  // (a custom protobuf Writer) uses that object's own method and never reaches
  // here.
  const LOUD_MEMBERS = [
    "writeUInt8", "writeUInt16LE", "writeUInt16BE", "writeUInt32LE",
    "writeUInt32BE", "writeUIntLE", "writeUIntBE",
    "writeInt8", "writeInt16LE", "writeInt16BE", "writeInt32LE",
    "writeInt32BE", "writeIntLE", "writeIntBE",
    "writeBigUInt64LE", "writeBigUInt64BE", "writeBigInt64LE", "writeBigInt64BE",
    "writeFloatLE", "writeFloatBE", "writeDoubleLE", "writeDoubleBE",
    "readUInt8", "readUInt16LE", "readUInt16BE", "readUInt32LE",
    "readUInt32BE", "readUIntLE", "readUIntBE",
    "readInt8", "readInt16LE", "readInt16BE", "readInt32LE",
    "readInt32BE", "readIntLE", "readIntBE",
    "readBigUInt64LE", "readBigUInt64BE", "readBigInt64LE", "readBigInt64BE",
    "readFloatLE", "readFloatBE", "readDoubleLE", "readDoubleBE",
    "swap16", "swap32", "swap64",
  ];
  for (let i = 0; i < LOUD_MEMBERS.length; i++) {
    const name = LOUD_MEMBERS[i];
    if (typeof Buffer.prototype[name] === "function") continue;
    Object.defineProperty(Buffer.prototype, name, {
      value: function loudBufferMember() {
        throw new Error(
          "elohim-render buffer shim: " +
            name +
            "() is not implemented (SSR runtime shim, " +
            "elohim/elohim-render/src/shim/node_buffer.rs). It is an off-boot-path " +
            "binary wire-protocol method (ws framing / protobuf varint, called at " +
            "connection time, not during SSR render). Implement it deliberately " +
            "(byte-exact via DataView) if a render path proves it needs it."
        );
      },
      writable: true,
      configurable: true,
      enumerable: false,
    });
  }

  if (typeof globalThis.Buffer === "undefined") {
    globalThis.Buffer = Buffer;
  }
})(globalThis);
