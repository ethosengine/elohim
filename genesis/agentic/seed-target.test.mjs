import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");

function capture(args) {
  const scratch = mkdtempSync(join(tmpdir(), "seed-target-"));
  try {
    const output = join(scratch, "argv.json");
    writeFileSync(
      join(scratch, "pnpm"),
      '#!/usr/bin/env node\nrequire("node:fs").writeFileSync(process.env.SEED_TEST_OUTPUT, JSON.stringify(process.argv.slice(2)));\n',
      { mode: 0o755 },
    );
    const result = spawnSync("just", ["seed", ...args], {
      cwd: root,
      env: {
        ...process.env,
        PATH: `${scratch}:${process.env.PATH}`,
        SEED_TEST_OUTPUT: output,
      },
      encoding: "utf8",
    });
    return {
      ...result,
      argv:
        result.status === 0 ? JSON.parse(readFileSync(output, "utf8")) : null,
    };
  } finally {
    rmSync(scratch, { recursive: true, force: true });
  }
}

test("targeted seed preserves one logical ID as one argument and selects content only", () => {
  const id = "value-scanner:household mental load/%2F";
  const result = capture(["apply", "local", "content", "", id]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.argv, [
    "exec",
    "tsx",
    "src/seed.ts",
    "--content-only",
    `--ids=${id}`,
  ]);
});

test("seed IDs are shell data and cannot execute command substitution", () => {
  const id = "literal'$(exit 79)`exit 80`";
  const result = capture(["apply", "local", "content", "", id]);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(result.argv.at(-1), `--ids=${id}`);
});

test("ordinary seed invocation retains its existing arguments", () => {
  const result = capture(["apply", "local", "content", "5"]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.argv, ["exec", "tsx", "src/seed.ts", "--limit", "5"]);
});

test("targeting is refused for validation and unrelated write scopes", () => {
  for (const args of [
    ["validate", "local", "content", "", "one"],
    ["apply", "local", "humans", "", "one"],
  ]) {
    const result = capture(args);
    assert.equal(result.status, 2);
    assert.match(result.stderr, /seed IDs require apply content/);
  }
});
