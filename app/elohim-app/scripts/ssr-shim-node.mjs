#!/usr/bin/env node
/**
 * ssr-shim-node.mjs
 *
 * Post-build script that makes Angular's SSR server bundle loadable in
 * deno_core's FsModuleLoader (which can only resolve file:// URLs and
 * relative paths — bare Node specifiers like "crypto", "stream", etc. fail).
 *
 * Steps:
 * 1. Write minimal shim .mjs files for each Node built-in used by the bundle
 * 2. Rewrite all bare Node specifier imports in server/*.mjs to point to the
 *    shims via relative paths (./node-shims/crypto.mjs, etc.)
 *
 * The shims expose only the shape needed to avoid import-time crashes during
 * SSR rendering. The Holochain WebSocket client, libp2p, and Helia code paths
 * are never exercised during a server-side render — Angular just needs to
 * construct the DI tree and render HTML. The shims are intentional no-ops.
 *
 * This script is run after `ng build` via the `postbuild` hook in package.json.
 */

import { readdir, readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, dirname, relative } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SERVER_DIR = join(__dirname, '..', 'dist', 'elohim-app', 'server');
const SHIMS_DIR = join(SERVER_DIR, 'node-shims');

// ---------------------------------------------------------------------------
// Shim definitions
// Each key is the bare Node specifier that appears in import statements.
// The value is the ESM shim content that replaces it.
// These are intentional no-ops — the SSR render path never calls them.
// ---------------------------------------------------------------------------
const SHIMS = {
  crypto: `
// crypto shim — delegates to Web Crypto API where possible
const webCrypto = globalThis.crypto ?? {};
const webcrypto = webCrypto;

function createHash(algorithm) {
  return {
    _data: [],
    update(data) { this._data.push(data); return this; },
    digest(enc) {
      // No-op hash — not called during SSR
      return enc === 'hex' ? '0'.repeat(64) : new Uint8Array(32);
    },
  };
}

function createHmac() {
  return {
    update() { return this; },
    digest() { return ''; },
  };
}

function randomBytes(n) {
  const buf = new Uint8Array(n);
  (webCrypto.getRandomValues ?? (() => {}))(buf);
  return { toString: () => Array.from(buf).map(b => b.toString(16).padStart(2, '0')).join('') };
}

function createPrivateKey() { return {}; }
function createPublicKey() { return {}; }
function generateKeyPairSync() { return { privateKey: {}, publicKey: {} }; }

export default {
  webcrypto, createHash, createHmac, randomBytes,
  createPrivateKey, createPublicKey, generateKeyPairSync,
  getRandomValues: (buf) => (webCrypto.getRandomValues ?? (() => {}))(buf),
};
export { webcrypto, createHash, createHmac, randomBytes, createPrivateKey, createPublicKey };
`,

  stream: `
// stream shim — minimal Duplex stub for import-time compatibility
class Duplex {
  constructor() {}
  pipe() { return this; }
  on() { return this; }
  once() { return this; }
  off() { return this; }
  emit() { return false; }
  write() { return true; }
  end() {}
  destroy() {}
}

export { Duplex };
export default { Duplex };
`,

  buffer: `
// buffer shim — delegates to globalThis.Buffer if available, else Uint8Array wrapper
const Buffer = globalThis.Buffer ?? {
  from(data, enc) {
    if (typeof data === 'string') {
      return new TextEncoder().encode(data);
    }
    return new Uint8Array(data);
  },
  alloc(n) { return new Uint8Array(n); },
  allocUnsafe(n) { return new Uint8Array(n); },
  isBuffer(v) { return v instanceof Uint8Array; },
  concat(bufs) {
    const len = bufs.reduce((s, b) => s + b.length, 0);
    const out = new Uint8Array(len);
    let off = 0;
    for (const b of bufs) { out.set(b, off); off += b.length; }
    return out;
  },
};

export { Buffer };
export default { Buffer };
`,

  path: `
// path shim — minimal POSIX path utilities
function join(...parts) { return parts.join('/').replace(/\/+/g, '/'); }
function dirname(p) { return p.split('/').slice(0, -1).join('/') || '.'; }
function basename(p, ext) {
  const b = p.split('/').pop() ?? '';
  return ext && b.endsWith(ext) ? b.slice(0, -ext.length) : b;
}
function extname(p) { const i = p.lastIndexOf('.'); return i < 0 ? '' : p.slice(i); }
function resolve(...parts) { return join(...parts); }
function isAbsolute(p) { return p.startsWith('/'); }
function relative(from, to) { return to; }
function normalize(p) { return p.replace(/\/+/g, '/'); }

const posix = { join, dirname, basename, extname, resolve, isAbsolute, relative, normalize };

export { join, dirname, basename, extname, resolve, isAbsolute, relative, normalize, posix };
export default { join, dirname, basename, extname, resolve, isAbsolute, relative, normalize, posix, sep: '/', delimiter: ':' };
`,

  util: `
// util shim — minimal promisify + inspect
function promisify(fn) {
  return (...args) => new Promise((resolve, reject) => {
    fn(...args, (err, val) => err ? reject(err) : resolve(val));
  });
}
function inspect(v) { return String(v); }
function inherits(ctor, superCtor) { Object.setPrototypeOf(ctor.prototype, superCtor.prototype); }
function deprecate(fn) { return fn; }
function isFunction(v) { return typeof v === 'function'; }
function isString(v) { return typeof v === 'string'; }

export { promisify, inspect, inherits, deprecate, isFunction, isString };
export default { promisify, inspect, inherits, deprecate, isFunction, isString };
`,

  fs: `
// fs shim — no-op file system (SSR renderer never reads from disk via this)
const noOp = () => {};
const noOpCb = (...args) => { const cb = args[args.length-1]; if (typeof cb === 'function') cb(new Error('fs not available in SSR')); };

export const readFile = (...a) => noOpCb(...a);
export const writeFile = (...a) => noOpCb(...a);
export const stat = (...a) => noOpCb(...a);
export const statSync = () => { throw new Error('fs.statSync not available in SSR'); };
export const existsSync = () => false;
export const readFileSync = () => { throw new Error('fs.readFileSync not available in SSR'); };
export const promises = {
  readFile: () => Promise.reject(new Error('fs not available in SSR')),
  writeFile: () => Promise.reject(new Error('fs not available in SSR')),
};

export default {
  readFile, writeFile, stat, statSync, existsSync, readFileSync, promises,
};
`,

  os: `
// os shim — returns safe defaults
function platform() { return 'linux'; }
function arch() { return 'x64'; }
function hostname() { return 'ssr-host'; }
function tmpdir() { return '/tmp'; }
function homedir() { return '/tmp'; }
function networkInterfaces() { return {}; }
function cpus() { return []; }
function totalmem() { return 4 * 1024 * 1024 * 1024; }
function freemem() { return 2 * 1024 * 1024 * 1024; }
function uptime() { return 0; }
function release() { return '0.0.0'; }
function type() { return 'Linux'; }
function userInfo() { return { username: 'ssr', uid: -1, gid: -1, shell: null, homedir: '/tmp' }; }

export { platform, arch, hostname, tmpdir, homedir, networkInterfaces, cpus, totalmem, freemem, uptime, release, type, userInfo };
export default { platform, arch, hostname, tmpdir, homedir, networkInterfaces, cpus, totalmem, freemem, uptime, release, type, userInfo, EOL: '\\n' };
`,

  net: `
// net shim — no-op network stubs
class Socket {
  constructor() {}
  connect() { return this; }
  on() { return this; }
  once() { return this; }
  write() { return true; }
  end() {}
  destroy() {}
}

class Server {
  constructor() {}
  listen() { return this; }
  on() { return this; }
  close() {}
}

function createServer() { return new Server(); }
function createConnection() { return new Socket(); }
function isIPv4(s) { return /^(\d{1,3}\.){3}\d{1,3}$/.test(s); }
function isIPv6(s) { return s.includes(':'); }
function isIP(s) { return isIPv4(s) ? 4 : isIPv6(s) ? 6 : 0; }

export { Socket, Server, createServer, createConnection, isIPv4, isIPv6, isIP };
export { isIP as ipVersion };
export default { Socket, Server, createServer, createConnection, isIPv4, isIPv6, isIP };
`,

  tls: `
// tls shim — no-op TLS stubs
class TLSSocket {
  constructor() {}
  on() { return this; }
  once() { return this; }
  write() { return true; }
  end() {}
  destroy() {}
  get encrypted() { return true; }
}

function connect() { return new TLSSocket(); }
function createServer() { return {}; }

export { TLSSocket, connect, createServer };
export default { TLSSocket, connect, createServer };
`,

  http: `
// http shim — no-op HTTP stubs
class Agent {
  constructor() {}
  destroy() {}
}

class IncomingMessage {}
class ServerResponse {}

function request() { return { on() { return this; }, end() {}, write() {} }; }
function createServer() { return { listen() { return this; }, on() { return this; } }; }

export { Agent, IncomingMessage, ServerResponse, request, createServer };
export { Agent as NodeAgent };
export default { Agent, IncomingMessage, ServerResponse, request, createServer, globalAgent: new Agent() };
`,

  https: `
// https shim — no-op HTTPS stubs
function request() { return { on() { return this; }, end() {}, write() {} }; }
function get() { return request(); }
function createServer() { return { listen() { return this; }, on() { return this; } }; }

export { request, get, createServer };
export default { request, get, createServer };
`,

  events: `
// events shim — minimal EventEmitter
class EventEmitter {
  constructor() { this._events = {}; }
  on(event, fn) { (this._events[event] ??= []).push(fn); return this; }
  once(event, fn) {
    const wrapper = (...a) => { fn(...a); this.off(event, wrapper); };
    return this.on(event, wrapper);
  }
  off(event, fn) {
    if (!fn) { delete this._events[event]; return this; }
    this._events[event] = (this._events[event] ?? []).filter(f => f !== fn);
    return this;
  }
  emit(event, ...args) {
    (this._events[event] ?? []).forEach(fn => fn(...args));
    return (this._events[event]?.length ?? 0) > 0;
  }
  removeAllListeners(event) {
    if (event) delete this._events[event];
    else this._events = {};
    return this;
  }
  addListener(event, fn) { return this.on(event, fn); }
  removeListener(event, fn) { return this.off(event, fn); }
  listenerCount(event) { return this._events[event]?.length ?? 0; }
  listeners(event) { return [...(this._events[event] ?? [])]; }
  setMaxListeners() { return this; }
  getMaxListeners() { return Infinity; }
}

function setMaxListeners() {}
function on(emitter, event) {
  // Returns an async iterable — return empty one
  return { [Symbol.asyncIterator]() { return { async next() { return { done: true, value: undefined }; } }; } };
}

export { EventEmitter, setMaxListeners, on };
export { EventEmitter as EventEmitter2 };
export default EventEmitter;
`,

  url: `
// url shim — delegates to WHATWG URL (available in modern runtimes)
const URL2 = globalThis.URL;
const URLSearchParams2 = globalThis.URLSearchParams;
function parse(urlStr) {
  try { const u = new URL2(urlStr); return { href: u.href, pathname: u.pathname, hostname: u.hostname, port: u.port, protocol: u.protocol, search: u.search, hash: u.hash }; }
  catch { return { href: urlStr }; }
}
function format(obj) { return typeof obj === 'string' ? obj : obj?.href ?? ''; }
function resolve(from, to) { try { return new URL2(to, from).href; } catch { return to; } }
function pathToFileURL(p) { return { href: 'file://' + p, toString() { return this.href; } }; }
function fileURLToPath(u) { return typeof u === 'string' ? u.replace('file://', '') : u.href.replace('file://', ''); }

export { URL2 as URL, URLSearchParams2 as URLSearchParams, parse, format, resolve, pathToFileURL, fileURLToPath };
export default { URL: URL2, URLSearchParams: URLSearchParams2, parse, format, resolve, pathToFileURL, fileURLToPath };
`,

  module: `
// module shim — minimal module utilities
function createRequire() {
  return function require(id) {
    throw new Error('require() not available in SSR runtime: ' + id);
  };
}

class Module {
  constructor() {}
  static _resolveFilename() { return ''; }
}

export { createRequire, Module };
export default { createRequire, Module, builtinModules: [] };
`,

  dgram: `
// dgram shim — no-op UDP socket
class Socket {
  constructor() {}
  bind() { return this; }
  on() { return this; }
  close() {}
  send() {}
}

function createSocket() { return new Socket(); }
export { createSocket };
export default { createSocket };
`,

  cluster: `
// cluster shim — always reports as worker
export const isMaster = false;
export const isWorker = true;
export const workers = {};
export function fork() { return {}; }
export function on() {}
export default { isMaster, isWorker, workers, fork, on };
`,

  worker_threads: `
// worker_threads shim — stub for SSR environment
export const isMainThread = true;
export const threadId = 0;
export const workerData = null;
export const parentPort = null;
export class Worker { constructor() {} on() { return this; } terminate() {} }
export class MessageChannel { constructor() { this.port1 = {}; this.port2 = {}; } }
export default { isMainThread, threadId, workerData, parentPort, Worker, MessageChannel };
`,

  vm: `
// vm shim — minimal script execution stub
class Script {
  constructor(code) { this._code = code; }
  runInThisContext() { return (0, eval)(this._code); }
  runInContext() { return (0, eval)(this._code); }
}

function createContext(sandbox) { return sandbox; }
function runInThisContext(code) { return (0, eval)(code); }
function runInNewContext(code, sandbox) { return (0, eval)(code); }

export { Script, createContext, runInThisContext, runInNewContext };
export default { Script, createContext, runInThisContext, runInNewContext };
`,

  tty: `
// tty shim — always reports non-TTY
export function isatty() { return false; }
export class ReadStream { isTTY = false; }
export class WriteStream { isTTY = false; columns = 80; rows = 24; }
export default { isatty, ReadStream, WriteStream };
`,

  process: `
// process shim — minimal process object for SSR
const proc = {
  env: {},
  argv: [],
  argv0: 'node',
  platform: 'linux',
  version: 'v18.0.0',
  versions: {},
  pid: 1,
  ppid: 0,
  arch: 'x64',
  cwd() { return '/'; },
  exit(code) { throw new Error('process.exit(' + code + ') called in SSR'); },
  on() { return this; },
  off() { return this; },
  once() { return this; },
  emit() { return false; },
  nextTick(fn, ...args) { Promise.resolve().then(() => fn(...args)); },
  hrtime(prev) { const n = Date.now() * 1e6; if (prev) return [Math.floor((n - (prev[0]*1e9+prev[1]))/1e9), (n - (prev[0]*1e9+prev[1])) % 1e9]; return [Math.floor(n/1e9), n % 1e9]; },
  uptime() { return 0; },
  memoryUsage() { return { rss: 0, heapTotal: 0, heapUsed: 0, external: 0, arrayBuffers: 0 }; },
  stdout: { write() {}, on() { return this; } },
  stderr: { write() {}, on() { return this; } },
  stdin: { on() { return this; }, resume() {} },
};

export default proc;
export const env = proc.env;
export const argv = proc.argv;
export const platform = proc.platform;
export const version = proc.version;
export const versions = proc.versions;
export const pid = proc.pid;
`,

  'dns/promises': `
// dns/promises shim — no-op DNS resolver
class Resolver {
  constructor() {}
  resolve() { return Promise.reject(new Error('DNS not available in SSR')); }
  resolve4() { return Promise.reject(new Error('DNS not available in SSR')); }
  resolve6() { return Promise.reject(new Error('DNS not available in SSR')); }
  resolveTxt() { return Promise.reject(new Error('DNS not available in SSR')); }
  setServers() {}
}

export { Resolver };
export default { Resolver };
`,

  'fs/promises': `
// fs/promises shim — Helia/IPFS uses this for local filesystem access.
// In SSR, filesystem operations are not available; all calls return rejections.
// This shim prevents FsModuleLoader's ImportPrefixMissing error when the
// Helia chunk is loaded as a static ESM import (even inside a try-catch call site,
// the module itself must be loadable before the specifier error fires).
export function open() { return Promise.reject(new Error('fs/promises not available in SSR')); }
export function readFile() { return Promise.reject(new Error('fs/promises not available in SSR')); }
export function writeFile() { return Promise.reject(new Error('fs/promises not available in SSR')); }
export function stat() { return Promise.reject(new Error('fs/promises not available in SSR')); }
export function readdir() { return Promise.resolve([]); }
export function mkdir() { return Promise.resolve(); }
export function rm() { return Promise.resolve(); }
export function unlink() { return Promise.resolve(); }
export function rename() { return Promise.resolve(); }
export function copyFile() { return Promise.resolve(); }
export function access() { return Promise.reject(new Error('fs/promises not available in SSR')); }
export default { open, readFile, writeFile, stat, readdir, mkdir, rm, unlink, rename, copyFile, access };
`,
};

// ---------------------------------------------------------------------------
// Globals preamble — loaded from ssr-globals-preamble.mjs (separate file so
// backslash sequences and template literals don't get mangled by embedding).
// ---------------------------------------------------------------------------
const GLOBALS_PREAMBLE = await readFile(join(__dirname, 'ssr-globals-preamble.mjs'), 'utf8');

// ---------------------------------------------------------------------------
// DEAD CODE — retained only as documentation; actual preamble is the file above.
// The original inline preamble was extracted to avoid template-literal escaping
// issues (backslashes in regex patterns and \n in strings were being consumed).
// ---------------------------------------------------------------------------
const _DEAD_PREAMBLE = `
// === deno_core Node global shims ===
// Injected by ssr-shim-node.mjs post-build script.
// These shims satisfy bare global references in Zone.js and bundled packages
// that assume a Node environment. None of these code paths execute during
// the actual SSR render — they only need to survive module evaluation.

if (typeof globalThis.process === 'undefined') {
  globalThis.process = {
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
    nextTick(fn, ...args) { Promise.resolve().then(() => fn(...args)); },
    hrtime(prev) {
      const n = Date.now() * 1e6;
      if (prev) return [Math.floor((n - (prev[0]*1e9+prev[1]))/1e9), (n - (prev[0]*1e9+prev[1])) % 1e9];
      return [Math.floor(n/1e9), n % 1e9];
    },
    uptime() { return 0; },
    memoryUsage() { return { rss: 0, heapTotal: 0, heapUsed: 0, external: 0, arrayBuffers: 0 }; },
    stdout: { write(s) {}, on() { return this; }, isTTY: false },
    stderr: { write(s) {}, on() { return this; }, isTTY: false },
    stdin: { on() { return this; }, resume() {}, isTTY: false },
  };
}

if (typeof globalThis.Buffer === 'undefined') {
  // Minimal Buffer shim — wraps Uint8Array
  class Buffer extends Uint8Array {
    static from(data, enc) {
      if (typeof data === 'string') return new Uint8Array(new TextEncoder().encode(data));
      return new Uint8Array(data);
    }
    static alloc(n) { return new Uint8Array(n); }
    static allocUnsafe(n) { return new Uint8Array(n); }
    static isBuffer(v) { return v instanceof Uint8Array; }
    static concat(bufs) {
      const len = bufs.reduce((s, b) => s + b.length, 0);
      const out = new Uint8Array(len);
      let off = 0;
      for (const b of bufs) { out.set(b, off); off += b.length; }
      return out;
    }
    toString(enc) {
      if (!enc || enc === 'utf8' || enc === 'utf-8') return new TextDecoder().decode(this);
      if (enc === 'hex') return Array.from(this).map(b => b.toString(16).padStart(2,'0')).join('');
      if (enc === 'base64') return btoa(String.fromCharCode(...this));
      return new TextDecoder().decode(this);
    }
  }
  globalThis.Buffer = Buffer;
}

if (typeof globalThis.global === 'undefined') {
  globalThis.global = globalThis;
}

// Synchronous require() shim for CJS packages bundled via __commonJS wrappers.
// The ws/isomorphic-ws CJS path calls require('events'), require('stream'), etc.
// We need a synchronous return value — actual ESM shim modules can't be require()'d,
// so we return equivalent plain-object stubs here.
(function installRequireShim() {
  class _EventEmitter {
    constructor() { this._events = {}; }
    on(e, fn) { (this._events[e] ??= []).push(fn); return this; }
    once(e, fn) { const w = (...a) => { fn(...a); this.off(e, w); }; return this.on(e, w); }
    off(e, fn) { if (!fn) { delete this._events[e]; return this; } this._events[e] = (this._events[e]||[]).filter(f=>f!==fn); return this; }
    emit(e, ...a) { (this._events[e]||[]).forEach(f=>f(...a)); return !!(this._events[e]?.length); }
    removeAllListeners(e) { if (e) delete this._events[e]; else this._events={}; return this; }
    addListener(e, fn) { return this.on(e, fn); }
    removeListener(e, fn) { return this.off(e, fn); }
    listenerCount(e) { return this._events[e]?.length ?? 0; }
    listeners(e) { return [...(this._events[e]||[])]; }
    setMaxListeners() { return this; }
    getMaxListeners() { return Infinity; }
    static EventEmitter = null; // set below
  }
  _EventEmitter.EventEmitter = _EventEmitter;

  class _Duplex {
    pipe() { return this; } on() { return this; } once() { return this; }
    off() { return this; } emit() { return false; } write() { return true; }
    end() {} destroy() {}
  }

  const _webCrypto = globalThis.crypto ?? {};
  function _createHash() {
    return { update() { return this; }, digest(enc) { return enc === 'hex' ? '0'.repeat(64) : new Uint8Array(32); } };
  }
  function _randomBytes(n) {
    const b = new Uint8Array(n);
    (_webCrypto.getRandomValues ?? (() => {}))(b);
    return { toString() { return Array.from(b).map(x=>x.toString(16).padStart(2,'0')).join(''); } };
  }

  const _process = globalThis.process;

  const REQUIRE_SHIMS = {
    events: _EventEmitter,
    'node:events': _EventEmitter,
    stream: { Duplex: _Duplex, Readable: _Duplex, Writable: _Duplex, Transform: _Duplex, Stream: _Duplex },
    'node:stream': { Duplex: _Duplex, Readable: _Duplex, Writable: _Duplex },
    buffer: { Buffer: globalThis.Buffer },
    'node:buffer': { Buffer: globalThis.Buffer },
    crypto: { createHash: _createHash, randomBytes: _randomBytes, webcrypto: _webCrypto, createPrivateKey: ()=>({}), createPublicKey: ()=>({}) },
    'node:crypto': { createHash: _createHash, randomBytes: _randomBytes, webcrypto: _webCrypto },
    path: { join: (...p) => p.join('/'), dirname: p => p.split('/').slice(0,-1).join('/'), basename: p => p.split('/').pop(), extname: p => { const i=p.lastIndexOf('.'); return i<0?'':p.slice(i); }, resolve: (...p) => p.join('/'), isAbsolute: p => p.startsWith('/'), sep: '/', delimiter: ':' },
    'node:path': { join: (...p) => p.join('/'), sep: '/', delimiter: ':' },
    util: { promisify: fn => (...a) => new Promise((res,rej) => fn(...a,(e,v) => e?rej(e):res(v))), inspect: v => String(v), inherits: (c,s) => Object.setPrototypeOf(c.prototype, s.prototype) },
    'node:util': { promisify: fn => (...a) => new Promise((res,rej) => fn(...a,(e,v) => e?rej(e):res(v))), inspect: String },
    fs: { existsSync: ()=>false, readFileSync: ()=>{ throw new Error('fs.readFileSync not available in SSR'); }, promises: { readFile: ()=>Promise.reject(new Error('fs not in SSR')), writeFile: ()=>Promise.reject(new Error('fs not in SSR')) } },
    os: { platform: ()=>'linux', arch: ()=>'x64', hostname: ()=>'ssr', tmpdir: ()=>'/tmp', networkInterfaces: ()=>({}), cpus: ()=>[], EOL: '\n' },
    'node:os': { platform: ()=>'linux', arch: ()=>'x64', networkInterfaces: ()=>({}), EOL: '\n' },
    net: { createServer: ()=>({ listen(){return this;}, on(){return this;} }), createConnection: ()=>({ on(){return this;}, write(){}, end(){}, destroy(){} }), isIPv4: s=>/^(\d{1,3}\.){3}\d{1,3}$/.test(s), isIPv6: s=>s.includes(':'), isIP: s=>/^(\d{1,3}\.){3}\d{1,3}$/.test(s)?4:s.includes(':')?6:0 },
    tls: { connect: ()=>({ on(){return this;}, write(){}, end(){}, destroy(){}, encrypted:true }), TLSSocket: class { on(){return this;} write(){return true;} end(){} destroy(){} get encrypted(){return true;} } },
    http: { Agent: class { destroy(){} }, request: ()=>({ on(){return this;}, end(){}, write(){} }), createServer: ()=>({ listen(){return this;}, on(){return this;} }), globalAgent: { destroy(){} } },
    https: { request: ()=>({ on(){return this;}, end(){}, write(){} }), get: ()=>({ on(){return this;}, end(){} }) },
    url: { parse: u=>{ try{const x=new URL(u);return{href:x.href,pathname:x.pathname,hostname:x.hostname,port:x.port,protocol:x.protocol,search:x.search};}catch{return{href:u};} }, format: o=>typeof o==='string'?o:o?.href??'', pathToFileURL: p=>({href:'file://'+p,toString(){return this.href;}}), fileURLToPath: u=>typeof u==='string'?u.replace('file://',''):u.href.replace('file://','') },
    'node:url': { pathToFileURL: p=>({href:'file://'+p,toString(){return this.href;}}), fileURLToPath: u=>typeof u==='string'?u.replace('file://',''):u.href.replace('file://','') },
    module: { createRequire: ()=>req },
    'node:module': { createRequire: ()=>req },
    process: _process,
    'node:process': _process,
    tty: { isatty: ()=>false },
    vm: { Script: class { constructor(c){this._c=c;} runInThisContext(){return (0,eval)(this._c);} }, createContext: s=>s, runInThisContext: c=>(0,eval)(c) },
    dgram: { createSocket: ()=>({ bind(){return this;}, on(){return this;}, close(){}, send(){} }) },
    cluster: { isMaster: false, isWorker: true, workers: {} },
    worker_threads: { isMainThread: true, threadId: 0 },
  };

  function req(id) {
    if (Object.prototype.hasOwnProperty.call(REQUIRE_SHIMS, id)) {
      return REQUIRE_SHIMS[id];
    }
    throw new Error('require() not available in SSR runtime — unshimmed module: ' + id);
  }
  req.resolve = (id) => id;

  // Install globally — polyfills.server.mjs banner does globalThis.require ??= createRequire(...)
  // We install BEFORE that banner runs by prepending this preamble, so ??= is a no-op.
  if (typeof globalThis.require === 'undefined') {
    globalThis.require = req;
  }
}());

// Timer shims — deno_core bare runtime has no setTimeout/clearTimeout.
// Zone.js and bundled packages reference these as globals. We provide
// minimal stub implementations based on Promise microtasks.
// These stubs are intentionally non-functional for actual async timing —
// they exist only so module-level references don't throw at eval time.
if (typeof globalThis.setTimeout === 'undefined') {
  let _timerId = 1;
  const _timers = new Map();

  globalThis.setTimeout = function(fn, delay, ...args) {
    const id = _timerId++;
    // Use Promise microtask for zero-delay or very short delays
    Promise.resolve().then(() => {
      if (_timers.has(id)) {
        _timers.delete(id);
        if (typeof fn === 'function') fn(...args);
        else if (typeof fn === 'string') (0, eval)(fn);
      }
    });
    _timers.set(id, fn);
    return id;
  };

  globalThis.clearTimeout = function(id) {
    _timers.delete(id);
  };

  globalThis.setInterval = function(fn, delay, ...args) {
    // Stub: fire once only (intervals are not meaningful in SSR)
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
// === end deno_core Node global shims ===
`.trimStart();  // _DEAD_PREAMBLE ends here

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------
async function main() {
  console.log('[ssr-shim-node] Creating Node built-in shims for deno_core compatibility...');

  // Ensure shims directory exists
  await mkdir(SHIMS_DIR, { recursive: true });

  // Write shim files
  for (const [specifier, content] of Object.entries(SHIMS)) {
    // Handle sub-path specifiers like "dns/promises" → "dns__promises.mjs"
    const filename = specifier.replace(/\//g, '__') + '.mjs';
    const shimPath = join(SHIMS_DIR, filename);
    await writeFile(shimPath, content.trimStart(), 'utf8');
    console.log(`  wrote shim: node-shims/${filename}`);
  }

  // Inject globals preamble into polyfills.server.mjs (the first module
  // evaluated by the bundle — sets process, Buffer, global, setImmediate
  // as globalThis properties before Zone.js and other code runs).
  const polyfillsPath = join(SERVER_DIR, 'polyfills.server.mjs');
  const polyfillsContent = await readFile(polyfillsPath, 'utf8');
  if (!polyfillsContent.startsWith('// === deno_core Node global shims ===')) {
    await writeFile(polyfillsPath, GLOBALS_PREAMBLE + polyfillsContent, 'utf8');
    console.log('  injected globals preamble into polyfills.server.mjs');
  } else {
    console.log('  polyfills.server.mjs already has globals preamble');
  }

  // Build the full rewrite map: both bare specifiers ("crypto") AND
  // their node:-prefixed equivalents ("node:crypto") point to the same shim.
  // This covers Angular's polyfills banner which emits `import ... from 'node:module'`.
  const rewriteMap = new Map();
  for (const specifier of Object.keys(SHIMS)) {
    const shimFilename = specifier.replace(/\//g, '__') + '.mjs';
    const shimRelPath = `./node-shims/${shimFilename}`;
    rewriteMap.set(specifier, shimRelPath);
    // node: prefix alias — "node:crypto" → same shim as "crypto"
    rewriteMap.set(`node:${specifier}`, shimRelPath);
  }

  // Find all server .mjs files (excluding our shims directory)
  const allFiles = await readdir(SERVER_DIR);
  const mjsFiles = allFiles.filter(f => f.endsWith('.mjs'));

  let totalReplacements = 0;

  for (const filename of mjsFiles) {
    const filePath = join(SERVER_DIR, filename);
    let content = await readFile(filePath, 'utf8');
    let fileReplacements = 0;

    for (const [specifier, shimRelPath] of rewriteMap) {
      // Escape special regex chars in specifier
      const escaped = specifier.replace(/[/\\^$*+?.()|[\]{}]/g, '\\$&');

      // Match both:
      //   import X from "specifier"
      //   import { X, Y } from "specifier"
      //   import * as X from "specifier"
      // and both " and ' quotes, with or without a space before the quote
      // (esbuild production builds emit minified `from"specifier"` without a space).
      const dqPattern = new RegExp(`from\\s*"${escaped}"`, 'g');
      const sqPattern = new RegExp(`from\\s*'${escaped}'`, 'g');

      const before = content;
      // Preserve the original spacing: if the match had no space, emit no space.
      content = content.replace(dqPattern, (match) => {
        const hasSpace = match.startsWith('from ');
        return hasSpace ? `from "${shimRelPath}"` : `from"${shimRelPath}"`;
      });
      content = content.replace(sqPattern, (match) => {
        const hasSpace = match.startsWith('from ');
        return hasSpace ? `from '${shimRelPath}'` : `from'${shimRelPath}'`;
      });

      if (content !== before) {
        fileReplacements++;
      }
    }

    if (fileReplacements > 0) {
      await writeFile(filePath, content, 'utf8');
      console.log(`  patched: ${filename} (${fileReplacements} specifier(s))`);
      totalReplacements += fileReplacements;
    }
  }

  console.log(`[ssr-shim-node] Done. ${totalReplacements} file(s) had Node specifier import(s) rewritten.`);
}

main().catch(err => {
  console.error('[ssr-shim-node] FAILED:', err);
  process.exit(1);
});
