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

export function render(_url) {
  const parts = [];

  // --- module.createRequire is REAL: returns a require function, loud if called.
  const require = createRequire("file:///bundle/polyfills.server.mjs");
  parts.push("createRequire-fn:" + (typeof require === "function"));
  parts.push("createRequireBare-fn:" + (typeof createRequireBare === "function"));
  try {
    require("fs");
    parts.push("require-call:DID-NOT-THROW");
  } catch (e) {
    parts.push("require-loud:" + /not supported in the SSR runtime/.test(e.message));
  }

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
