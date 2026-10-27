// Exercise the ACTUAL hook function: a tree without an attested submodule's
// checkout must be named before any gate runs, and a present one never is.
import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
const HOOK = readFileSync(join(ROOT, ".husky/pre-push.bash"), "utf8");
const start = HOOK.indexOf("attested_submodules_missing() {\n");
const end = HOOK.indexOf("\n}\n", start);
assert.ok(start >= 0 && end > start, "hook has attested_submodules_missing() function");
const MISSING = HOOK.slice(start, end + "\n}\n".length);

function missing(statusLines) {
  const r = spawnSync("bash", ["-c", [
    "set -o pipefail",
    `git() { printf '%s\\n' ${statusLines.map((l) => `'${l}'`).join(" ")}; }`,
    MISSING,
    "attested_submodules_missing",
  ].join("\n")], { cwd: ROOT, encoding: "utf8", timeout: 10000 });
  assert.equal(r.status, 0, r.stdout + r.stderr);
  return r.stdout.split("\n").filter(Boolean);
}

test("names_only_uninitialized_attested_submodules", () => {
  assert.deepEqual(missing([
    "-76ba95d8a0c774d566d583a42629ac215abc4ba0 elohim/brit",
    " 9db775b3eca070ae89509251b0563b9e4d6518b1 elohim/rakia (heads/main)",
    "-1111111111111111111111111111111111111111 sophia",
  ]), ["elohim/brit", "sophia"]);
});

test("a_modified_or_conflicted_checkout_is_present", () => {
  assert.deepEqual(missing([
    "+76ba95d8a0c774d566d583a42629ac215abc4ba0 elohim/brit (heads/x)",
    "U9db775b3eca070ae89509251b0563b9e4d6518b1 elohim/rakia",
    " 1111111111111111111111111111111111111111 sophia (heads/main)",
  ]), []);
});
