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
  for (const path of [
    "../../../rakia/work-composition/.epr-meta/progress.json",
    "../../../eprfs/epr-cli/tests/fixtures/views/orchard/composition.json",
  ]) {
    const value = read(path);
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
  const manifest = read("../../../rakia/work-composition/manifest.json");
  assert.ok(
    validateManifest(manifest),
    JSON.stringify(validateManifest.errors),
  );
  const empty = { ...manifest };
  delete empty.workComposition;
  assert.equal(validateManifest(empty), false);
});
