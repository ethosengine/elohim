/**
 * Static contract for the saga recording lane.
 *
 * The edge job proved its local stage guards in build #1275, but the upstream
 * orchestrator also dispatched accumulated graph work and Genesis. These tests
 * keep `[edge:validate-only]` isolated at dispatch time and make every future
 * edge stage declare whether it is safe to run in validation-only mode.
 */

import { describe, test } from "node:test";
import { strict as assert } from "node:assert";
import { readFileSync } from "node:fs";

const orchestrator = readFileSync(
  new URL("./Jenkinsfile", import.meta.url),
  "utf8",
);
const edge = readFileSync(
  new URL("../../elohim/holochain/Jenkinsfile", import.meta.url),
  "utf8",
);

function balancedBlock(text, openBrace) {
  let depth = 1;
  let i = openBrace + 1;

  while (i < text.length && depth > 0) {
    if (text.startsWith("//", i)) {
      i = text.indexOf("\n", i + 2);
      if (i === -1) return text.slice(openBrace + 1);
      continue;
    }
    if (text.startsWith("/*", i)) {
      const end = text.indexOf("*/", i + 2);
      assert.notEqual(
        end,
        -1,
        "unterminated block comment while parsing Jenkinsfile",
      );
      i = end + 2;
      continue;
    }
    if (text[i] === "'" || text[i] === '"') {
      const quote = text[i];
      const triple = text.startsWith(quote.repeat(3), i);
      i += triple ? 3 : 1;
      while (i < text.length) {
        if (triple && text.startsWith(quote.repeat(3), i)) {
          i += 3;
          break;
        }
        if (!triple && text[i] === quote) {
          i += 1;
          break;
        }
        if (text[i] === "\\") i += 1;
        i += 1;
      }
      continue;
    }
    if (text[i] === "{") depth += 1;
    if (text[i] === "}") depth -= 1;
    i += 1;
  }

  assert.equal(depth, 0, "unterminated Jenkinsfile block");
  return text.slice(openBrace + 1, i - 1);
}

function staticStages(text) {
  const pipelineStart = text.indexOf("pipeline {");
  assert.notEqual(pipelineStart, -1, "pipeline block missing");
  const pipeline = text.slice(pipelineStart);
  const stages = new Map();
  const matcher = /stage\(\s*(['"])([^'"]+)\1\s*\)\s*\{/g;
  let match;

  while ((match = matcher.exec(pipeline)) !== null) {
    const openBrace = pipelineStart + match.index + match[0].lastIndexOf("{");
    stages.set(match[2], balancedBlock(text, openBrace));
  }
  return stages;
}

function assertEdgeStageContract(text) {
  const stages = staticStages(text);
  const allowed = new Set([
    "Check Trigger",
    "Checkout",
    "Dataplane Validation",
    "Cleanup",
  ]);

  assert.ok(
    stages.size >= 18,
    `expected the full edge stage set, found ${stages.size}`,
  );
  for (const [name, body] of stages) {
    if (allowed.has(name)) continue;
    assert.match(
      body,
      /!isValidateOnly\(\)|!skipBuildStage\(\)/,
      `${name} must skip during validate-only runs or be consciously allowlisted`,
    );
  }

  const forbidden =
    /kubectl|deployHuman|deployDoorway|resolveHappDigest|cleanupOrphanedHumans/;
  for (const name of allowed) {
    assert.ok(stages.has(name), `${name} stage is missing`);
    assert.doesNotMatch(
      stages.get(name),
      forbidden,
      `${name} reaches the deploy plane`,
    );
  }

  return stages;
}

describe("orchestrator validate-only dispatch", () => {
  test("recognizes the tag and collapses the graph to edge only", () => {
    assert.match(orchestrator, /\[edge:validate-only\]/);
    assert.match(orchestrator, /env\.EDGE_VALIDATE_ONLY_FROM_TAG = 'true'/);
    const isolation = orchestrator.match(
      /if \(env\.EDGE_VALIDATE_ONLY_FROM_TAG == 'true'\) \{\s*graphPipelines = \['elohim-edge'\]/,
    );
    assert.ok(isolation, "validate-only graph isolation block is missing");
    const route = orchestrator.indexOf(
      "def graphPipelines = applyBuildGraphRouting",
    );
    const isolate = isolation.index;
    const publish = orchestrator.indexOf(
      "env.PIPELINES_TO_RUN = graphPipelines.join",
      route,
    );
    assert.ok(
      route < isolate && isolate < publish,
      "isolation must be the final routing decision",
    );
  });

  test("sends an explicit safe parameter set to the edge job", () => {
    assert.match(
      orchestrator,
      /validateOnlyDownstream = name == 'elohim-edge'/,
    );
    assert.match(orchestrator, /FORCE_BUILD', value: !validateOnlyDownstream/);
    assert.match(orchestrator, /FORCE_DEPLOY', value: !validateOnlyDownstream/);
    assert.match(orchestrator, /VALIDATE_ONLY', value: validateOnlyDownstream/);
  });
});

describe("edge validate-only stage allowlist", () => {
  const stages = assertEdgeStageContract(edge);

  test("every non-allowlisted static stage carries a validate-only gate", () => {
    assertEdgeStageContract(edge);
  });

  test("negative control: removing one gate is rejected", () => {
    const unsafe = edge.replace(
      "expression { !isValidateOnly() }",
      "expression { true }",
    );
    assert.throws(
      () => assertEdgeStageContract(unsafe),
      /Setup Version must skip/,
    );
  });

  test("Dataplane Validation invokes both read-side measurement scripts", () => {
    const validation = stages.get("Dataplane Validation");
    assert.match(validation, /substrate-seam-smoke\.sh/);
    assert.match(validation, /run-dataplane-validation\.sh/);
  });

  test("Dataplane Validation gates both read-side scripts behind the fleet-quiesce gate", () => {
    const validation = stages.get("Dataplane Validation");
    assert.match(
      validation,
      /fleet-quiesce-gate\.sh/,
      "stage must invoke fleet-quiesce-gate.sh before measuring",
    );

    const gateIdx = validation.indexOf("fleet-quiesce-gate.sh");
    const seamIdx = validation.indexOf("substrate-seam-smoke.sh");
    const dataplaneIdx = validation.indexOf("run-dataplane-validation.sh");
    assert.ok(
      gateIdx !== -1 && seamIdx !== -1 && dataplaneIdx !== -1,
      "all three script references must be present",
    );
    assert.ok(
      gateIdx < seamIdx && gateIdx < dataplaneIdx,
      "fleet-quiesce-gate.sh must be invoked before either read-side measurement script",
    );

    // A skip/guard must tie the gate's result to whether the read-side
    // scripts run at all — otherwise a churning fleet still gets measured.
    assert.match(
      validation,
      /gateRc/,
      "stage must reference a gate-result guard variable (e.g. gateRc)",
    );
    const guardMatch = validation.match(/if\s*\(\s*gateRc\s*==\s*0\s*\)\s*\{/);
    assert.ok(
      guardMatch,
      "stage must guard the read-side scripts on the gate result (gateRc == 0)",
    );
    const guardBody = balancedBlock(
      validation,
      guardMatch.index + guardMatch[0].lastIndexOf("{"),
    );
    assert.match(
      guardBody,
      /substrate-seam-smoke\.sh/,
      "seam smoke must run only inside the gate-passed branch",
    );
    assert.match(
      guardBody,
      /run-dataplane-validation\.sh/,
      "dataplane validation must run only inside the gate-passed branch",
    );
  });
});
