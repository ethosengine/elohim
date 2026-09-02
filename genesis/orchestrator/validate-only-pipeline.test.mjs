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
const runDataplaneValidation = readFileSync(
  new URL("../../scripts/ci/run-dataplane-validation.sh", import.meta.url),
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
    stages.set(match[2], inlineTopLevelHelpers(text, balancedBlock(text, openBrace)));
  }
  return stages;
}

// A stage body may be a single call to a top-level `def name()` — the edge
// Jenkinsfile's pipeline{} block sits at the JVM 64KB CPS ceiling, so stage
// bodies are hoisted into helpers (runMeshQuiesceMeasure, runDataplaneValidation).
// The contract these tests pin is the stage's BEHAVIOUR, so a hoisted body is
// inlined (one level) before the assertions read it; a helper that does not
// exist is left as the bare call so the assertion fails loudly rather than
// silently passing on an empty body.
function inlineTopLevelHelpers(text, body) {
  const calls = body.matchAll(/^\s*([A-Za-z_]\w*)\(\)\s*$/gm);
  let out = body;
  for (const call of calls) {
    const name = call[1];
    const def = text.search(new RegExp(`^def\\s+${name}\\s*\\(\\s*\\)\\s*\\{`, "m"));
    if (def === -1) continue;
    const openBrace = text.indexOf("{", def);
    out += `\n// inlined top-level def ${name}()\n` + balancedBlock(text, openBrace);
  }
  return out;
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

  test("dataplane measurement rides the fleet-quiesce gate (in-script, CPS-safe)", () => {
    // The gate lives INSIDE run-dataplane-validation.sh, not the Jenkinsfile
    // stage — inflating the stage's CPS method breached the 64KB JVM limit on
    // edge #1282 (MethodTooLargeException at Jenkinsfile parse; the build died
    // stageless). The stage's only quiesce responsibility is the deadline env.

    // 1. The runner invokes the gate BEFORE the cucumber suite...
    const gateIdx = runDataplaneValidation.indexOf("fleet-quiesce-gate.sh");
    const cucumberIdx = runDataplaneValidation.indexOf("cucumber-js");
    const reportIdx = runDataplaneValidation.indexOf("build-sprint-report.ts");
    assert.ok(
      gateIdx !== -1,
      "run-dataplane-validation.sh must invoke fleet-quiesce-gate.sh",
    );
    assert.ok(
      cucumberIdx !== -1 && gateIdx < cucumberIdx,
      "fleet-quiesce-gate.sh must be invoked before the cucumber suite",
    );
    assert.ok(
      reportIdx !== -1 && gateIdx < reportIdx,
      "fleet-quiesce-gate.sh must be invoked before the sprint report is built",
    );

    // 2. ...and a gate failure exits without measuring (exit 3 = the
    // did-not-measure idiom shared with the zero-scenario guard), so no
    // sprint-report can be generated from a churn window.
    const guard = runDataplaneValidation.match(
      /if\s*\[\s*"\$\{QUIESCE_EXIT\}"\s+-ne\s+0\s*\]/,
    );
    assert.ok(
      guard,
      "runner must guard on the gate's exit code (QUIESCE_EXIT)",
    );
    const afterGuard = runDataplaneValidation.slice(guard.index);
    const exitMatch = afterGuard.match(/^\s*exit 3\s*$/m);
    assert.ok(
      exitMatch && afterGuard.indexOf("exit 3") < afterGuard.indexOf("cucumber-js"),
      "gate failure must exit 3 before any cucumber invocation",
    );

    // 3. The Jenkinsfile stage supplies the deadline, and it is the SAME long
    // bounded wait on both paths (2026-08-07). The old isValidateOnly()
    // ternary gave the post-deploy path 900s — shorter than the ~20min
    // conductor restart churn that path itself causes — so the run that most
    // needs a measurement was the one that systematically could not take one.
    // Guard against a regression to any per-path/short deadline.
    const validation = stages.get("Dataplane Validation");
    assert.match(
      validation,
      /QUIESCE_DEADLINE_SECS=2700/,
      "stage must set QUIESCE_DEADLINE_SECS=2700",
    );
    assert.ok(
      !/QUIESCE_DEADLINE_SECS=[^"]*isValidateOnly/.test(validation),
      "QUIESCE_DEADLINE_SECS must not be per-path — post-deploy churn (~20min) needs the full bound too",
    );
    assert.ok(
      !/QUIESCE_DEADLINE_SECS=[^"]*900/.test(validation),
      "900s expires inside the post-deploy restart-churn window (edge #1319)",
    );
  });
});
