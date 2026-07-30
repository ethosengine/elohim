// elohim-render web-platform API shims: DOMException, AbortController /
// AbortSignal, Headers, Request, Response, ReadableStream, Blob.
//
// WHY THIS FILE EXISTS
//
// deno_core ships NO web-platform globals (they live in deno_web / deno_fetch,
// which this crate deliberately does not pull in -- see src/shim/mod.rs). Until
// Angular 22 that was survivable: the old FetchBackend only reached for
// `globalThis.fetch`. Angular 22's FetchBackend.handle() runs
// `new AbortController()` SYNCHRONOUSLY on every subscribe, and constructs a
// `new DOMException(...)` for the request-timeout path. In a bare isolate the
// constructor throws before fetch() is ever called, the HttpClient PendingTask
// never settles, and `ApplicationRef.whenStable()` never resolves -- the render
// hangs to the wall-time limit (elohim-app, whose polling timers keep the loop
// alive) or dies with "Promise resolution is still pending but the event loop
// has already resolved" (lamad, which has no live timers).
//
// The response path needs just as much: FetchBackend reads
// `new HttpHeaders(response.headers)` (which takes its fast path only for a real
// `Headers` instance), `response.headers.get('content-length')` /
// `get('content-type')`, and -- the load-bearing one -- streams the body via
// `response.body.getReader()` / `reader.read()` / `reader.cancel()`. A shim
// response without a `ReadableStream` body silently yields a null HTTP body on
// EVERY successful SSR fetch, so a content-bearing render is impossible.
//
// SCOPE DISCIPLINE (mirrors src/shim/node_builtins.rs and url.js)
//
// Spec subsets, not spec compliance: implement what a proven render path
// exercises, keep behaviour honest, prefer absence over a fabricated value.
// Known, deliberate divergences:
//   - No global `EventTarget` / `Event`. `AbortSignal` carries its own listener
//     plumbing instead. Introducing a global `EventTarget` would hand zone.js
//     (live in the server polyfills bundle) a new prototype to monkey-patch for
//     no gain, since nothing in the render path does
//     `signal instanceof EventTarget`.
//   - `Headers` stores one combined value per lowercased name (comma-joined on
//     append), which is what `HttpHeaders` re-splits anyway -- `get()` returns
//     that joined form for every header, Set-Cookie included (compat). Set-
//     Cookie is the one exception underneath: it also keeps a dedicated,
//     never-joined value list (a Set-Cookie value can itself contain ", ",
//     e.g. an Expires attribute, so comma-joining it is lossy), and
//     `getSetCookie()` returns that list rather than re-splitting `get()`'s
//     joined form.
//   - `Response`'s constructor accepts non-spec `url` / `redirected` / `type`
//     init keys so the fetch shim can report the fields FetchBackend reads.
//     The spec builds those via `Response.error()` / internal fetch state,
//     which we have no plumbing for.
//   - `ReadableStream` is a byte-agnostic queue over an underlying source with
//     `start`/`pull`/`cancel`. No BYOB readers, no tee(), no pipeThrough(),
//     no backpressure signalling beyond `desiredSize`. The fetch shim enqueues
//     the whole body in one chunk (op_fetch is not incremental), so the
//     queue never grows past one element in practice.
//   - `Blob` is a byte container with `text()`/`arrayBuffer()`/`bytes()`/
//     `slice()`. No `stream()` line-endings normalization, no File.
//
// WARNING: pure ASCII. This file is served through deno_core's
// `js = [dir ...]` macro, which panics at const-eval on any non-ASCII byte.
((globalThis) => {
  // --- DOMException --------------------------------------------------------
  // Angular constructs `new DOMException('signal timed out', 'TimeoutError')`
  // and `new DOMException('Request timed out', 'TimeoutError')`; the fetch shim
  // constructs an 'AbortError'. The legacy numeric `code` is table-driven --
  // names outside the legacy table read 0, exactly as in a browser.
  const LEGACY_CODES = Object.create(null);
  Object.assign(LEGACY_CODES, {
    IndexSizeError: 1,
    HierarchyRequestError: 3,
    WrongDocumentError: 4,
    InvalidCharacterError: 5,
    NoModificationAllowedError: 7,
    NotFoundError: 8,
    NotSupportedError: 9,
    InUseAttributeError: 10,
    InvalidStateError: 11,
    SyntaxError: 12,
    InvalidModificationError: 13,
    NamespaceError: 14,
    InvalidAccessError: 15,
    TypeMismatchError: 17,
    SecurityError: 18,
    NetworkError: 19,
    AbortError: 20,
    URLMismatchError: 21,
    QuotaExceededError: 22,
    TimeoutError: 23,
    InvalidNodeTypeError: 24,
    DataCloneError: 25,
  });

  class DOMException extends Error {
    constructor(message, name) {
      super(message === undefined ? "" : String(message));
      this.name = name === undefined ? "Error" : String(name);
    }
    get code() {
      return LEGACY_CODES[this.name] || 0;
    }
  }
  Object.defineProperty(DOMException.prototype, "constructor", {
    value: DOMException,
    writable: true,
    configurable: true,
  });

  // --- AbortSignal / AbortController --------------------------------------
  // A signal owns its listener list directly (no global EventTarget; see the
  // header note). Listeners fire once, in registration order, and a listener
  // that throws must not stop the remaining listeners or the abort itself --
  // the error is reported and the loop continues, matching a browser's
  // per-listener error isolation.
  const SIGNAL_LISTENERS = Symbol("abortListeners");

  class AbortSignal {
    constructor() {
      // Guard against user construction the way the spec does: AbortSignal has
      // no public constructor, only AbortController / the static factories.
      if (!constructingSignal) {
        throw new TypeError("Illegal constructor");
      }
      this._aborted = false;
      this._reason = undefined;
      this.onabort = null;
      this[SIGNAL_LISTENERS] = [];
    }

    get aborted() {
      return this._aborted;
    }

    get reason() {
      return this._reason;
    }

    throwIfAborted() {
      if (this._aborted) {
        throw this._reason;
      }
    }

    addEventListener(type, listener, options) {
      if (type !== "abort" || typeof listener !== "function") {
        return;
      }
      const once = !!(options && typeof options === "object" && options.once);
      if (this._aborted) {
        // Spec: a listener added to an already-aborted signal never fires.
        return;
      }
      this[SIGNAL_LISTENERS].push({ listener, once });
    }

    removeEventListener(type, listener) {
      if (type !== "abort") {
        return;
      }
      const list = this[SIGNAL_LISTENERS];
      for (let i = 0; i < list.length; i++) {
        if (list[i].listener === listener) {
          list.splice(i, 1);
          return;
        }
      }
    }

    dispatchEvent(event) {
      if (!event || event.type !== "abort") {
        return true;
      }
      runAbortListeners(this, event);
      return true;
    }

    static abort(reason) {
      const signal = newSignal();
      abortSignal(signal, reason);
      return signal;
    }

    static timeout(ms) {
      const signal = newSignal();
      const handle = globalThis.setTimeout(() => {
        abortSignal(signal, new DOMException("signal timed out", "TimeoutError"));
      }, Number(ms) || 0);
      // A pending AbortSignal.timeout must not by itself keep the render's
      // event loop alive past the work it guards.
      if (handle && typeof handle.unref === "function") {
        handle.unref();
      }
      // Defensive: if the signal ever settles through some other path before
      // the timer fires, cancel the pending timer rather than leave it to
      // fire uselessly later -- an isolate reused across renders (see
      // src/shim/mod.rs) should not carry a stale pending timer forward.
      signal.addEventListener("abort", () => {
        globalThis.clearTimeout(handle);
      });
      return signal;
    }
  }

  let constructingSignal = false;
  function newSignal() {
    constructingSignal = true;
    try {
      return new AbortSignal();
    } finally {
      constructingSignal = false;
    }
  }

  function runAbortListeners(signal, event) {
    const list = signal[SIGNAL_LISTENERS];
    signal[SIGNAL_LISTENERS] = [];
    if (typeof signal.onabort === "function") {
      try {
        signal.onabort.call(signal, event);
      } catch (e) {
        reportListenerError(e);
      }
    }
    // `[SIGNAL_LISTENERS]` was already reset to `[]` above (abort is
    // terminal -- there is no second abort to re-register a listener for),
    // so every listener in `list` fires exactly once here regardless of its
    // `once` flag; there is no re-registration branch to take.
    for (let i = 0; i < list.length; i++) {
      try {
        list[i].listener.call(signal, event);
      } catch (e) {
        reportListenerError(e);
      }
    }
  }

  function reportListenerError(e) {
    // console is shimmed to Rust tracing (src/shim/console.js), so a throwing
    // abort listener is visible in the render log instead of swallowed.
    try {
      globalThis.console.error("abort listener threw:", e);
    } catch (_ignored) {
      // Nothing more we can do; never let reporting mask the abort.
    }
  }

  function abortSignal(signal, reason) {
    if (signal._aborted) {
      return;
    }
    signal._aborted = true;
    signal._reason =
      reason === undefined
        ? new DOMException("This operation was aborted", "AbortError")
        : reason;
    runAbortListeners(signal, { type: "abort", target: signal });
  }

  class AbortController {
    constructor() {
      this._signal = newSignal();
    }
    get signal() {
      return this._signal;
    }
    abort(reason) {
      abortSignal(this._signal, reason);
    }
  }

  // --- Headers ------------------------------------------------------------
  // One combined value per lowercased name. `HttpHeaders` splits on ',' when it
  // ingests us, so storing the combined form loses nothing it can observe.
  const FORBIDDEN_NAME = /[^!#$%&'*+\-.^_`|~0-9A-Za-z]/;

  function normalizeName(name) {
    const n = String(name);
    if (n.length === 0 || FORBIDDEN_NAME.test(n)) {
      throw new TypeError("Invalid header name: " + n);
    }
    return n.toLowerCase();
  }

  const FORBIDDEN_VALUE = /[\r\n\0]/;

  function normalizeValue(value) {
    // Strip leading/trailing HTTP whitespace, as the spec's normalize does.
    const trimmed = String(value === undefined || value === null ? "" : value).replace(
      /^[\t\n\r ]+|[\t\n\r ]+$/g,
      ""
    );
    // Mirror normalizeName's strictness: a header value carrying a raw CR,
    // LF, or NUL cannot be represented on the wire (CRLF is header-injection
    // territory) and a real Headers.set()/append() rejects it.
    if (FORBIDDEN_VALUE.test(trimmed)) {
      throw new TypeError("Invalid header value: " + trimmed);
    }
    return trimmed;
  }

  class Headers {
    constructor(init) {
      this._map = new Map();
      // Dedicated Set-Cookie list: unlike every other header, Set-Cookie
      // values must never be comma-joined (an Expires attribute legitimately
      // contains a comma, e.g. "Expires=Wed, 21 Oct ... GMT"), and a
      // response can legitimately carry more than one. `_map` still carries
      // a joined value under "set-cookie" for `get()` compat (see the header
      // note); this list is the source of truth for `getSetCookie()`.
      this._setCookies = [];
      if (init === undefined || init === null) {
        return;
      }
      if (init instanceof Headers) {
        // Clone internal state directly rather than forEach+append: forEach
        // yields one already-joined value per name, which would silently
        // collapse a multi-value Set-Cookie list (and any other repeated
        // header) down to a single re-split entry.
        this._map = new Map(init._map);
        this._setCookies = init._setCookies.slice();
        return;
      } else if (Array.isArray(init)) {
        for (const pair of init) {
          if (!pair || pair.length !== 2) {
            throw new TypeError("Header pair must be [name, value]");
          }
          this.append(pair[0], pair[1]);
        }
      } else if (typeof init[Symbol.iterator] === "function") {
        for (const pair of init) {
          this.append(pair[0], pair[1]);
        }
      } else {
        for (const name of Object.keys(init)) {
          this.append(name, init[name]);
        }
      }
    }

    append(name, value) {
      const key = normalizeName(name);
      const val = normalizeValue(value);
      if (key === "set-cookie") {
        this._setCookies.push(val);
      }
      const existing = this._map.get(key);
      this._map.set(key, existing === undefined ? val : existing + ", " + val);
    }

    set(name, value) {
      const key = normalizeName(name);
      const val = normalizeValue(value);
      if (key === "set-cookie") {
        this._setCookies = [val];
      }
      this._map.set(key, val);
    }

    get(name) {
      const val = this._map.get(normalizeName(name));
      return val === undefined ? null : val;
    }

    has(name) {
      return this._map.has(normalizeName(name));
    }

    delete(name) {
      const key = normalizeName(name);
      if (key === "set-cookie") {
        this._setCookies = [];
      }
      this._map.delete(key);
    }

    getSetCookie() {
      // The dedicated list (see the constructor note), never the comma-joined
      // `_map` entry -- a Set-Cookie value can itself contain ", " (an
      // Expires attribute), so re-splitting the joined form is lossy/wrong.
      return this._setCookies.slice();
    }

    forEach(callback, thisArg) {
      // Spec iteration order is sorted by name.
      for (const [name, value] of this.entries()) {
        callback.call(thisArg, value, name, this);
      }
    }

    *entries() {
      const names = Array.from(this._map.keys()).sort();
      for (const name of names) {
        yield [name, this._map.get(name)];
      }
    }

    *keys() {
      for (const [name] of this.entries()) {
        yield name;
      }
    }

    *values() {
      for (const [, value] of this.entries()) {
        yield value;
      }
    }

    [Symbol.iterator]() {
      return this.entries();
    }
  }

  // --- Blob ---------------------------------------------------------------
  function partsToBytes(parts) {
    if (parts === undefined || parts === null) {
      return new Uint8Array(0);
    }
    const chunks = [];
    let total = 0;
    for (const part of parts) {
      let bytes;
      if (part instanceof Blob) {
        bytes = part._bytes;
      } else if (part instanceof Uint8Array) {
        bytes = part;
      } else if (ArrayBuffer.isView(part)) {
        bytes = new Uint8Array(part.buffer, part.byteOffset, part.byteLength);
      } else if (part instanceof ArrayBuffer) {
        bytes = new Uint8Array(part);
      } else {
        bytes = new TextEncoder().encode(String(part));
      }
      chunks.push(bytes);
      total += bytes.length;
    }
    const out = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
      out.set(chunk, offset);
      offset += chunk.length;
    }
    return out;
  }

  class Blob {
    constructor(parts, options) {
      this._bytes = partsToBytes(parts);
      this._type = options && options.type ? String(options.type).toLowerCase() : "";
    }
    get size() {
      return this._bytes.length;
    }
    get type() {
      return this._type;
    }
    async text() {
      return new TextDecoder().decode(this._bytes);
    }
    async bytes() {
      return this._bytes.slice();
    }
    async arrayBuffer() {
      return this._bytes.slice().buffer;
    }
    slice(start, end, contentType) {
      const sliced = this._bytes.slice(start, end);
      return new Blob([sliced], {
        type: contentType === undefined ? this._type : contentType,
      });
    }
  }

  // --- ReadableStream -----------------------------------------------------
  // Queue over an underlying source. `pull` is invoked whenever a read finds
  // the queue empty and the stream is neither closed nor errored, so a source
  // that enqueues everything in `start` (what the fetch shim does) never sees
  // a pull at all.
  const STATE_READABLE = 0;
  const STATE_CLOSED = 1;
  const STATE_ERRORED = 2;

  class ReadableStreamDefaultController {
    constructor(stream) {
      this._stream = stream;
    }
    get desiredSize() {
      const s = this._stream;
      return s._state === STATE_READABLE ? s._highWaterMark - s._queue.length : 0;
    }
    enqueue(chunk) {
      const s = this._stream;
      if (s._state !== STATE_READABLE) {
        throw new TypeError("Cannot enqueue to a closed or errored stream");
      }
      s._queue.push(chunk);
      settlePendingReads(s);
    }
    close() {
      const s = this._stream;
      if (s._state !== STATE_READABLE) {
        return;
      }
      s._state = STATE_CLOSED;
      settlePendingReads(s);
    }
    error(reason) {
      const s = this._stream;
      if (s._state !== STATE_READABLE) {
        return;
      }
      s._state = STATE_ERRORED;
      s._storedError =
        reason === undefined ? new TypeError("Stream errored") : reason;
      settlePendingReads(s);
    }
  }

  // Hand queued chunks (or the terminal state) to whoever is waiting. Called
  // after every enqueue/close/error and at the head of each read.
  function settlePendingReads(stream) {
    while (stream._pendingReads.length > 0) {
      if (stream._queue.length > 0) {
        const { resolve } = stream._pendingReads.shift();
        resolve({ done: false, value: stream._queue.shift() });
        continue;
      }
      if (stream._state === STATE_CLOSED) {
        const { resolve } = stream._pendingReads.shift();
        resolve({ done: true, value: undefined });
        continue;
      }
      if (stream._state === STATE_ERRORED) {
        const { reject } = stream._pendingReads.shift();
        reject(stream._storedError);
        continue;
      }
      // Readable but empty: the reader waits for the next enqueue/close. The
      // loop MUST break here or it spins forever.
      return;
    }
  }

  class ReadableStreamDefaultReader {
    constructor(stream) {
      this._stream = stream;
      this._closedPromise = new Promise((resolve, reject) => {
        stream._closedResolvers.push({ resolve, reject });
      });
      // A rejected `closed` nobody inspects must not surface as an unhandled
      // rejection; consumers opt in by touching `.closed`.
      this._closedPromise.catch(() => {});
    }
    get closed() {
      return this._closedPromise;
    }
    read() {
      const s = this._stream;
      if (s._reader !== this) {
        return Promise.reject(new TypeError("Reader has been released"));
      }
      if (s._queue.length > 0) {
        return Promise.resolve({ done: false, value: s._queue.shift() });
      }
      if (s._state === STATE_CLOSED) {
        return Promise.resolve({ done: true, value: undefined });
      }
      if (s._state === STATE_ERRORED) {
        return Promise.reject(s._storedError);
      }
      const pending = new Promise((resolve, reject) => {
        s._pendingReads.push({ resolve, reject });
      });
      pullFrom(s);
      return pending;
    }
    async cancel(reason) {
      const s = this._stream;
      await cancelStream(s, reason);
    }
    releaseLock() {
      const s = this._stream;
      if (s._reader !== this) {
        return;
      }
      s._reader = null;
      // Spec: releasing the lock while reads are outstanding rejects them
      // with a TypeError rather than leaving them pending forever (nothing
      // else can ever settle a read whose reader is no longer attached).
      if (s._pendingReads.length > 0) {
        const pending = s._pendingReads;
        s._pendingReads = [];
        for (const { reject } of pending) {
          reject(new TypeError("The reader was released"));
        }
      }
    }
  }

  function pullFrom(stream) {
    if (stream._state !== STATE_READABLE || stream._pulling) {
      return;
    }
    const source = stream._source;
    if (!source || typeof source.pull !== "function") {
      return;
    }
    stream._pulling = true;
    Promise.resolve()
      .then(() => source.pull(stream._controller))
      .then(
        () => {
          stream._pulling = false;
          // If the pull produced nothing and the stream is still readable, we
          // deliberately do NOT re-pull: that would be the tight-loop shape.
          // The next read() drives the next pull.
        },
        (e) => {
          stream._pulling = false;
          stream._controller.error(e);
        }
      );
  }

  async function cancelStream(stream, reason) {
    if (stream._state === STATE_READABLE) {
      stream._state = STATE_CLOSED;
      stream._queue.length = 0;
      settlePendingReads(stream);
    }
    const source = stream._source;
    if (source && typeof source.cancel === "function") {
      await source.cancel(reason);
    }
  }

  class ReadableStream {
    constructor(source, strategy) {
      this._source = source || {};
      this._queue = [];
      this._pendingReads = [];
      this._closedResolvers = [];
      this._state = STATE_READABLE;
      this._storedError = undefined;
      this._reader = null;
      this._pulling = false;
      this._highWaterMark =
        strategy && typeof strategy.highWaterMark === "number"
          ? strategy.highWaterMark
          : 1;
      this._controller = new ReadableStreamDefaultController(this);
      if (typeof this._source.start === "function") {
        // Sync start, like the spec: a source that enqueues synchronously has
        // its data available to the first read() without an extra tick.
        const started = this._source.start(this._controller);
        if (started && typeof started.then === "function") {
          started.catch((e) => this._controller.error(e));
        }
      }
    }
    get locked() {
      return this._reader !== null;
    }
    getReader(options) {
      if (options && options.mode === "byob") {
        throw new TypeError("BYOB readers are not supported in the SSR runtime");
      }
      if (this._reader !== null) {
        throw new TypeError("ReadableStream is already locked");
      }
      this._reader = new ReadableStreamDefaultReader(this);
      return this._reader;
    }
    cancel(reason) {
      return cancelStream(this, reason);
    }
    async *[Symbol.asyncIterator]() {
      const reader = this.getReader();
      try {
        while (true) {
          const { done, value } = await reader.read();
          if (done) {
            return;
          }
          yield value;
        }
      } finally {
        reader.releaseLock();
      }
    }
  }

  // A stream carrying exactly `bytes` then closing. This is the only shape the
  // fetch shim needs, and it keeps the response body a real ReadableStream so
  // FetchBackend's `getReader()` loop drains in one iteration.
  function streamOfBytes(bytes) {
    return new ReadableStream({
      start(controller) {
        if (bytes && bytes.length > 0) {
          controller.enqueue(bytes);
        }
        controller.close();
      },
    });
  }

  // --- Body mixin ---------------------------------------------------------
  function bodyInit(target, body) {
    if (body === undefined || body === null) {
      target._bodyBytes = null;
      target._bodyStream = null;
      target._bodyContentType = null;
      return;
    }
    if (body instanceof ReadableStream) {
      target._bodyBytes = null;
      target._bodyStream = body;
      target._bodyContentType = null;
      return;
    }
    let contentType = null;
    let bytes;
    if (typeof body === "string") {
      bytes = new TextEncoder().encode(body);
      contentType = "text/plain;charset=UTF-8";
    } else if (body instanceof Blob) {
      bytes = body._bytes;
      contentType = body.type || null;
    } else if (body instanceof Uint8Array) {
      bytes = body;
    } else if (ArrayBuffer.isView(body)) {
      bytes = new Uint8Array(body.buffer, body.byteOffset, body.byteLength);
    } else if (body instanceof ArrayBuffer) {
      bytes = new Uint8Array(body);
    } else {
      bytes = new TextEncoder().encode(String(body));
      contentType = "text/plain;charset=UTF-8";
    }
    target._bodyBytes = bytes;
    target._bodyStream = null;
    target._bodyContentType = contentType;
  }

  async function drainBody(target) {
    if (target._bodyUsed) {
      throw new TypeError("Body has already been consumed");
    }
    target._bodyUsed = true;
    if (target._bodyBytes !== null && target._bodyBytes !== undefined) {
      return target._bodyBytes;
    }
    const stream = target._bodyStream;
    if (!stream) {
      return new Uint8Array(0);
    }
    const reader = stream.getReader();
    const chunks = [];
    let total = 0;
    for (;;) {
      const { done, value } = await reader.read();
      if (done) {
        break;
      }
      chunks.push(value);
      total += value.length;
    }
    const out = new Uint8Array(total);
    let offset = 0;
    for (const chunk of chunks) {
      out.set(chunk, offset);
      offset += chunk.length;
    }
    target._bodyBytes = out;
    return out;
  }

  function defineBodyMixin(proto) {
    Object.defineProperty(proto, "body", {
      configurable: true,
      get() {
        if (this._bodyStream) {
          return this._bodyStream;
        }
        if (this._bodyBytes === null || this._bodyBytes === undefined) {
          return null;
        }
        // Materialize lazily so a caller that only reads text() never builds
        // a stream, and one that reads `.body` twice sees the same instance.
        this._bodyStream = streamOfBytes(this._bodyBytes);
        this._bodyBytes = null;
        return this._bodyStream;
      },
    });
    Object.defineProperty(proto, "bodyUsed", {
      configurable: true,
      get() {
        return !!this._bodyUsed;
      },
    });
    proto.arrayBuffer = async function arrayBuffer() {
      const bytes = await drainBody(this);
      return bytes.slice().buffer;
    };
    proto.bytes = async function bytes() {
      return (await drainBody(this)).slice();
    };
    proto.text = async function text() {
      return new TextDecoder().decode(await drainBody(this));
    };
    proto.json = async function json() {
      const raw = new TextDecoder().decode(await drainBody(this));
      return JSON.parse(raw);
    };
    proto.blob = async function blob() {
      const bytes = await drainBody(this);
      const type = this.headers ? this.headers.get("content-type") : null;
      return new Blob([bytes], { type: type || "" });
    };
  }

  // A body that has been lazily materialized into a stream (see the `body`
  // getter in defineBodyMixin: `_bodyBytes` cleared, `_bodyStream` set) has
  // no buffered bytes left to hand a clone -- cloning it would need tee(),
  // which this subset does not implement (see the file header). Silently
  // building a copy with an empty body is worse than refusing outright.
  function assertBodyCloneable(target) {
    if (target._bodyBytes === null && target._bodyStream) {
      throw new TypeError("cannot clone a stream-backed body");
    }
  }

  // --- Request ------------------------------------------------------------
  // The render path calls `fetch(urlString, init)`, so Request exists for
  // bundle code that constructs one (and for `fetch(request)`), not because
  // FetchBackend needs it.
  class Request {
    constructor(input, init) {
      const opts = init || {};
      if (input instanceof Request) {
        this._url = input.url;
        this.method = (opts.method || input.method || "GET").toUpperCase();
        this.headers = new Headers(opts.headers || input.headers);
        this.signal = opts.signal || input.signal || null;
        bodyInit(this, opts.body !== undefined ? opts.body : input._bodyBytes);
      } else {
        this._url = String(input);
        this.method = (opts.method || "GET").toUpperCase();
        this.headers = new Headers(opts.headers);
        this.signal = opts.signal || null;
        bodyInit(this, opts.body);
      }
      this.credentials = opts.credentials || "same-origin";
      this.mode = opts.mode || "cors";
      this.cache = opts.cache || "default";
      this.redirect = opts.redirect || "follow";
      this.referrer = opts.referrer === undefined ? "about:client" : opts.referrer;
      this.integrity = opts.integrity || "";
      this.keepalive = !!opts.keepalive;
      this._bodyUsed = false;
      if (this._bodyContentType && !this.headers.has("content-type")) {
        this.headers.set("content-type", this._bodyContentType);
      }
    }
    get url() {
      return this._url;
    }
    clone() {
      assertBodyCloneable(this);
      return new Request(this, { body: this._bodyBytes });
    }
  }
  defineBodyMixin(Request.prototype);

  // --- Response -----------------------------------------------------------
  const DEFAULT_STATUS_TEXT = {
    200: "OK",
    201: "Created",
    202: "Accepted",
    204: "No Content",
    301: "Moved Permanently",
    302: "Found",
    304: "Not Modified",
    400: "Bad Request",
    401: "Unauthorized",
    403: "Forbidden",
    404: "Not Found",
    409: "Conflict",
    410: "Gone",
    429: "Too Many Requests",
    500: "Internal Server Error",
    502: "Bad Gateway",
    503: "Service Unavailable",
    504: "Gateway Timeout",
  };

  // Statuses the spec forbids a body on: 101/103 are informational
  // (Switching Protocols / Early Hints, no body by definition), 204/205 are
  // No Content / Reset Content, 304 is Not Modified.
  const NULL_BODY_STATUSES = new Set([101, 103, 204, 205, 304]);

  class Response {
    constructor(body, init) {
      const opts = init || {};
      this._status = opts.status === undefined ? 200 : Number(opts.status);
      this._statusText =
        opts.statusText === undefined
          ? DEFAULT_STATUS_TEXT[this._status] || ""
          : String(opts.statusText);
      this.headers = new Headers(opts.headers);
      // Non-spec init keys (see the header note): the fetch shim needs to
      // report the response URL / redirect state FetchBackend reads.
      this._url = opts.url === undefined ? "" : String(opts.url);
      this._redirected = !!opts.redirected;
      this._type = opts.type === undefined ? "default" : String(opts.type);
      this._bodyUsed = false;
      bodyInit(this, NULL_BODY_STATUSES.has(this._status) ? null : body);
      if (this._bodyContentType && !this.headers.has("content-type")) {
        this.headers.set("content-type", this._bodyContentType);
      }
    }
    get status() {
      return this._status;
    }
    get statusText() {
      return this._statusText;
    }
    get ok() {
      return this._status >= 200 && this._status < 300;
    }
    get url() {
      return this._url;
    }
    get redirected() {
      return this._redirected;
    }
    get type() {
      return this._type;
    }
    clone() {
      if (this._bodyUsed) {
        throw new TypeError("Cannot clone a Response with a used body");
      }
      // Only a fully-buffered body can be cloned here; a stream-backed body
      // would need tee(), which this subset does not implement (see
      // assertBodyCloneable).
      assertBodyCloneable(this);
      const copy = new Response(this._bodyBytes, {
        status: this._status,
        statusText: this._statusText,
        headers: this.headers,
        url: this._url,
        redirected: this._redirected,
        type: this._type,
      });
      return copy;
    }
    static error() {
      const r = new Response(null, { status: 0, statusText: "", type: "error" });
      return r;
    }
    static json(data, init) {
      const opts = init || {};
      const r = new Response(JSON.stringify(data), opts);
      if (!r.headers.has("content-type")) {
        r.headers.set("content-type", "application/json");
      }
      return r;
    }
  }
  defineBodyMixin(Response.prototype);

  // --- Install ------------------------------------------------------------
  // Guarded so a future deno_core / snapshot that DOES provide these wins.
  const exports = {
    DOMException,
    AbortController,
    AbortSignal,
    Headers,
    Request,
    Response,
    ReadableStream,
    Blob,
  };
  for (const name of Object.keys(exports)) {
    if (typeof globalThis[name] === "undefined") {
      globalThis[name] = exports[name];
    }
  }

  // Internal handle the fetch shim uses to build a byte-backed response body
  // without re-deriving the stream plumbing.
  globalThis[Symbol.for("elohim.render.streamOfBytes")] = streamOfBytes;
})(globalThis);
