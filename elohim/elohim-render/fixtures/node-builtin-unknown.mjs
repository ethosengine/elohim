// Golden fixture: importing an unshimmed Node builtin must fail LOUD and NAMED
// (the served error module throws at evaluation, naming the builtin) rather than
// panicking with the raw "Relative import path not prefixed" TypeError.
import "node:zlib";

export function render(_url) {
  return "<main>unreachable</main>";
}
