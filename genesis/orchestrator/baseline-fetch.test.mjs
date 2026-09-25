/**
 * Static contract: a baseline outside the shallow clone is fetched, never skipped.
 *
 * The orchestrator job clones at --depth=200. Orchestrator #1906 received a
 * 242-commit push; the stored baseline 82dcb5c8 sat below the clone, and
 * analyzeChangeset fell back to `git diff HEAD~1` — 7 files instead of the
 * batch, so the conductor pin move and every other pipeline's changes were
 * never dispatched. The baseline is fetched by SHA (then the history is
 * unshallowed) before any fallback, and the fallback marks the run UNSTABLE.
 */

import { test } from "node:test";
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const orchestrator = readFileSync(new URL("./Jenkinsfile", import.meta.url), "utf8");
const start = orchestrator.indexOf("def analyzeChangeset(");
const body = orchestrator.slice(start, orchestrator.indexOf("\n}\n", start));

test("a missing baseline is fetched by SHA, then unshallowed, before HEAD~1", () => {
  const fetchSha = body.indexOf("git fetch --no-tags --quiet --depth=1 origin ${baseCommit}");
  const unshallow = body.indexOf("--unshallow");
  const fallback = body.indexOf("git diff --name-only HEAD~1");
  assert.ok(fetchSha > 0, "fetches the baseline commit by SHA");
  assert.ok(unshallow > fetchSha, "unshallows only after the SHA fetch misses");
  assert.ok(fallback > unshallow, "HEAD~1 is reached only after both fetches");
});

test("falling back past a named baseline is never silent", () => {
  assert.match(body, /unstable\("Baseline \$\{baseCommit\.take\(8\)\} unreachable/);
});
