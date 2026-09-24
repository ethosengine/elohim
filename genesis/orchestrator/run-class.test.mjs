/**
 * Run classes as declared data (native-delivery sprint, Lane B).
 *
 * A step declares `class: build | deploy | verify | measure | profile` in its
 * build-manifest (rakia `steps[].class`, default build). A dispatch's class is
 * the HIGHEST class among its stale steps, and a class executes only the steps
 * at or below it. `[run:verify|measure|profile]` generalizes
 * `[edge:validate-only]`; `[build:*]` still forces `build`.
 *
 * Extends validate-only-pipeline.test.mjs's pattern: the pure policy lives in
 * pipeline-registry.mjs and is exercised directly; the Groovy mirrors
 * (build-graph.groovy, the orchestrator and edge Jenkinsfiles) are pinned by
 * static contract, since this host has no Groovy runtime.
 */

import { describe, test } from "node:test";
import { strict as assert } from "node:assert";
import { existsSync, readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import {
  RUN_CLASSES,
  stepClass,
  runClassRank,
  deriveRunClass,
  classAllows,
  parseRunClassTag,
  planRunClasses,
  loadPipelineRegistry,
} from "./pipeline-registry.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(HERE, "../..");
const read = (rel) => readFileSync(resolve(ROOT, rel), "utf8");

const orchestrator = read("genesis/orchestrator/Jenkinsfile");
const edge = read("elohim/holochain/Jenkinsfile");
const buildGraph = read("genesis/orchestrator/build-graph.groovy");
const mirrorSchema = JSON.parse(read("genesis/orchestrator/manifest.schema.json"));
// The rakia submodule may be uninitialised (a fresh clone, a scratch worktree).
const RAKIA_SCHEMA = resolve(ROOT, "elohim/rakia/schemas/v1/build-manifest.schema.json");
const rakiaSchema = existsSync(RAKIA_SCHEMA) ? JSON.parse(readFileSync(RAKIA_SCHEMA, "utf8")) : null;

function topLevelDef(text, name) {
  const start = text.search(new RegExp(`^def\\s+${name}\\s*\\(`, "m"));
  assert.notEqual(start, -1, `top-level def ${name}() is missing`);
  const next = text.slice(start + 4).search(/^(def\s|pipeline\s*\{|@NonCPS)/m);
  return next === -1 ? text.slice(start) : text.slice(start, start + 4 + next);
}

describe("class order and derivation", () => {
  test("the order is build > deploy > verify > measure > profile", () => {
    assert.deepEqual(RUN_CLASSES, ["build", "deploy", "verify", "measure", "profile"]);
    const ranks = RUN_CLASSES.map(runClassRank);
    assert.deepEqual([...ranks].sort((a, b) => b - a), ranks, "ranks descend with the list");
  });

  test("an absent or unknown class reads as build", () => {
    assert.equal(stepClass({}), "build");
    assert.equal(stepClass({ class: undefined }), "build");
    assert.equal(stepClass({ class: "nonsense" }), "build");
    assert.equal(stepClass({ class: "verify" }), "verify");
  });

  const table = [
    [["verify"], "verify"],
    [["verify", "measure"], "verify"],
    [["measure", "profile"], "measure"],
    [["profile"], "profile"],
    [["verify", "deploy"], "deploy"],
    [["profile", "build", "verify"], "build"],
    [[undefined, "verify"], "build"],
  ];
  for (const [classes, expected] of table) {
    test(`max of [${classes.map(String).join(", ")}] is ${expected}`, () => {
      assert.equal(deriveRunClass(classes), expected);
    });
  }

  test("no stale steps derive no class", () => {
    assert.equal(deriveRunClass([]), null);
  });

  test("a class executes the steps at or below it", () => {
    assert.equal(classAllows("build", "profile"), true);
    assert.equal(classAllows("build", "build"), true);
    assert.equal(classAllows("verify", "verify"), true);
    assert.equal(classAllows("verify", "measure"), true);
    assert.equal(classAllows("verify", "deploy"), false);
    assert.equal(classAllows("measure", "verify"), false);
    assert.equal(classAllows("profile", "measure"), false);
  });
});

describe("[run:*] commit tag", () => {
  test("parses the three sub-deploy classes, case-insensitively", () => {
    assert.equal(parseRunClassTag("fix: steps [run:verify]"), "verify");
    assert.equal(parseRunClassTag("[RUN:Measure] quiesce"), "measure");
    assert.equal(parseRunClassTag("x\n\n[run:profile]\n"), "profile");
  });

  test("build and deploy are not run tags — [build:*] and [deploy-only] own those", () => {
    assert.equal(parseRunClassTag("[run:build]"), null);
    assert.equal(parseRunClassTag("[run:deploy]"), null);
    assert.equal(parseRunClassTag("[build:edge]"), null);
    assert.equal(parseRunClassTag(""), null);
    assert.equal(parseRunClassTag(undefined), null);
  });

  test("the orchestrator parses it in a @NonCPS def with the same vocabulary", () => {
    const def = orchestrator.match(/@NonCPS\s+def parseRunClassTag\(String [a-zA-Z]+\) \{[\s\S]*?\n\}/);
    assert.ok(def, "@NonCPS def parseRunClassTag(String) is missing");
    assert.match(def[0], /\\\[run:\(verify\|measure\|profile\)\\\]/);
    assert.match(orchestrator, /env\.RUN_CLASS_FROM_TAG = parseRunClassTag\(commitMsg\)/);
  });
});

describe("planRunClasses — the dispatch set and each pipeline's class", () => {
  const stepClasses = {
    "elohim-edge": {
      "cargo-build-storage": "build",
      "deploy-manifests": "deploy",
      "dataplane-validation": "verify",
    },
    elohim: { "build-angular": "build", "e2e-alpha": "verify" },
    "elohim-genesis": { seed: "build" },
    "elohim-profiler": { "flamegraph": "profile" },
  };

  test("the graph's class is the max of each pipeline's stale steps", () => {
    const plan = planRunClasses({
      staleSteps: {
        "elohim-edge": ["dataplane-validation"],
        elohim: ["build-angular", "e2e-alpha"],
      },
      stepClasses,
    });
    assert.deepEqual(plan, { "elohim-edge": "verify", elohim: "build" });
  });

  test("a step the registry does not know reads as build", () => {
    const plan = planRunClasses({
      staleSteps: { "elohim-edge": ["dataplane-validation", "brand-new-step"] },
      stepClasses,
    });
    assert.deepEqual(plan, { "elohim-edge": "build" });
  });

  test("[build:*] still forces build, even over a verify-only change", () => {
    const plan = planRunClasses({
      staleSteps: { "elohim-edge": ["dataplane-validation"] },
      stepClasses,
      forced: ["elohim-edge", "elohim-genesis"],
    });
    assert.deepEqual(plan, { "elohim-edge": "build", "elohim-genesis": "build" });
  });

  test("[run:verify] selects every pipeline declaring a step at or below verify, at verify", () => {
    const plan = planRunClasses({
      staleSteps: { "elohim-genesis": ["seed"] },
      stepClasses,
      tag: "verify",
    });
    assert.deepEqual(plan, { "elohim-edge": "verify", elohim: "verify", "elohim-profiler": "verify" });
  });

  test("[run:measure] on an empty change selects only pipelines declaring a measure-or-lower step", () => {
    assert.deepEqual(planRunClasses({ staleSteps: {}, stepClasses, tag: "measure" }), {
      "elohim-profiler": "measure",
    });
  });

  test("[run:*] keeps a [build:*]-forced pipeline at build", () => {
    const plan = planRunClasses({ staleSteps: {}, stepClasses, tag: "verify", forced: ["elohim"] });
    assert.equal(plan.elohim, "build");
    assert.equal(plan["elohim-edge"], "verify");
  });
});

describe("declared data — the manifests and both schemas", () => {
  const registry = loadPipelineRegistry(ROOT);

  test("every registered pipeline carries a class for every step, defaulting to build", () => {
    assert.ok(registry.size > 5);
    for (const entry of registry.values()) {
      assert.ok(entry.stepClasses && typeof entry.stepClasses === "object", entry.pipeline);
      for (const [step, cls] of Object.entries(entry.stepClasses)) {
        assert.ok(RUN_CLASSES.includes(cls), `${entry.pipeline}:${step} has class ${cls}`);
      }
    }
  });

  test("the orchestrator schema mirror accepts steps[].class with the same enum and default", () => {
    const cls = mirrorSchema.$defs.step.properties.class;
    assert.ok(cls, "manifest.schema.json step lacks class");
    assert.deepEqual(cls.enum, RUN_CLASSES);
    assert.equal(cls.default, "build");
    assert.ok(!mirrorSchema.$defs.step.required.includes("class"), "class must stay optional");
  });

  test("when the pinned rakia schema declares steps[].class, it is the same vocabulary", () => {
    const cls = rakiaSchema?.$defs.step.properties.class;
    if (!cls) return; // submodule absent, or the pin predates B1 — consumers default to build meanwhile
    assert.deepEqual(cls.enum, RUN_CLASSES);
    assert.equal(cls.default, "build");
  });
});

describe("Groovy mirrors carry the class", () => {
  test("build-graph.groovy reads each step's class, absent → build", () => {
    assert.match(buildGraph, /runClass: \(stepDef\['class'\] \?: 'build'\)\.toString\(\)/);
    assert.match(buildGraph, /def RUN_CLASS_ORDER = \['build', 'deploy', 'verify', 'measure', 'profile'\]/);
  });

  test("build-graph.groovy returns the derived class per pipeline and the tag selections", () => {
    assert.match(buildGraph, /runClasses: deriveRunClasses\(pipelineSteps, graph\)/);
    assert.match(buildGraph, /runClassSelections: runClassSelections\(graph\)/);
    assert.match(buildGraph, /stepClasses: stepClassesOf\(manifest\.steps\)/);
  });

  test("the orchestrator's fallback metadata path also carries stepClasses", () => {
    assert.match(topLevelDef(orchestrator, "getPipelineMetadata"), /stepClasses:/);
  });
});

describe("orchestrator dispatch sends RUN_CLASS to every downstream", () => {
  const trigger = topLevelDef(orchestrator, "triggerPipeline");

  test("RUN_CLASS is a stringParam added unconditionally to the monorepo parameter list", () => {
    const listStart = trigger.indexOf("def buildParams = [");
    assert.notEqual(listStart, -1);
    const add = trigger.indexOf("buildParams.add(stringParam(name: 'RUN_CLASS', value: runClass))");
    assert.ok(add > listStart, "RUN_CLASS must be added to buildParams");
    const between = trigger.slice(listStart, add);
    const opens = (between.match(/\{/g) || []).length;
    const closes = (between.match(/\}/g) || []).length;
    assert.equal(opens, closes, "RUN_CLASS must not sit inside a per-pipeline conditional");
  });

  test("the force flags derive from the class instead of an edge-only boolean", () => {
    assert.match(trigger, /String runClass = runClassFor\(name, env\.PIPELINE_RUN_CLASSES\)/);
    assert.match(trigger, /boolean belowDeploy = runClassBelowDeploy\(runClass\)/);
    assert.match(trigger, /FORCE_BUILD', value: !belowDeploy/);
    assert.match(trigger, /FORCE_DEPLOY', value: !belowDeploy/);
  });

  test("an absent entry reads as build", () => {
    const def = orchestrator.match(/@NonCPS\s+def runClassFor\(String name, String encoded\) \{[\s\S]*?\n\}/);
    assert.ok(def, "@NonCPS def runClassFor is missing");
    assert.match(def[0], /: 'build'/);
  });

  test("routing records a class for every dispatched pipeline and echoes it", () => {
    const route = topLevelDef(orchestrator, "applyBuildGraphRouting");
    assert.match(route, /env\.PIPELINE_RUN_CLASSES = /);
    assert.match(route, /RUN_CLASS=/);
  });

  test("a pipeline below deploy never pulls Genesis in", () => {
    const route = topLevelDef(orchestrator, "applyBuildGraphRouting");
    assert.match(route, /getPipelineMetadata\(it\)\.triggersGenesis && !runClassBelowDeploy\(runClasses\[it\] \?: 'build'\)/);
  });
});

describe("edge declares RUN_CLASS and reads verify as validate-only", () => {
  test("RUN_CLASS is a declared string parameter defaulting to build", () => {
    assert.match(edge, /string\(\s*name: 'RUN_CLASS',\s*defaultValue: 'build'/);
  });

  test("computeValidateOnly coalesces the param and treats verify as validate-only", () => {
    const def = topLevelDef(edge, "computeValidateOnly");
    assert.match(def, /params\.RUN_CLASS \?: 'build'/);
    assert.match(def, /runClass == 'verify'/);
  });
});
