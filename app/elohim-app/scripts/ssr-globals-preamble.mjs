// === deno_core Node global shims ===
// Injected by ssr-shim-node.mjs post-build script into polyfills.server.mjs.
// These shims make the Angular SSR bundle loadable in deno_core's bare V8 runtime.
//
// deno_core (without deno_node) provides:
//   - V8 builtins (Promise, Map, Set, etc.)
//   - Extensions registered in RuntimeOptions (console, URL, TextEncoder in with_shims())
//   - Module loading via FsModuleLoader (file:// URLs and relative paths only)
//
// It does NOT provide: process, Buffer, global, setTimeout, require, or any Node builtins.
//
// The Angular SSR bundle (using platform: 'node' in esbuild) emits bare ESM imports
// for Node builtins — those are rewritten to ./node-shims/*.mjs by ssr-shim-node.mjs.
// However, bundled CJS packages (ws, undici, etc.) wrapped via __commonJS call require()
// synchronously. This preamble installs a require() shim that returns stub objects.
//
// None of these code paths execute during an actual SSR render — they only need to
// survive module evaluation so Angular can bootstrap and render the component tree.

if (typeof globalThis.global === 'undefined') {
  globalThis.global = globalThis;
}

// Disable Zone.js Promise wrapping before Zone.js loads.
//
// Zone.js patches globalThis.Promise with ZoneAwarePromise to track async
// operations. In deno_core's V8 isolate, Zone-wrapped Promises do not integrate
// with deno_core's native microtask scheduler — the event loop's
// EventLoopPendingState.has_pending_promise_events check sees Zone-wrapped
// Promises as pending when V8's native queue is empty, causing poll_event_loop()
// to return Poll::Pending indefinitely.
//
// Setting these flags BEFORE zone.js loads prevents it from replacing the native
// Promise, keeping deno_core's microtask tracking intact. Angular 19 with
// NoopNgZone (injected via server config) does not require Zone's Promise tracking
// for change detection or stability — the PendingTasks mechanism is used instead.
globalThis.__Zone_disable_ZoneAwarePromise = true;
// Also disable other Zone patches that could interfere with deno_core's event loop:
globalThis.__Zone_disable_timers = true;       // We provide our own timer shims
globalThis.__Zone_disable_requestAnimationFrame = true;
globalThis.__Zone_disable_blocking = true;
globalThis.__Zone_disable_EventEmitter = true;
globalThis.__Zone_disable_fs = true;
globalThis.__Zone_disable_XHR = true;          // No XHR in SSR; FetchBackend is used

// Web Crypto shim — deno_core bare runtime has no globalThis.crypto.
// Many crypto libraries (noble/hashes, libp2p) check for getRandomValues.
// For SSR we provide a deterministic-enough stub (Holochain never connects).
if (typeof globalThis.crypto === 'undefined' || typeof globalThis.crypto.getRandomValues === 'undefined') {
  // Simple LCG PRNG — not cryptographically secure, but avoids import-time crashes.
  // The SSR render path never uses cryptographic operations for real.
  let _seed = (Date.now() ^ 0xdeadbeef) >>> 0;
  function _lcgRandom() {
    _seed = (Math.imul(1664525, _seed) + 1013904223) >>> 0;
    return _seed;
  }
  function _getRandomValues(buf) {
    for (let i = 0; i < buf.length; i++) {
      buf[i] = _lcgRandom() & 0xff;
    }
    return buf;
  }

  const _subtle = {
    async digest(algorithm, data) {
      return new ArrayBuffer(32); // stub
    },
    async generateKey() { return { privateKey: {}, publicKey: {} }; },
    async exportKey() { return new ArrayBuffer(32); },
    async importKey() { return {}; },
    async sign() { return new ArrayBuffer(64); },
    async verify() { return false; },
    async encrypt() { return new ArrayBuffer(0); },
    async decrypt() { return new ArrayBuffer(0); },
    async deriveBits() { return new ArrayBuffer(32); },
    async deriveKey() { return {}; },
    async wrapKey() { return new ArrayBuffer(32); },
    async unwrapKey() { return {}; },
  };

  if (typeof globalThis.crypto === 'undefined') {
    globalThis.crypto = {
      getRandomValues: _getRandomValues,
      randomUUID() {
        const b = new Uint8Array(16);
        _getRandomValues(b);
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        const hex = Array.from(b).map(x => x.toString(16).padStart(2, '0')).join('');
        return hex.slice(0,8)+'-'+hex.slice(8,12)+'-'+hex.slice(12,16)+'-'+hex.slice(16,20)+'-'+hex.slice(20);
      },
      subtle: _subtle,
    };
  } else {
    globalThis.crypto.getRandomValues = _getRandomValues;
    if (!globalThis.crypto.subtle) globalThis.crypto.subtle = _subtle;
  }
}

if (typeof globalThis.process === 'undefined') {
  const proc = {
    env: {},
    argv: [],
    argv0: 'node',
    platform: 'linux',
    version: 'v18.0.0',
    versions: { node: '18.0.0' },
    pid: 1,
    ppid: 0,
    arch: 'x64',
    cwd() { return '/'; },
    exit(code) { throw new Error('process.exit(' + code + ') called in SSR'); },
    on() { return this; },
    off() { return this; },
    once() { return this; },
    emit() { return false; },
    removeListener() { return this; },
    addListener() { return this; },
    nextTick(fn, ...args) { Promise.resolve().then(() => fn(...args)); },
    hrtime(prev) {
      const n = Date.now() * 1e6;
      if (prev) {
        return [Math.floor((n - (prev[0] * 1e9 + prev[1])) / 1e9), (n - (prev[0] * 1e9 + prev[1])) % 1e9];
      }
      return [Math.floor(n / 1e9), n % 1e9];
    },
    uptime() { return 0; },
    memoryUsage() { return { rss: 0, heapTotal: 0, heapUsed: 0, external: 0, arrayBuffers: 0 }; },
    stdout: { write() {}, on() { return this; }, isTTY: false },
    stderr: { write() {}, on() { return this; }, isTTY: false },
    stdin: { on() { return this; }, resume() {}, isTTY: false },
  };
  globalThis.process = proc;
}

if (typeof globalThis.Buffer === 'undefined') {
  class Buffer extends Uint8Array {
    static from(data, enc) {
      if (typeof data === 'string') {
        return new Buffer(new TextEncoder().encode(data));
      }
      if (data instanceof ArrayBuffer) return new Buffer(data);
      return new Buffer(Uint8Array.from(data));
    }
    static alloc(n, fill) {
      const b = new Buffer(n);
      if (fill !== undefined) b.fill(fill);
      return b;
    }
    static allocUnsafe(n) { return new Buffer(n); }
    static isBuffer(v) { return v instanceof Buffer || v instanceof Uint8Array; }
    static concat(bufs, totalLen) {
      const len = totalLen ?? bufs.reduce((s, b) => s + b.length, 0);
      const out = new Buffer(len);
      let off = 0;
      for (const b of bufs) { out.set(b, off); off += b.length; }
      return out;
    }
    static byteLength(str, enc) {
      if (typeof str === 'string') return new TextEncoder().encode(str).length;
      return str.length;
    }
    static compare(a, b) {
      for (let i = 0; i < Math.min(a.length, b.length); i++) {
        if (a[i] !== b[i]) return a[i] < b[i] ? -1 : 1;
      }
      return a.length - b.length;
    }
    toString(enc, start, end) {
      const slice = (start !== undefined || end !== undefined) ? this.slice(start, end) : this;
      if (!enc || enc === 'utf8' || enc === 'utf-8') return new TextDecoder().decode(slice);
      if (enc === 'hex') return Array.from(slice).map(b => b.toString(16).padStart(2, '0')).join('');
      if (enc === 'base64') return btoa(String.fromCharCode(...slice));
      if (enc === 'binary' || enc === 'latin1') return String.fromCharCode(...slice);
      return new TextDecoder().decode(slice);
    }
    write(str, offset, length, enc) {
      const encoded = new TextEncoder().encode(str);
      this.set(encoded.slice(0, length ?? encoded.length), offset ?? 0);
      return encoded.length;
    }
    copy(target, targetStart) {
      target.set(this, targetStart ?? 0);
    }
    fill(val, start, end) {
      Uint8Array.prototype.fill.call(this, val, start, end);
      return this;
    }
    includes(val) {
      if (typeof val === 'number') return this.indexOf(val) !== -1;
      if (typeof val === 'string') {
        const encoded = new TextEncoder().encode(val);
        outer: for (let i = 0; i <= this.length - encoded.length; i++) {
          for (let j = 0; j < encoded.length; j++) {
            if (this[i + j] !== encoded[j]) continue outer;
          }
          return true;
        }
        return false;
      }
      return false;
    }
  }
  globalThis.Buffer = Buffer;
}

// Timer shims — deno_core bare runtime has no setTimeout/clearTimeout.
if (typeof globalThis.setTimeout === 'undefined') {
  let _timerId = 1;
  const _timers = new Map();

  globalThis.setTimeout = function(fn, _delay, ...args) {
    const id = _timerId++;
    Promise.resolve().then(() => {
      if (_timers.has(id)) {
        _timers.delete(id);
        if (typeof fn === 'function') fn(...args);
        else (0, eval)(String(fn));
      }
    });
    _timers.set(id, fn);
    return id;
  };

  globalThis.clearTimeout = function(id) {
    _timers.delete(id);
  };

  globalThis.setInterval = function(fn, delay, ...args) {
    // Stub: fire once only — intervals are not meaningful in SSR
    return globalThis.setTimeout(fn, delay, ...args);
  };

  globalThis.clearInterval = globalThis.clearTimeout;
}

if (typeof globalThis.setImmediate === 'undefined') {
  globalThis.setImmediate = (fn, ...args) => globalThis.setTimeout(fn, 0, ...args);
  globalThis.clearImmediate = globalThis.clearTimeout;
}

if (typeof globalThis.queueMicrotask === 'undefined') {
  globalThis.queueMicrotask = (fn) => Promise.resolve().then(fn);
}

// Storage shims — DoorwayRegistryService reads localStorage on construction.
// In SSR there is no real storage; return an in-memory Map-backed stub.
if (typeof globalThis.localStorage === 'undefined') {
  const _store = new Map();
  globalThis.localStorage = {
    getItem(k) { return _store.has(k) ? _store.get(k) : null; },
    setItem(k, v) { _store.set(k, String(v)); },
    removeItem(k) { _store.delete(k); },
    clear() { _store.clear(); },
    key(n) { return [..._store.keys()][n] ?? null; },
    get length() { return _store.size; },
  };
}

if (typeof globalThis.sessionStorage === 'undefined') {
  const _sstore = new Map();
  globalThis.sessionStorage = {
    getItem(k) { return _sstore.has(k) ? _sstore.get(k) : null; },
    setItem(k, v) { _sstore.set(k, String(v)); },
    removeItem(k) { _sstore.delete(k); },
    clear() { _sstore.clear(); },
    key(n) { return [..._sstore.keys()][n] ?? null; },
    get length() { return _sstore.size; },
  };
}

// Location shim — app code reads globalThis.location.hostname to detect Eclipse Che.
// In SSR there is no real browser location; return a neutral localhost-like object.
if (typeof globalThis.location === 'undefined') {
  globalThis.location = {
    hostname: 'ssr-server',
    host: 'ssr-server',
    href: 'http://ssr-server/',
    origin: 'http://ssr-server',
    pathname: '/',
    port: '',
    protocol: 'http:',
    search: '',
    hash: '',
    replace() {},
    assign() {},
    reload() {},
    toString() { return this.href; },
  };
}

// Performance API shim — Angular 19 uses performance.mark() for diagnostics.
if (typeof globalThis.performance === 'undefined') {
  const _start = Date.now();
  globalThis.performance = {
    now() { return Date.now() - _start; },
    mark() {},
    measure() {},
    clearMarks() {},
    clearMeasures() {},
    getEntriesByName() { return []; },
    getEntriesByType() { return []; },
    getEntries() { return []; },
    timeOrigin: Date.now(),
  };
}

// Minimal DOM / browser globals needed for Angular SSR + app services.
// @angular/platform-server normally provides a domino-based DOM; in deno_core
// we have the WHATWG DOM from V8/deno_core's extensions but many APIs are missing.
// These stubs let Angular bootstrap and app services construct without crashing.

if (typeof globalThis.navigator === 'undefined') {
  globalThis.navigator = {
    userAgent: 'Mozilla/5.0 (SSR; deno_core) AppleWebKit/537.36',
    platform: 'Linux',
    language: 'en-US',
    languages: ['en-US', 'en'],
    onLine: true,
    cookieEnabled: false,
    clipboard: { writeText() { return Promise.resolve(); }, readText() { return Promise.resolve(''); } },
    geolocation: { getCurrentPosition(s, e) { if (e) e(new Error('not available')); }, watchPosition() { return 0; }, clearWatch() {} },
    registerProtocolHandler() {},
    permissions: { query() { return Promise.resolve({ state: 'denied' }); } },
  };
}

if (typeof globalThis.requestAnimationFrame === 'undefined') {
  globalThis.requestAnimationFrame = (fn) => globalThis.setTimeout(fn, 16);
  globalThis.cancelAnimationFrame = globalThis.clearTimeout;
}

if (typeof globalThis.indexedDB === 'undefined') {
  globalThis.indexedDB = null;
}

// Event class — Zone.js and Angular check for its existence.
// deno_core's URL/text shims don't include the DOM Event class.
if (typeof globalThis.Event === 'undefined') {
  globalThis.Event = class Event {
    constructor(type, init) {
      this.type = type;
      this.bubbles = init?.bubbles ?? false;
      this.cancelable = init?.cancelable ?? false;
      this.composed = init?.composed ?? false;
      this.defaultPrevented = false;
      this.timeStamp = Date.now();
      this.target = null;
      this.currentTarget = null;
      this.isTrusted = false;
    }
    preventDefault() { this.defaultPrevented = true; }
    stopPropagation() {}
    stopImmediatePropagation() {}
    composedPath() { return []; }
  };
}

if (typeof globalThis.CustomEvent === 'undefined') {
  globalThis.CustomEvent = class CustomEvent extends globalThis.Event {
    constructor(type, init) { super(type, init); this.detail = init?.detail ?? null; }
  };
}

if (typeof globalThis.EventTarget === 'undefined') {
  globalThis.EventTarget = class EventTarget {
    constructor() { this._listeners = {}; }
    addEventListener(type, fn) { (this._listeners[type] = this._listeners[type] || []).push(fn); }
    removeEventListener(type, fn) { this._listeners[type] = (this._listeners[type] || []).filter(f => f !== fn); }
    dispatchEvent(evt) { (this._listeners[evt.type] || []).forEach(f => f(evt)); return !evt.defaultPrevented; }
  };
}

if (typeof globalThis.MutationObserver === 'undefined') {
  globalThis.MutationObserver = class MutationObserver {
    constructor() {}
    observe() {}
    disconnect() {}
    takeRecords() { return []; }
  };
}

if (typeof globalThis.IntersectionObserver === 'undefined') {
  globalThis.IntersectionObserver = class IntersectionObserver {
    constructor() {}
    observe() {}
    unobserve() {}
    disconnect() {}
    takeRecords() { return []; }
  };
}

if (typeof globalThis.ResizeObserver === 'undefined') {
  globalThis.ResizeObserver = class ResizeObserver {
    constructor() {}
    observe() {}
    unobserve() {}
    disconnect() {}
  };
}

if (typeof globalThis.matchMedia === 'undefined') {
  globalThis.matchMedia = (query) => ({
    matches: false, media: query, onchange: null,
    addListener() {}, removeListener() {},
    addEventListener() {}, removeEventListener() {},
    dispatchEvent() { return false; },
  });
}

if (typeof globalThis.fetch === 'undefined') {
  globalThis.fetch = (url, opts) => Promise.reject(new Error('fetch not available in SSR (no DataFetcher wired): ' + url));
}

// window = globalThis for code that does `window.addEventListener` etc.
if (typeof globalThis.window === 'undefined') {
  globalThis.window = globalThis;
}

// Synchronous require() shim for CJS packages wrapped via __commonJS in the bundle.
// ws, undici, and other packages call require('events'), require('stream'), etc.
// synchronously at module evaluation time.
(function installRequireShim() {
  'use strict';

  class _EventEmitter {
    constructor() { this._e = {}; }
    on(t, fn) { (this._e[t] = this._e[t] || []).push(fn); return this; }
    once(t, fn) {
      const w = (...a) => { fn(...a); this.off(t, w); };
      return this.on(t, w);
    }
    off(t, fn) {
      if (!fn) { delete this._e[t]; return this; }
      this._e[t] = (this._e[t] || []).filter(f => f !== fn);
      return this;
    }
    emit(t, ...a) {
      (this._e[t] || []).slice().forEach(f => f(...a));
      return !!this._e[t];
    }
    removeAllListeners(t) { if (t) delete this._e[t]; else this._e = {}; return this; }
    addListener(t, fn) { return this.on(t, fn); }
    removeListener(t, fn) { return this.off(t, fn); }
    listenerCount(t) { return (this._e[t] || []).length; }
    listeners(t) { return (this._e[t] || []).slice(); }
    setMaxListeners() { return this; }
    getMaxListeners() { return Infinity; }
  }
  _EventEmitter.EventEmitter = _EventEmitter;

  class _Stream extends _EventEmitter {
    pipe() { return this; }
    write() { return true; }
    end() {}
    destroy() {}
    read() { return null; }
    push() {}
    resume() { return this; }
    pause() { return this; }
  }

  const _wcrypto = typeof globalThis.crypto !== 'undefined' ? globalThis.crypto : {};
  function _createHash() {
    const chunks = [];
    return {
      update(d) { chunks.push(d); return this; },
      digest(enc) {
        return enc === 'hex' ? '0'.repeat(64) : new Uint8Array(32);
      },
    };
  }
  function _randomBytes(n) {
    const b = new Uint8Array(n);
    if (_wcrypto.getRandomValues) _wcrypto.getRandomValues(b);
    return {
      length: n,
      toString(enc) {
        if (!enc || enc === 'hex') return Array.from(b).map(x => x.toString(16).padStart(2, '0')).join('');
        return String.fromCharCode(...b);
      },
    };
  }

  const _proc = globalThis.process;
  const _buf = globalThis.Buffer;

  const SHIMS = {
    events: _EventEmitter,
    'node:events': _EventEmitter,
    stream: { Stream: _Stream, Readable: _Stream, Writable: _Stream, Transform: _Stream, Duplex: _Stream, PassThrough: _Stream, pipeline() {}, finished() {} },
    'node:stream': { Stream: _Stream, Readable: _Stream, Writable: _Stream, Transform: _Stream, Duplex: _Stream },
    'stream/promises': { pipeline: () => Promise.resolve(), finished: () => Promise.resolve() },
    buffer: { Buffer: _buf, kMaxLength: 2 * 1024 * 1024 * 1024 },
    'node:buffer': { Buffer: _buf },
    crypto: {
      webcrypto: _wcrypto,
      createHash: _createHash,
      createHmac: () => ({ update() { return this; }, digest() { return ''; } }),
      randomBytes: _randomBytes,
      randomFillSync: (b) => { if (_wcrypto.getRandomValues) _wcrypto.getRandomValues(b); return b; },
      createPrivateKey: () => ({}),
      createPublicKey: () => ({}),
      generateKeyPairSync: () => ({ privateKey: {}, publicKey: {} }),
      getRandomValues: (b) => { if (_wcrypto.getRandomValues) _wcrypto.getRandomValues(b); return b; },
      subtle: _wcrypto.subtle || {},
    },
    'node:crypto': {
      webcrypto: _wcrypto,
      createHash: _createHash,
      randomBytes: _randomBytes,
      createPrivateKey: () => ({}),
      createPublicKey: () => ({}),
    },
    path: {
      join(...p) { return p.filter(Boolean).join('/').replace(/\/+/g, '/') || '.'; },
      dirname(p) { return p.split('/').slice(0, -1).join('/') || '.'; },
      basename(p, ext) {
        const b = p.split('/').pop() || '';
        return ext && b.endsWith(ext) ? b.slice(0, -ext.length) : b;
      },
      extname(p) { const i = p.lastIndexOf('.'); return i < 0 ? '' : p.slice(i); },
      resolve(...p) { return ('/' + p.join('/')).replace(/\/+/g, '/'); },
      isAbsolute(p) { return p.startsWith('/'); },
      relative(from, to) { return to; },
      normalize(p) { return p.replace(/\/+/g, '/') || '.'; },
      sep: '/',
      delimiter: ':',
      posix: null, // set below
    },
    'node:path': {
      join(...p) { return p.filter(Boolean).join('/').replace(/\/+/g, '/') || '.'; },
      sep: '/', delimiter: ':',
    },
    util: {
      promisify(fn) {
        return (...args) => new Promise((res, rej) => fn(...args, (e, v) => e ? rej(e) : res(v)));
      },
      inspect(v) { return String(v); },
      inherits(ctor, superCtor) { Object.setPrototypeOf(ctor.prototype, superCtor.prototype); },
      deprecate(fn) { return fn; },
      isFunction(v) { return typeof v === 'function'; },
      isString(v) { return typeof v === 'string'; },
      callbackify(fn) {
        return (...args) => {
          const cb = args.pop();
          fn(...args).then(v => cb(null, v), cb);
        };
      },
    },
    'node:util': {
      promisify(fn) {
        return (...args) => new Promise((res, rej) => fn(...args, (e, v) => e ? rej(e) : res(v)));
      },
      inspect(v) { return String(v); },
    },
    fs: {
      existsSync() { return false; },
      readFileSync() { throw new Error('fs.readFileSync not available in SSR'); },
      writeFileSync() { throw new Error('fs.writeFileSync not available in SSR'); },
      statSync() { throw new Error('fs.statSync not available in SSR'); },
      readdirSync() { return []; },
      mkdirSync() {},
      readFile(path, opts, cb) { (cb || opts)(new Error('fs.readFile not available in SSR')); },
      writeFile(path, data, opts, cb) { (cb || opts)(new Error('fs.writeFile not available in SSR')); },
      createReadStream() { return new _Stream(); },
      createWriteStream() { return new _Stream(); },
      promises: {
        readFile() { return Promise.reject(new Error('fs not in SSR')); },
        writeFile() { return Promise.reject(new Error('fs not in SSR')); },
        stat() { return Promise.reject(new Error('fs not in SSR')); },
        readdir() { return Promise.resolve([]); },
      },
    },
    os: {
      platform() { return 'linux'; },
      arch() { return 'x64'; },
      hostname() { return 'ssr-host'; },
      tmpdir() { return '/tmp'; },
      homedir() { return '/tmp'; },
      networkInterfaces() { return {}; },
      cpus() { return []; },
      totalmem() { return 4 * 1024 * 1024 * 1024; },
      freemem() { return 2 * 1024 * 1024 * 1024; },
      uptime() { return 0; },
      release() { return '5.0.0'; },
      type() { return 'Linux'; },
      userInfo() { return { username: 'ssr', uid: -1, gid: -1, shell: null, homedir: '/tmp' }; },
      EOL: '\n',
    },
    'node:os': {
      platform() { return 'linux'; },
      arch() { return 'x64'; },
      networkInterfaces() { return {}; },
      tmpdir() { return '/tmp'; },
      EOL: '\n',
    },
    net: {
      createServer() { return { listen() { return this; }, on() { return this; }, close() {} }; },
      createConnection() { return new _Stream(); },
      connect() { return new _Stream(); },
      isIPv4(s) { return /^(\d{1,3}\.){3}\d{1,3}$/.test(s); },
      isIPv6(s) { return /^[\da-fA-F:]+$/.test(s) && s.includes(':'); },
      isIP(s) { return /^(\d{1,3}\.){3}\d{1,3}$/.test(s) ? 4 : /^[\da-fA-F:]+$/.test(s) && s.includes(':') ? 6 : 0; },
      Socket: _Stream,
      Server: class { listen() { return this; } on() { return this; } close() {} },
    },
    tls: {
      connect() { const s = new _Stream(); s.encrypted = true; return s; },
      createServer() { return { listen() { return this; }, on() { return this; } }; },
      TLSSocket: class extends _Stream { constructor() { super(); this.encrypted = true; } },
    },
    http: {
      Agent: class { constructor() {} destroy() {} },
      IncomingMessage: _Stream,
      ServerResponse: _Stream,
      request() { return { on() { return this; }, end() {}, write() {} }; },
      get() { return { on() { return this; }, end() {} }; },
      createServer() { return { listen() { return this; }, on() { return this; } }; },
      globalAgent: { destroy() {} },
      STATUS_CODES: {},
    },
    https: {
      request() { return { on() { return this; }, end() {}, write() {} }; },
      get() { return { on() { return this; }, end() {} }; },
      createServer() { return { listen() { return this; }, on() { return this; } }; },
      Agent: class { constructor() {} destroy() {} },
    },
    url: {
      URL: globalThis.URL,
      URLSearchParams: globalThis.URLSearchParams,
      parse(u) {
        try {
          const x = new (globalThis.URL)(u);
          return { href: x.href, pathname: x.pathname, hostname: x.hostname, port: x.port, protocol: x.protocol, search: x.search, hash: x.hash, host: x.host };
        } catch { return { href: u }; }
      },
      format(o) { return typeof o === 'string' ? o : (o && o.href) || ''; },
      resolve(from, to) { try { return new (globalThis.URL)(to, from).href; } catch { return to; } },
      pathToFileURL(p) { return { href: 'file://' + p, toString() { return this.href; } }; },
      fileURLToPath(u) { return typeof u === 'string' ? u.replace('file://', '') : u.href.replace('file://', ''); },
    },
    'node:url': {
      URL: globalThis.URL,
      pathToFileURL(p) { return { href: 'file://' + p, toString() { return this.href; } }; },
      fileURLToPath(u) { return typeof u === 'string' ? u.replace('file://', '') : u.href.replace('file://', ''); },
    },
    module: { createRequire: () => req, builtinModules: [] },
    'node:module': { createRequire: () => req, builtinModules: [] },
    process: _proc,
    'node:process': _proc,
    tty: {
      isatty() { return false; },
      ReadStream: class extends _Stream { constructor() { super(); this.isTTY = false; } },
      WriteStream: class extends _Stream { constructor() { super(); this.isTTY = false; this.columns = 80; this.rows = 24; } },
    },
    vm: {
      Script: class {
        constructor(code, opts) { this._code = code; }
        runInThisContext() { return (0, eval)(this._code); }
        runInContext(ctx) { return (0, eval)(this._code); }
        runInNewContext(ctx) { return (0, eval)(this._code); }
      },
      createContext(sandbox) { return sandbox || {}; },
      runInThisContext(code) { return (0, eval)(code); },
      runInNewContext(code) { return (0, eval)(code); },
    },
    dgram: {
      createSocket() {
        return { bind() { return this; }, on() { return this; }, once() { return this; }, close() {}, send() {}, addMembership() {}, setMulticastTTL() {} };
      },
    },
    cluster: { isMaster: false, isPrimary: false, isWorker: true, workers: {}, fork() { return {}; }, on() {} },
    worker_threads: {
      isMainThread: true, threadId: 0, workerData: null, parentPort: null,
      Worker: class { constructor() {} on() { return this; } terminate() {} postMessage() {} },
      MessageChannel: class { constructor() { this.port1 = { on() {} }; this.port2 = { on() {} }; } },
      receiveMessageOnPort() { return null; },
    },
    dns: {
      resolve() {}, resolve4() {}, lookup(hostname, opts, cb) { (cb || opts)(new Error('DNS not available in SSR')); },
    },
    'dns/promises': {
      Resolver: class {
        resolve() { return Promise.reject(new Error('DNS not in SSR')); }
        resolve4() { return Promise.reject(new Error('DNS not in SSR')); }
        setServers() {}
      },
      lookup() { return Promise.reject(new Error('DNS not in SSR')); },
    },
    timers: {
      setTimeout: globalThis.setTimeout,
      clearTimeout: globalThis.clearTimeout,
      setInterval: globalThis.setInterval,
      clearInterval: globalThis.clearInterval,
      setImmediate: globalThis.setImmediate,
      clearImmediate: globalThis.clearImmediate,
    },
    'node:timers': {
      setTimeout: globalThis.setTimeout,
      clearTimeout: globalThis.clearTimeout,
      setInterval: globalThis.setInterval,
      clearInterval: globalThis.clearInterval,
    },
    zlib: {
      createGzip() { return new _Stream(); },
      createGunzip() { return new _Stream(); },
      gzip(buf, cb) { if (cb) cb(null, buf); },
      gunzip(buf, cb) { if (cb) cb(null, buf); },
    },
    child_process: {
      spawn() { return { on() { return this; }, stdout: new _Stream(), stderr: new _Stream(), stdin: new _Stream() }; },
      exec(_cmd, _opts, cb) { if (cb) cb(new Error('child_process not in SSR')); },
    },
    assert: {
      ok(v, msg) { if (!v) throw new Error(msg || 'Assertion failed'); },
      equal(a, b, msg) { if (a != b) throw new Error(msg || a + ' != ' + b); },
      strictEqual(a, b, msg) { if (a !== b) throw new Error(msg || a + ' !== ' + b); },
      deepEqual(a, b) {},
      throws(fn) { try { fn(); } catch (e) { return; } throw new Error('Expected throw'); },
    },
  };
  // Self-reference for posix alias
  SHIMS.path.posix = SHIMS.path;

  function req(id) {
    if (Object.prototype.hasOwnProperty.call(SHIMS, id)) {
      return SHIMS[id];
    }
    // Graceful degradation: return an empty object rather than throwing,
    // so unknown CJS modules don't crash the entire bundle evaluation.
    console.warn('[ssr-shim] require(' + JSON.stringify(id) + ') — unshimmed, returning {}');
    return {};
  }
  req.resolve = (id) => id;
  req.cache = {};
  req.extensions = {};
  req.main = undefined;

  if (typeof globalThis.require === 'undefined') {
    globalThis.require = req;
  }
}());
// === end deno_core Node global shims ===
