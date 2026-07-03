// Golden fixture: a named import of a crypto member that is present for ESM
// linking but NOT implemented (a loud, named stub). Importing links fine; the
// call throws a clear error naming the member -- the bounded-blast-radius
// contract, never a cryptic "undefined is not a function".
import { randomUUID } from "crypto";

export function render(_url) {
  return `<main>${randomUUID()}</main>`;
}
