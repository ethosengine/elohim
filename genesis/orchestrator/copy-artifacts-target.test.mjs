/**
 * Static contract: a copyArtifacts target is never pre-created by `sh`.
 *
 * `sh` runs as root in the builder container; copyArtifacts writes as the
 * agent user. A root-made target directory refuses the copy ("Failed to copy
 * file"). Orchestrator #1910 lost the App's readiness refusal that way — the
 * refused deploy read as delivered, no __pendingDeploy__ was recorded, and
 * the timer pass (#1911) had nothing to re-dispatch — and stageAnnotations
 * lost genesis's conductor-readiness the same way.
 */

import { test } from "node:test";
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const code = readFileSync(new URL("./Jenkinsfile", import.meta.url), "utf8")
  .split("\n")
  .filter((line) => !line.trim().startsWith("//"))
  .join("\n");

test("no copyArtifacts target directory is created by sh mkdir", () => {
  const targets = [...code.matchAll(/target:\s*([A-Za-z_]\w*)/g)].map((m) => m[1]);
  assert.ok(targets.length > 0, "found copyArtifacts targets");
  for (const t of new Set(targets)) {
    assert.doesNotMatch(code, new RegExp(`sh\\s+"mkdir -p '\\$\\{${t}\\}'"`), `${t} is pre-created by sh`);
  }
});
