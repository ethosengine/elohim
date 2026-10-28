import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const Ajv = require("ajv/dist/2020");
const read = (p) =>
  JSON.parse(readFileSync(new URL(p, import.meta.url), "utf8"));
const schema = read("../v1/manifest/work-composition.schema.json");
const validate = new Ajv({ strict: false }).compile(schema);
test("coding and orchard share a bounded data-only composition contract", () => {
  // Local fixture copied from the rescued native orchard; no native build or submodule required.
  const orchard = read("./fixtures/work-composition/orchard.json");
  const coding = structuredClone(orchard);
  coding.purpose = "Coding work";
  coding.groups[0].label = "Milestones";
  coding.groups[1].label = "Issues";
  for (const value of [orchard, coding]) {
    assert.ok(validate(value), JSON.stringify(validate.errors));
    assert.equal(validate({ ...value, execute: "run code" }), false);
    assert.equal(validate({ ...value, version: 3 }), false);
    const bad = structuredClone(value);
    bad.groups[0].view = "../escape";
    assert.equal(validate(bad), false);
  }
});

test("composition-only apps validate without inventing content types", () => {
  const ajv = new Ajv({ strict: false });
  for (const name of [
    "substrate-signal",
    "instrument-archetype",
    "observation-polarity",
    "epr-kind",
    "session-lifecycle-state",
    "closure",
  ])
    ajv.addSchema(
      read(`../v1/enums/${name}.schema.json`),
      `epr:enums/${name}.schema.json`,
    );
  for (const name of ["pillar-projection", "observation-kind"])
    ajv.addSchema(
      read(`../v1/manifest/${name}.schema.json`),
      `epr:${name}.schema.json`,
    );
  const validateManifest = ajv.compile(
    read("../v1/manifest/app-manifest.schema.json"),
  );
  const manifest = {
    id: "bafkreicompositionfixture",
    name: "Orchard work",
    version: "1.0.0",
    workComposition: { $ref: "views/composition.json" },
  };
  assert.ok(
    validateManifest(manifest),
    JSON.stringify(validateManifest.errors),
  );
  assert.ok(validateManifest({ ...manifest, needs: {} }));
  assert.equal(
    validateManifest({ ...manifest, needs: { closure: [""] } }),
    false,
    "composition-only apps retain dev's first-reader gate for needs",
  );
  for (const ref of [
    "",
    "../escape",
    "/absolute",
    "views/../../escape",
    "views\\escape",
  ]) {
    assert.equal(
      validateManifest({ ...manifest, workComposition: { $ref: ref } }),
      false,
      ref,
    );
  }
  assert.equal(validateManifest({ ...manifest, workComposition: {} }), false);
  assert.equal(
    validateManifest({
      ...manifest,
      workComposition: { $ref: "view.json", execute: "code" },
    }),
    false,
  );
  assert.equal(
    validateManifest({
      ...manifest,
      vocabulary: { contentTypes: { article: {} } },
    }),
    false,
    "composition does not bypass vocabulary coupling validation",
  );
  const empty = { ...manifest };
  delete empty.workComposition;
  assert.equal(validateManifest(empty), false);
});
