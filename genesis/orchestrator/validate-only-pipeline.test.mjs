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
    // The tag is the edge-scoped verify class; the booleans derive from the
    // run class (run-class.test.mjs pins the class contract itself).
    assert.match(
      orchestrator,
      /if \(name == 'elohim-edge' && env\.EDGE_VALIDATE_ONLY_FROM_TAG == 'true'\) runClass = 'verify'/,
    );
    assert.match(
      orchestrator,
      /validateOnlyDownstream = name == 'elohim-edge' && belowDeploy/,
    );
    assert.match(orchestrator, /FORCE_BUILD', value: !belowDeploy/);
    assert.match(orchestrator, /FORCE_DEPLOY', value: !belowDeploy/);
    assert.match(orchestrator, /VALIDATE_ONLY', value: validateOnlyDownstream/);
  });
});

describe("orchestrator graph-derived validate-only (a2o-only change)", () => {
  const edgeManifest = JSON.parse(
    readFileSync(
      new URL("../../elohim/holochain/build-manifest.json", import.meta.url),
      "utf8",
    ),
  );

  test("dataplane-validation deploys nothing, so it alone can never roll the fleet", () => {
    const step = edgeManifest.steps["dataplane-validation"];
    assert.ok(step, "edge manifest lost its dataplane-validation step");
    assert.deepEqual(step.depends ?? [], []);
    for (const [name, other] of Object.entries(edgeManifest.steps)) {
      assert.ok(
        !(other.depends ?? []).includes("dataplane-validation"),
        `${name} depends on dataplane-validation — a validation-only change would cascade into it`,
      );
    }
  });

  test("an edge selection whose stale steps are all below deploy dispatches validate-only", () => {
    // Class rule: the edge's run class is the max steps[].class of its stale
    // steps; below deploy (verify/measure/profile) it dispatches validate-only.
    const rule = orchestrator.match(
      /if \(edgeStaleSteps && runClassBelowDeploy\(runClasses\['elohim-edge'\] \?: 'build'\) && !edgeForced\) \{\s*echo "🧪 elohim-edge: stale steps are all class/,
    );
    assert.ok(rule, "graph-derived validate-only rule is missing");
    assert.match(orchestrator, /def forcedPipelines = \(env\.FORCE_BUILD_PIPELINES \?: ''\)\.split\(','\)\.findAll \{ it \}/);
    assert.match(orchestrator, /boolean edgeForced = forcedPipelines\.contains\('elohim-edge'\)/);
    assert.match(
      orchestrator,
      /validateOnlyDownstream = name == 'elohim-edge' && belowDeploy/,
    );
  });

  test("a validate-only edge never pulls Genesis in", () => {
    assert.match(
      orchestrator,
      /triggersGenesis && !runClassBelowDeploy\(runClasses\[it\] \?: 'build'\)/,
    );
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

// ── Lane C3 (one-head-delivered sprint): Dataplane Validation leaves the
// delivery path. Edge #1486 spent 45 min in the stage on its deploy run and
// ended `FLEET-CHURNING … DID NOT MEASURE` while app and genesis waited. A
// deploy-bearing orchestrator run now skips the stage and says so; the
// orchestrator fires the strict validate-only edge run as a fire-and-forget
// sibling. Source-asserted: running the Groovy needs a controller.

function topLevelDef(text, name) {
  const at = text.search(new RegExp(`^def\\s+${name}\\s*\\(`, "m"));
  assert.notEqual(at, -1, `top-level def ${name}() is missing`);
  return balancedBlock(text, text.indexOf("{", at));
}

function stageBlock(text, name) {
  const at = text.indexOf(`stage('${name}') {`);
  assert.notEqual(at, -1, `stage '${name}' is missing`);
  return balancedBlock(text, text.indexOf("{", at));
}

function whenOf(stage) {
  const at = stage.indexOf("when {");
  assert.notEqual(at, -1, "stage has no when block");
  return balancedBlock(stage, stage.indexOf("{", at));
}

describe("Dataplane Validation runs beside app and genesis (Lane C3)", () => {
  const dvWhen = whenOf(stageBlock(edge, "Dataplane Validation"));

  test("edge: the stage's when names RUN_CLASS and skips a deploy-bearing class", () => {
    assert.match(
      dvWhen,
      /expression \{ isValidateOnly\(\) \|\| !owesValidationToSibling\(params\.RUN_CLASS\) \}/,
    );
    // Evaluated LAST inside allOf (which short-circuits), so the debt is
    // recorded only when every other condition would have run the stage here.
    const allOf = balancedBlock(dvWhen, dvWhen.indexOf("{", dvWhen.indexOf("allOf")));
    const lastLine = allOf.trim().split("\n").pop().trim();
    assert.match(lastLine, /owesValidationToSibling\(params\.RUN_CLASS\)/);
    assert.ok(
      allOf.indexOf("DEPLOY_REFUSED") < allOf.indexOf("owesValidationToSibling"),
      "a refused deploy must short-circuit before the class rule records a debt",
    );
  });

  test("edge: build|deploy owe the measurement; empty (manual) and sub-deploy classes measure here", () => {
    const rule = topLevelDef(edge, "owesValidationToSibling");
    assert.match(rule, /!\(runClass in \['build', 'deploy'\]\)\) return false/);
    assert.match(rule, /env\.DATAPLANE_VALIDATION = 'owed-by-sibling'/);
    // Manual/legacy runs keep today's behaviour: the declared default is empty,
    // and computeValidateOnly still coalesces it to build.
    assert.match(edge, /string\(\s*name: 'RUN_CLASS',\s*defaultValue: ''/);
    assert.match(topLevelDef(edge, "computeValidateOnly"), /params\.RUN_CLASS \?: 'build'/);
  });

  test("edge: the deploy run's description says the measurement is owed, after the DEPLOY-REFUSED prefix logic", () => {
    const desc = topLevelDef(edge, "edgeRunDescription");
    assert.match(desc, /Dataplane Validation: owed by sibling run/);
    assert.match(desc, /env\.DATAPLANE_VALIDATION == 'owed-by-sibling'/);
    assert.match(desc, /env\.DEPLOY_REFUSED == 'true' \? "DEPLOY-REFUSED: \$\{env\.DEPLOY_REFUSED_REASON\} \| \$\{summary\}" :/);
  });

  test("edge: only an orchestrator (UpstreamCause) dispatch owes the measurement; a manual run measures inline", () => {
    const rule = topLevelDef(edge, "owesValidationToSibling");
    const cause = rule.indexOf("currentBuild.getBuildCauses('hudson.model.Cause$UpstreamCause').isEmpty()) return false");
    assert.ok(cause !== -1, "a run with no UpstreamCause must return false");
    assert.ok(cause < rule.indexOf("env.DATAPLANE_VALIDATION = 'owed-by-sibling'"), "the cause check precedes the debt");
  });

  test("edge: a strict no-measure (fleet never settled) is NOT_BUILT, never FAILURE; reds and zero scenarios stay red", () => {
    const body = topLevelDef(edge, "runDataplaneValidation");
    // The script's status is read, not thrown: exit 3 is split by its banner.
    assert.match(body, /rc = sh\(returnStatus: true, script: "#!\/bin\/bash\\nset -o pipefail\\nbash '\$\{env\.WORKSPACE\}\/scripts\/ci\/run-dataplane-validation\.sh' 2>&1 \| tee '\$\{log\}'"\)/);
    assert.match(body, /noMeasure = \(rc == 3\) \? dataplaneNoMeasureReason\(readFile\(log\)\) : null/);
    assert.match(body, /if \(rc != 0 && !noMeasure\) \{ error\("run-dataplane-validation\.sh exited \$\{rc\}"\) \}/);
    // Strict reds still end FAILURE inside the validate-only catchError.
    assert.match(body, /if \(strict\) \{\s*catchError\(buildResult: 'FAILURE', stageResult: 'FAILURE'\) \{ body\(\) \}/);
    // NOT_BUILT is applied after the served-shell gate, so it never masks its FAILURE.
    const shellGate = body.indexOf("Served shell does not boot through");
    const notBuilt = body.indexOf("currentBuild.result = 'NOT_BUILT'");
    assert.ok(shellGate !== -1 && notBuilt > shellGate, "NOT_BUILT comes after the served-shell gate");
    assert.match(body, /if \(strict\) \{ currentBuild\.result = 'NOT_BUILT' \} else \{ unstable\(/);
    assert.match(body, /env\.DATAPLANE_NO_MEASURE = noMeasure\s*\n\s*currentBuild\.description = edgeRunDescription\(\)/);
    assert.match(topLevelDef(edge, "edgeRunDescription"), /"DATAPLANE: DID NOT MEASURE \(\$\{env\.DATAPLANE_NO_MEASURE\}\) \| \$\{summary\}"/);
  });

  test("edge: the no-measure banner the Jenkinsfile reads is printed only on the quiesce path", () => {
    const banner = "=== Dataplane Validation: DID NOT MEASURE ===";
    const reason = topLevelDef(edge, "dataplaneNoMeasureReason");
    assert.ok(reason.includes(banner), "the Jenkinsfile keys the no-measure on the script's banner");
    assert.match(reason, /startsWith\('Fleet-quiesce gate exited'\)/);
    assert.equal(runDataplaneValidation.split(banner).length, 2, "the script prints the banner exactly once");
    const bannerAt = runDataplaneValidation.indexOf(banner);
    assert.ok(bannerAt < runDataplaneValidation.indexOf("cucumber-js"), "banner is on the pre-cucumber quiesce path");
    assert.ok(runDataplaneValidation.includes('echo "Fleet-quiesce gate exited ${QUIESCE_EXIT}'), "the reason line the Jenkinsfile quotes");
    // The zero-scenario guard also exits 3 but never prints the banner: it stays a red.
    const zero = runDataplaneValidation.indexOf("0 dataplane scenarios ran");
    assert.ok(zero > bannerAt && !runDataplaneValidation.slice(zero).includes(banner));
  });

  test("orchestrator: exactly one VALIDATE_ONLY=true dispatch, fire-and-forget", () => {
    const hits = orchestrator.match(/booleanParam\(name: 'VALIDATE_ONLY', value: true\)/g) ?? [];
    assert.equal(hits.length, 1, "exactly one sibling dispatch carries VALIDATE_ONLY=true");
    const sibling = topLevelDef(orchestrator, "dispatchValidationSibling");
    assert.match(sibling, /booleanParam\(name: 'VALIDATE_ONLY', value: true\)/);
    const call = sibling.slice(sibling.indexOf("build("));
    assert.match(call, /wait: false/);
    assert.match(call, /propagate: false/);
    assert.doesNotMatch(call, /waitForStart|waitForBuild/);
    assert.match(sibling, /stringParam\(name: 'RUN_CLASS', value: 'verify'\)/);
    assert.match(sibling, /booleanParam\(name: 'FORCE_BUILD', value: false\)/);
    assert.match(sibling, /booleanParam\(name: 'FORCE_DEPLOY', value: false\)/);
    assert.match(sibling, /booleanParam\(name: 'DEPLOY_ONLY', value: false\)/);
    // A measurement, not a delivery: never recorded as a result or a baseline.
    assert.doesNotMatch(sibling, /results\[[^\]]+\]\s*=|pipelineBaselines|recordPipelineResult/);
  });

  test("orchestrator: the sibling is fired after each level's verdicts, not per branch", () => {
    const doom = orchestrator.indexOf("failedPipelines.addAll(doomDependents(runnable, pipelines, results, pipelineBaselines))");
    const fire = orchestrator.indexOf("dispatchValidationSibling(runnable, results)");
    const checkpoint = orchestrator.indexOf("archivePipelineBaselines('execute')");
    assert.ok(doom !== -1 && fire > doom && fire < checkpoint, "sibling fires after doomDependents, before the level checkpoint");
    assert.equal(orchestrator.split("dispatchValidationSibling(runnable, results)").length, 2, "one call site");
  });

  test("orchestrator: never on a refused, validate-only, skipped or unowed edge run", () => {
    const sibling = topLevelDef(orchestrator, "dispatchValidationSibling");
    const guards = sibling.slice(0, sibling.indexOf("build("));
    for (const guard of [
      /!ran\.contains\('elohim-edge'\)/,
      /!edge\?\.success/,
      /edge\.skipped/,
      /edge\.dispatched/,
      /edge\.deployRefused/,
      /!edge\.validationOwed/,
      /env\.EDGE_VALIDATE_ONLY_FROM_TAG == 'true'/,
      /runClassBelowDeploy\(runClassFor\('elohim-edge', env\.PIPELINE_RUN_CLASSES\)\)/,
    ]) {
      assert.match(guards, guard);
    }
    // validationOwed is read from the edge run's own description — one contract home.
    assert.match(
      topLevelDef(orchestrator, "dispatchResult"),
      /validationOwed: \(result\.description \?: ''\)\.toString\(\)\.contains\('Dataplane Validation: owed by sibling run'\)/,
    );
  });
});
