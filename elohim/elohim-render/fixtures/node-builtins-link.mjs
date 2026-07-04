// Golden fixture: the LINK-SAFETY contract for the synthetic Node-builtin shims.
//
// Every one of these is a NAMED import. ESM linking is static and happens BEFORE
// any body runs, so if the shim module for a builtin does not EXPORT the named
// member, THIS MODULE fails to link (SyntaxError: does not provide an export
// named 'X') and render() is never reached. Reaching render() at all therefore
// proves the whole named-import surface links. render() then proves the members
// behave: real ones work, loud ones throw a named error when called.
import { createRequire } from "node:module";
import { createRequire as createRequireBare } from "module";
import { createServer, isIP, isIPv4, isIPv6 } from "node:net";
import { Buffer } from "node:buffer";
import { EventEmitter, on, setMaxListeners } from "node:events";
import { promisify } from "node:util";
import { Agent } from "node:http";
import { Duplex } from "node:stream";
import { TLSSocket, connect } from "node:tls";
import { createSocket } from "node:dgram";
import { networkInterfaces } from "node:os";
// Static crypto import: evaluates the crypto shim, which registers its CJS
// exports on the global that `require("crypto")` reads (single source of truth).
import cryptoDefault from "node:crypto";

export function render(_url) {
  const parts = [];

  // --- module.createRequire is REAL: returns a require function that resolves
  //     Node builtins to CJS exports, and stays loud for non-builtin ids.
  const require = createRequire("file:///bundle/polyfills.server.mjs");
  parts.push("createRequire-fn:" + (typeof require === "function"));
  parts.push("createRequireBare-fn:" + (typeof createRequireBare === "function"));
  // A NON-builtin id (an npm native addon `ws` try/catches) stays loud.
  try {
    require("bufferutil");
    parts.push("require-call:DID-NOT-THROW");
  } catch (e) {
    parts.push("require-loud:" + /not supported in the SSR runtime/.test(e.message));
  }

  // --- require("events") returns the CJS shape: the constructible EventEmitter
  //     itself (Node: `require("events") === EventEmitter`), with siblings on it.
  const evCjs = require("events");
  parts.push("require-events-ctor:" + (typeof evCjs === "function"));
  parts.push("require-events-self:" + (evCjs.EventEmitter === evCjs));
  // `class X extends require("events")` must be a legal class definition (the
  // loud EventEmitter is constructible; super() only throws on instantiation).
  let extendsOk = false;
  try {
    class MyEmitter extends evCjs {}
    extendsOk = typeof MyEmitter === "function";
  } catch (_e) {
    extendsOk = false;
  }
  parts.push("require-events-extends:" + extendsOk);
  // A `node:` prefix resolves the same builtin.
  parts.push("require-node-prefix:" + (require("node:events") === evCjs));
  // A plain-namespace builtin: require("net") is an object carrying its members.
  const netCjs = require("net");
  parts.push(
    "require-net-shape:" +
      (typeof netCjs === "object" && typeof netCjs.createServer === "function"),
  );

  // require("crypto") is served from the crypto shim's global registration and is
  // LITERALLY the same object as the ESM `import crypto from "node:crypto"` -- the
  // crypto member list is not forked between the ESM and CJS surfaces.
  parts.push("require-crypto-same:" + (require("crypto") === cryptoDefault));
  parts.push(
    "require-crypto-createHash:" +
      (typeof require("crypto").createHash === "function"),
  );

  // --- util.promisify is REAL.
  const pfn = promisify(function (cb) {
    cb(null, 42);
  });
  parts.push("promisify-fn:" + (typeof pfn === "function"));

  // --- events.setMaxListeners is a REAL no-op.
  parts.push("setMaxListeners-noop:" + (setMaxListeners(10) === undefined));

  // --- A loud stub throws a NAMED error when called.
  try {
    createServer();
    parts.push("createServer:DID-NOT-THROW");
  } catch (e) {
    parts.push(
      "createServer-loud:" +
        (/net shim/.test(e.message) && /createServer/.test(e.message)),
    );
  }

  // --- All named members link (are defined), even the loud ones.
  const present = [
    createServer,
    isIP,
    isIPv4,
    isIPv6,
    Buffer,
    EventEmitter,
    on,
    setMaxListeners,
    promisify,
    Agent,
    Duplex,
    TLSSocket,
    connect,
    createSocket,
    networkInterfaces,
    createRequire,
    createRequireBare,
  ].every((x) => typeof x !== "undefined");
  parts.push("all-present:" + present);

  return parts.join("|");
}
