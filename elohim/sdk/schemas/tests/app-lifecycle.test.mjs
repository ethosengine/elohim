import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { createRequire } from "node:module";
const require = createRequire(import.meta.url);
const Ajv = require("ajv/dist/2020");
const read = (path) =>
  JSON.parse(readFileSync(new URL(path, import.meta.url), "utf8"));
const fixtures = read(
  "../../../../crates/elohim-views/tests/fixtures/app-lifecycle.json",
);
const ajv = new Ajv({ strict: false });
// Mixed existing draft schemas use the same subset; normalize only the metaschema
// and ref resolution for this test, never the contract bytes or vocabulary.
function schema(name, dir = "views") {
  const value = read(`../v1/${dir}/${name}.schema.json`);
  delete value.$schema;
  value.$id = `https://schemas.test/${dir}/${name}.schema.json`;
  return value;
}
ajv.addSchema(schema("app-sync-urgency", "enums"));
for (const name of Object.keys(fixtures)) ajv.addSchema(schema(name));
test("native fixtures conform to the schema and reject undeclared fields", () => {
  for (const [name, value] of Object.entries(fixtures)) {
    const validate = ajv.getSchema(
      `https://schemas.test/views/${name}.schema.json`,
    );
    assert.ok(validate(value), `${name}: ${JSON.stringify(validate.errors)}`);
    assert.equal(validate({ ...value, undeclared: true }), false, name);
  }
});
test("unknown urgency and fabricated successful absence refuse", () => {
  const demand = ajv.getSchema(
    "https://schemas.test/views/app-sync-demand.schema.json",
  );
  assert.equal(
    demand({ ...fixtures["app-sync-demand"], urgency: "urgent" }),
    false,
  );
  const answer = ajv.getSchema(
    "https://schemas.test/views/app-release-answer.schema.json",
  );
  assert.equal(answer({ state: "absent", reason: "timeout" }), false);
  assert.equal(answer({ state: "present", value: null }), false);
});
