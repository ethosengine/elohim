/**
 * Static contract: the changed-path list never rides in an env string.
 *
 * Orchestrator #1904/#1905: `env.CHANGED_PATHS_PASSTHROUGH` held 2185 paths
 * (138,512 bytes). Linux caps one env string at 128KiB (MAX_ARG_STRLEN), so
 * every `sh` after Determine Build Plan failed to exec — durable-task reported
 * `process apparently never started` (exit -2) in Brit Plan, read at the time
 * as a launch flake. A build parameter is an env var in the downstream job, so
 * the CHANGED_PATHS parameter is capped the same way.
 */

import { test } from "node:test";
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const orchestrator = readFileSync(new URL("./Jenkinsfile", import.meta.url), "utf8");
const code = orchestrator
  .split("\n")
  .filter((line) => !line.trim().startsWith("//"))
  .join("\n");

test("no env var is assigned a joined changed-file list", () => {
  assert.doesNotMatch(code, /env\.[A-Z_]+\s*=\s*changedFiles\.join/);
  assert.doesNotMatch(code, /CHANGED_PATHS_PASSTHROUGH/);
});

test("Determine Build Plan stashes the list and Execute Builds loads it", () => {
  assert.match(code, /stashChangedPaths\(changedFiles\)/);
  assert.match(code, /def changedFiles = loadChangedPaths\(\)/);
});

test("the downstream CHANGED_PATHS parameter goes through the byte cap", () => {
  const params = [...code.matchAll(/stringParam\(name: 'CHANGED_PATHS', value: ([^)]+\))\)/g)];
  assert.equal(params.length, 1, "exactly one CHANGED_PATHS parameter site");
  assert.equal(params[0][1], "changedPathsParam(changedFiles)");
  const cap = Number(code.match(/CHANGED_PATHS_PARAM_MAX = (\d+) \* 1024/)?.[1]) * 1024;
  assert.ok(cap > 0 && cap < 128 * 1024, `cap ${cap} must stay under MAX_ARG_STRLEN`);
});
