import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, mkdir, readFile, writeFile, rm } from "node:fs/promises";
import { execFileSync, spawnSync } from "node:child_process";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";

const script = fileURLToPath(
  new URL("./publish-compute-worker.sh", import.meta.url),
);
async function fixture(t) {
  const dir = await mkdtemp(join(tmpdir(), "publish-compute-worker-"));
  t.after(() => rm(dir, { recursive: true, force: true }));
  for (const path of [
    "bin",
    "scripts/ci",
    "genesis/agentic/compute",
    "compute-worker-image",
  ]) {
    await mkdir(join(dir, path), { recursive: true });
  }
  await writeFile(
    join(dir, "scripts/ci/build-compute-worker.sh"),
    "#!/bin/bash\nexit 0\n",
  );
  await writeFile(
    join(dir, "genesis/agentic/compute/Dockerfile.mesh"),
    "FROM scratch\n",
  );
  await writeFile(join(dir, "elohim.happ"), "pinned-happ");
  await writeFile(
    join(dir, "compute-worker-image/image.json"),
    '{"image":"stale-success"}',
  );
  await writeFile(
    join(dir, "bin/nerdctl"),
    `#!${process.execPath}
const fs = require('fs');
const args = process.argv.slice(2);
fs.appendFileSync(process.env.CALLS, JSON.stringify(args)+'\\n');
if (process.env.FAIL_PUSH === '1' && args.includes('push')) process.exit(1);
if (args.includes('inspect')) {
  const ref = args[4];
  const repo = ref.slice(0, ref.lastIndexOf(':'));
  const digest = require('crypto').createHash('sha256').update(ref).digest('hex');
  if (process.env.NO_DIGEST !== '1') console.log(repo+'@sha256:'+digest);
}
`,
    { mode: 0o755 },
  );
  execFileSync("git", ["init", "-q", dir]);
  execFileSync("git", [
    "-C",
    dir,
    "-c",
    "user.name=Test",
    "-c",
    "user.email=test@example.test",
    "commit",
    "--allow-empty",
    "-qm",
    "fixture",
  ]);
  const env = {
    ...process.env,
    PATH: `${dir}/bin:${process.env.PATH}`,
    CALLS: join(dir, "calls"),
    STORAGE_TAG: "storage-commit",
    DOORWAY_TAG: "doorway-commit",
    GIT_COMMIT_HASH: "source-commit",
  };
  return {
    dir,
    env,
    run: (extra = {}) =>
      spawnSync("bash", [script, join(dir, "elohim.happ")], {
        cwd: dir,
        env: { ...env, ...extra },
        encoding: "utf8",
      }),
  };
}

test("published worker records exact component digests and hApp bytes", async (t) => {
  const f = await fixture(t);
  const result = f.run();
  assert.equal(result.status, 0, result.stderr);
  const report = JSON.parse(
    await readFile(join(f.dir, "compute-worker-image/image.json"), "utf8"),
  );
  for (const key of [
    "image",
    "base",
    "tools",
    "storage",
    "doorway",
    "relay",
    "beacon",
  ])
    assert.match(report[key], /@sha256:[a-f0-9]{64}$/);
  assert.equal(
    report.happSha256,
    createHash("sha256").update("pinned-happ").digest("hex"),
  );
  const calls = (await readFile(f.env.CALLS, "utf8"))
    .trim()
    .split("\n")
    .map(JSON.parse);
  const build = calls.find((args) => args[2] === "build");
  for (const key of [
    "COMPUTE_TOOLS_IMAGE",
    "STORAGE_IMAGE",
    "DOORWAY_IMAGE",
    "RELAY_IMAGE",
    "BEACON_IMAGE",
  ])
    assert.ok(
      build.some(
        (arg) => arg.startsWith(`${key}=`) && /@sha256:[a-f0-9]{64}$/.test(arg),
      ),
    );
});

for (const [name, extra] of [
  ["unresolved digest", { NO_DIGEST: "1" }],
  ["failed publication", { FAIL_PUSH: "1" }],
]) {
  test(`${name} refuses and removes stale success evidence`, async (t) => {
    const f = await fixture(t);
    const result = f.run(extra);
    assert.notEqual(result.status, 0);
    await assert.rejects(
      readFile(join(f.dir, "compute-worker-image/image.json")),
      { code: "ENOENT" },
    );
  });
}
