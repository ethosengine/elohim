import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import Ajv2020 from "ajv/dist/2020.js";
import { mergeNativeRegistration, projectHook } from "./hook-package.mjs";

const root = fileURLToPath(new URL("../../../../../", import.meta.url));
const read = async (path) =>
  JSON.parse(await readFile(resolve(root, path), "utf8"));

test("schema validates every explicitly bound declaration and rejects missing or unknown fields", async () => {
  const validate = new Ajv2020().compile(
    await read("elohim/sdk/schemas/v1/seam-guidance.schema.json"),
  );
  for (const path of [
    "elohim/sdk/working-version.seam.json",
    "elohim/elohim-storage/.epr-meta/publication-authority.seam.json",
    "elohim/elohim-storage/.epr-meta/peer-serving.seam.json",
    "elohim/eprfs/epr-cli/developer-composition.seam.json",
  ]) {
    const declaration = await read(path);
    assert.ok(validate(declaration), JSON.stringify(validate.errors));
    assert.equal(validate({ ...declaration, mystery: true }), false);
    assert.equal(validate({ ...declaration, detection: [] }), false);
    for (const key of Object.keys(declaration)) {
      const invalid = { ...declaration };
      delete invalid[key];
      assert.equal(validate(invalid), false, key);
    }
  }
});

test("native runtime registration preserves unrelated hooks and is idempotent", async () => {
  const pkg = await read(
    ".epr-meta/elohim/packages/hooks/native-seam-guidance.json",
  );
  for (const runtime of ["claude", "codex"]) {
    const other = {
      matcher: "Bash",
      hooks: [{ type: "command", command: "other-tool", timeout: 9 }],
    };
    const before = {
      description: "preserve",
      hooks: { PreToolUse: [other], Stop: [other] },
    };
    const after = mergeNativeRegistration(before, pkg, runtime);
    assert.deepEqual(after.hooks.PreToolUse[0], other);
    assert.deepEqual(after.hooks.Stop, [other]);
    assert.equal(after.description, "preserve");
    assert.deepEqual(mergeNativeRegistration(after, pkg, runtime), after);
    assert.deepEqual(before.hooks.PreToolUse, [other]);
    assert.equal(
      await readFile(resolve(root, pkg.projections[runtime].path), "utf8"),
      projectHook(pkg),
    );
  }
  const legacy = structuredClone(pkg);
  legacy.metadata.sourceRuntime = "claude";
  assert.deepEqual(mergeNativeRegistration({}, legacy, "claude"), {});
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
