// Native seam guidance: the declaration and method schemas hold every bound artifact.
// Runtime registration is verified read-only by hook-package.test.mjs.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";

const root = fileURLToPath(new URL("../../../../../", import.meta.url));
const read = async (path) =>
  JSON.parse(await readFile(resolve(root, path), "utf8"));

const DECLARATIONS = [
  "elohim/sdk/working-version.seam.json",
  "elohim/sdk/app-lifecycle.seam.json",
  "elohim/elohim-storage/.epr-meta/publication-authority.seam.json",
  "elohim/elohim-storage/.epr-meta/peer-serving.seam.json",
  "elohim/eprfs/epr-cli/developer-composition.seam.json",
];

test("schema validates every declaration the opt-in binding names and rejects missing or unknown fields", async () => {
  const validate = new Ajv2020().compile(
    await read("elohim/sdk/schemas/v1/seam-guidance.schema.json"),
  );
  for (const path of DECLARATIONS) {
    const declaration = await read(path);
    assert.ok(validate(declaration), `${path}: ${JSON.stringify(validate.errors)}`);
    assert.equal(validate({ ...declaration, mystery: true }), false);
    assert.equal(validate({ ...declaration, detection: [] }), false);
    for (const key of Object.keys(declaration)) {
      const invalid = { ...declaration };
      delete invalid[key];
      assert.equal(validate(invalid), false, `${path}: ${key}`);
    }
  }
});

test("the opt-in binding documented in the composition map names exactly these declarations", async () => {
  const doc = await readFile(resolve(root, ".epr-meta/composition.md"), "utf8");
  const block = doc.split("```yaml")[1]?.split("```")[0] ?? "";
  const named = [...block.matchAll(/^\s+- (\S+\.seam\.json)$/gm)].map((m) => m[1]);
  assert.deepEqual(named, DECLARATIONS);
  assert.match(block, /validator: epr:validator-native-seam-guidance/);
});

test("every cited contract and interface needle resolves in the tree", async () => {
  for (const path of DECLARATIONS) {
    const declaration = await read(path);
    for (const source of [
      ...declaration.authoritativeContracts,
      ...declaration.reusableInterfaces,
    ]) {
      const text = await readFile(resolve(root, source.path), "utf8");
      assert.ok(text.includes(source.needle), `${path}: ${source.path}`);
    }
  }
});

test("classifier method and its application are governed algorithm-content", async () => {
  const validate = new Ajv2020().compile(
    await read("elohim/sdk/schemas/v1/seam-guidance-method.schema.json"),
  );
  const method = await read(".epr-meta/elohim/algorithms/seam-guidance.json");
  assert.ok(validate(method), JSON.stringify(validate.errors));
  for (const application of [
    { ...method.application, mode: "blocking" },
    { ...method.application, inference: "remote" },
    { ...method.application, maxCandidates: 4 },
    { ...method.application, packetBytes: 8193 },
  ])
    assert.equal(validate({ ...method, application }), false);
});
