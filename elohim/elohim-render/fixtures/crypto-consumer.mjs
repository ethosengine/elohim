// Golden fixture: exercises the Node `crypto` builtin shim through BOTH import
// forms the real Angular server bundle uses:
//   - a default import (`import c from "crypto"; c.createHash(...)`)
//   - a named import via the `node:` prefix (`import { createHash } from "node:crypto"`)
// Both must resolve to the injected shim and produce the real sha256 digest.
import cryptoDefault from "crypto";
import { createHash } from "node:crypto";

export function render(url) {
  const viaDefault = cryptoDefault.createHash("sha256").update("hello").digest("hex");
  const viaNamed = createHash("sha256").update("hello").digest("hex");
  return `<main data-url="${url}"><p>${viaDefault}</p><p>${viaNamed}</p></main>`;
}
