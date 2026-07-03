// Golden fixture: importing an unshimmed Node builtin must fail LOUD and NAMED
// (the served error module throws at evaluation, naming the builtin) rather than
// panicking with the raw "Relative import path not prefixed" TypeError.
// `readline` is a loader-recognised builtin with NO synthetic surface entry, so
// it exercises the loud whole-module throw path.
import "node:readline";

export function render(_url) {
  return "<main>unreachable</main>";
}
