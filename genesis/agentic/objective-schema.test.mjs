import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import Ajv from "ajv";
import addFormats from "ajv-formats";
import { loadObjective } from "./objective.mjs";

const schema = JSON.parse(
  readFileSync(".claude/schemas/objective.schema.json", "utf8"),
);
const ajv = new Ajv({ allErrors: true, strict: false });
addFormats(ajv);
const validate = ajv.compile(schema);

test("JSON and YAML share strict validation and the 2400-minute ceiling", async () => {
  const { mkdtempSync, writeFileSync, rmSync } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const { join } = await import("node:path");
  const { default: YAML } = await import("yaml");
  const dir = mkdtempSync(join(tmpdir(), "shift-objective-"));
  const objective = {
    name: "shared-delivery",
    description: "A bounded delivery with a fixed deadline",
    measure: { type: "cmd", run: "echo 1" },
    target: {
      predicate: "==",
      value: 1,
      stability: { consecutive: 2, across_triggers: true },
    },
    baseline: { predicate: ">=", value: 0 },
    budget: { iterations: 20, wall_clock_min: 2400 },
    scope: { paths: ["genesis/agentic/**"] },
  };
  try {
    for (const [suffix, encode] of [
      ["json", JSON.stringify],
      ["yaml", YAML.stringify],
    ]) {
      const file = join(dir, `objective.${suffix}`);
      writeFileSync(file, encode(objective));
      assert.deepEqual(loadObjective(file), objective);
      writeFileSync(
        file,
        encode({
          ...objective,
          budget: { iterations: 20, wall_clock_min: 2401 },
        }),
      );
      assert.throws(() => loadObjective(file), /Invalid objective/);
    }
  } finally {
    rmSync(dir, { recursive: true });
  }
});

test("valid binary objective passes", () => {
  const obj = {
    name: "lift-edge-green",
    description: "Make alpha-deploy stage pass",
    measure: { type: "cmd", run: "echo 1" },
    target: {
      predicate: ">=",
      value: 1,
      stability: { consecutive: 2, across_triggers: true },
    },
    baseline: { predicate: ">=", value: 0 },
    budget: { iterations: 10, wall_clock_min: 480 },
    scope: { paths: ["genesis/orchestrator/**"] },
  };
  const ok = validate(obj);
  assert.equal(ok, true, JSON.stringify(validate.errors));
});

test("valid progressive objective passes", () => {
  const obj = {
    name: "lift-gherkin-rate",
    description: "Raise pass rate from 5% to 20%",
    measure: { type: "cmd", run: "echo 0.2" },
    target: {
      predicate: ">=",
      value: 0.2,
      stability: { consecutive: 2, across_triggers: true },
    },
    baseline: { predicate: ">=", value: 0.05 },
    budget: { iterations: 10, wall_clock_min: 480 },
    scope: { paths: ["genesis/a2o/**"] },
  };
  assert.equal(validate(obj), true, JSON.stringify(validate.errors));
});

test("rejects missing measure", () => {
  const obj = {
    name: "bad",
    description: "no measure",
    target: {
      predicate: ">=",
      value: 1,
      stability: { consecutive: 2, across_triggers: true },
    },
    baseline: { predicate: ">=", value: 0 },
    budget: { iterations: 1, wall_clock_min: 1 },
    scope: { paths: ["**"] },
  };
  assert.equal(validate(obj), false);
});

test("rejects unknown predicate", () => {
  const obj = {
    name: "bad",
    description: "unknown predicate",
    measure: { type: "cmd", run: "echo" },
    target: {
      predicate: "~~",
      value: 1,
      stability: { consecutive: 2, across_triggers: true },
    },
    baseline: { predicate: ">=", value: 0 },
    budget: { iterations: 1, wall_clock_min: 1 },
    scope: { paths: ["**"] },
  };
  assert.equal(validate(obj), false);
});

test("rejects missing stability", () => {
  const obj = {
    name: "bad",
    description: "stability missing",
    measure: { type: "cmd", run: "echo" },
    target: { predicate: ">=", value: 1 },
    baseline: { predicate: ">=", value: 0 },
    budget: { iterations: 1, wall_clock_min: 1 },
    scope: { paths: ["**"] },
  };
  assert.equal(validate(obj), false);
});
