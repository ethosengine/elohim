/**
 * Static contract: the App's deploy intent carries the full commit SHA.
 *
 * timer-dispatch.mjs `readiness-refusal` accepts an intent only with a
 * full-40-hex commit (it becomes the App baseline the deploy-pending pass
 * re-dispatches). The root Jenkinsfile passed build.env's GIT_COMMIT_HASH —
 * the 8-char image-tag hash — so App #1728's refusal was unusable and
 * orchestrator #1912 recorded no __pendingDeploy__.
 */

import { test } from "node:test";
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const app = readFileSync(new URL("../../Jenkinsfile", import.meta.url), "utf8");
const start = app.indexOf("def fleetWriteReady(");
const body = app.slice(start, app.indexOf("\n}\n", start));

test("fleetWriteReady writes the full HEAD SHA into the deploy intent", () => {
  assert.match(body, /def intentCommit = sh\(script: 'git rev-parse HEAD', returnStdout: true\)\.trim\(\)/);
  assert.match(body, /"DEPLOY_INTENT_COMMIT=\$\{intentCommit\}"/);
  assert.doesNotMatch(body, /DEPLOY_INTENT_COMMIT=\$\{gitCommitHash/);
});
