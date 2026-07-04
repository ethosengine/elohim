// Golden fixture: a named import of a crypto member that is present for ESM
// linking but NOT implemented (a loud, named stub). Importing links fine; the
// call throws a clear error naming the member -- the bounded-blast-radius
// contract, never a cryptic "undefined is not a function".
//
// `randomBytes` is the exemplar loud stub: the Node Buffer-returning entropy API
// is off the SSR render path (the render path uses the WebCrypto getRandomValues
// global), so it stays a named throw. (randomUUID, by contrast, is now REAL --
// backed by the WebCrypto global's OS entropy.)
import { randomBytes } from "crypto";

export function render(_url) {
  const b = randomBytes(4);
  return `<main>${b.length}</main>`;
}
