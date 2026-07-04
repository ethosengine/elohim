// Golden fixture: the ONE-SOURCE-OF-TRUTH contract. The `node:crypto` module's
// `webcrypto` export (both the named export and the default export's `.webcrypto`
// member) must be the very SAME object as `globalThis.crypto`, so `crypto.subtle`
// and the entropy surface are single-sourced across the module and global surfaces.
import cryptoDefault from "node:crypto";
import { webcrypto } from "node:crypto";

export function render(_url) {
  const namedSame = webcrypto === globalThis.crypto;
  const defaultSame = cryptoDefault.webcrypto === globalThis.crypto;
  // And the real behaviour flows through the module surface too.
  const uuidViaModule =
    typeof webcrypto.randomUUID === "function" &&
    /^[0-9a-f-]{36}$/.test(webcrypto.randomUUID());
  return `namedSame:${namedSame} defaultSame:${defaultSame} uuidViaModule:${uuidViaModule}`;
}
