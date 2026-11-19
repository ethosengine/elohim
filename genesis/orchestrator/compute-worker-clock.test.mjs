// The one compute-worker check that reads the clock against checked-in data (E11): the declared
// grant's window is compared with now, so its verdict changes with time on the same bytes. It runs
// in orchestrator-tree, never in the named half whose green is reused.
import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";

test("the declared Adam runner grant is inside its window, or withdrawn", async (t) => {
  const { runnerGrant } = JSON.parse(
    await readFile(
      new URL("./data/adam-compute-worker.json", import.meta.url),
      "utf8",
    ),
  );
  if (runnerGrant?.enabled !== true) return;
  const { validUntil } = runnerGrant.grant;
  const left = Date.parse(validUntil) - Date.now();
  assert.ok(
    left > 0,
    `adam's runner grant expired at ${validUntil}: renew the window or withdraw it ` +
      "through a reviewed deploy-only commit (genesis/agentic/compute/operations.md)",
  );
  if (left < 48 * 3600 * 1000)
    t.diagnostic(`adam's runner grant expires at ${validUntil}, in under 48 h`);
});
