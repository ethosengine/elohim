// Golden fixture: `crypto.subtle.digest` is a LOUD, NAMED stub (no subtle op is
// proven on the SSR render path). Calling it throws an error naming the operation,
// so an unproven crypto path fails clearly instead of silently returning garbage.
export function render(_url) {
  return `<main>${globalThis.crypto.subtle.digest("SHA-256", new Uint8Array(1))}</main>`;
}
