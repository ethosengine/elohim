import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { validateInputs } from "./package-compute-worker.mjs";
const input = JSON.parse(
  readFileSync(new URL("./edge-1672-inputs.json", import.meta.url)),
);
test("all recovered inputs are immutable; source remains original tools checkout", () =>
  assert.equal(
    validateInputs(input).sourceCommit,
    "04ef84c0798e63d5e0a3e95b55f4a75c71077ac5",
  ));
test("refuses floating input or wrong repository before any subprocess", () => {
  for (const key of [
    "tools",
    "storage",
    "doorway",
    "relay",
    "beacon",
    "happArtifact",
  ]) {
    assert.throws(() =>
      validateInputs({
        ...input,
        [key]:
          "harbor.ethosengine.com/ethosengine/elohim-compute-worker:latest",
      }),
    );
  }
  assert.throws(() => validateInputs({ ...input, sourceCommit: "HEAD" }));
});

import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  chmodSync,
  symlinkSync,
  existsSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync, execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
const assembly = "b".repeat(40);
const finalImage =
  "harbor.ethosengine.com/ethosengine/elohim-compute-worker@sha256:" +
  "c".repeat(64);
function attempt(t, mode = "", config = input) {
  const root = mkdtempSync(join(tmpdir(), "package-ci-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  mkdirSync(join(root, "bin"));
  mkdirSync(join(root, "genesis/agentic/compute"), { recursive: true });
  mkdirSync(join(root, "compute-worker-image"));
  writeFileSync(
    join(root, "genesis/agentic/compute/Dockerfile.mesh"),
    "FROM pinned\n",
  );
  writeFileSync(join(root, "compute-worker-image/image.json"), "stale-success");
  writeFileSync(
    join(root, "inputs.json"),
    typeof config === "string" ? config : JSON.stringify(config),
  );
  const fake = `#!${process.execPath}
const fs=require('node:fs'),path=require('node:path');
const cmd=path.basename(process.argv[1]),a=process.argv.slice(2),root=process.env.FAKE_ROOT,mode=process.env.FAKE_MODE;
fs.appendFileSync(path.join(root,'calls'),JSON.stringify({cmd,a})+'\\n');
if(cmd==='git'){console.log(a.includes('--show-toplevel')?root:'${assembly}');process.exit(0);}
if(cmd==='oras'){fs.writeFileSync(path.join(a[a.indexOf('--output')+1],'elohim.happ'),'exact-pinned-happ');process.exit(0);}
const op=a[2];
if(op==='run')process.exit(99);
if(op==='image'){
 const built=a[4].includes(':package-');
 console.log(JSON.stringify([{Config:{Labels:{'org.opencontainers.image.revision':mode==='label'&&!built?'wrong':'${input.sourceCommit}','org.elohim.assembly.revision':'${assembly}'}},RepoDigests:['${finalImage}']}])); process.exit(0);
}
if(op==='build'){
 fs.copyFileSync(path.join(a.at(-1),'inputs.json'),path.join(root,'build-inputs.json'));
 if(mode==='build')process.exit(23);
}
if(op==='push'&&mode==='push')process.exit(24);
`;
  writeFileSync(join(root, "bin/fake"), fake);
  chmodSync(join(root, "bin/fake"), 0o755);
  for (const name of ["git", "nerdctl", "oras"])
    symlinkSync("fake", join(root, "bin", name));
  const result = spawnSync(
    process.execPath,
    [
      fileURLToPath(new URL("./package-compute-worker.mjs", import.meta.url)),
      "--inputs",
      join(root, "inputs.json"),
    ],
    {
      cwd: root,
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: join(root, "bin") + ":" + process.env.PATH,
        FAKE_ROOT: root,
        FAKE_MODE: mode,
      },
    },
  );
  const calls = readFileSync(join(root, "calls"), "utf8")
    .trim()
    .split("\n")
    .map(JSON.parse);
  return {
    root,
    result,
    calls,
    success: existsSync(join(root, "compute-worker-image/image.json")),
  };
}
test("integration packages only immutable inputs, avoids client-side container mounts and records derived provenance/hash", (t) => {
  const r = attempt(t);
  assert.equal(r.result.status, 0, r.result.stderr);
  assert(r.success);
  const report = JSON.parse(
    readFileSync(join(r.root, "compute-worker-image/image.json")),
  );
  assert.equal(report.image, finalImage);
  assert.equal(report.sourceCommit, input.sourceCommit);
  assert.equal(report.assemblyCommit, assembly);
  assert.equal(
    report.happSha256,
    createHash("sha256").update("exact-pinned-happ").digest("hex"),
  );
  assert.deepEqual(
    JSON.parse(readFileSync(join(r.root, "build-inputs.json"))),
    Object.fromEntries(Object.entries(report).filter(([k]) => k !== "image")),
  );
  assert(!r.calls.some((c) => c.cmd === "nerdctl" && c.a[2] === "run"));
  const builds = r.calls.filter(
    (c) => c.cmd === "nerdctl" && c.a[2] === "build",
  );
  assert.equal(builds.length, 1);
  for (const key of ["tools", "storage", "doorway", "relay", "beacon"])
    assert(builds[0].a.some((v) => v.endsWith("=" + input[key])));
  assert.deepEqual(r.calls.find((c) => c.cmd === "oras").a.slice(0, 2), [
    "pull",
    input.happArtifact,
  ]);
  assert(r.calls.every((c) => ["git", "nerdctl", "oras"].includes(c.cmd)));
  assert(!JSON.stringify(r.calls).includes("build-compute-worker.sh"));
});
for (const mode of ["label", "build", "push"])
  test(`integration ${mode} failure removes stale success and refuses`, (t) => {
    const r = attempt(t, mode);
    assert.notEqual(r.result.status, 0);
    assert.equal(r.success, false);
    if (mode === "label")
      assert(!r.calls.some((c) => c.cmd === "nerdctl" && c.a[2] === "build"));
  });
for (const [name, config] of [
  ["malformed", "{"],
  ["image", { ...input, image: finalImage }],
  ["assemblyCommit", { ...input, assemblyCommit: assembly }],
  ["floating", { ...input, storage: "latest" }],
])
  test(`integration rejects ${name} before external image commands and clears stale evidence`, (t) => {
    const r = attempt(t, "", config);
    assert.notEqual(r.result.status, 0);
    assert.equal(r.success, false);
    assert(r.calls.every((c) => c.cmd === "git"));
  });

test("BuildKit checkout verifier executes successfully only for the declared source", (t) => {
  const root = mkdtempSync(join(tmpdir(), "package-head-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const repo = join(root, "repo");
  mkdirSync(repo);
  execFileSync("git", ["init", "-q", repo]);
  execFileSync("git", [
    "-C",
    repo,
    "-c",
    "user.name=Fixture",
    "-c",
    "user.email=fixture@example.test",
    "-c",
    "core.hooksPath=/dev/null",
    "commit",
    "--allow-empty",
    "-qm",
    "fixture",
  ]);
  const commit = execFileSync("git", ["-C", repo, "rev-parse", "HEAD"], {
    encoding: "utf8",
  }).trim();
  const dockerfile = readFileSync(
    new URL("../../agentic/compute/Dockerfile.mesh", import.meta.url),
    "utf8",
  );
  const instruction = dockerfile.match(/^RUN node -e '(.+)'$/m)?.[1];
  assert(instruction);
  const code = instruction.replaceAll("/opt/compute/", root + "/");
  writeFileSync(
    join(root, "inputs.json"),
    JSON.stringify({ sourceCommit: commit }),
  );
  assert.equal(spawnSync(process.execPath, ["-e", code]).status, 0);
  for (const sourceCommit of ["0".repeat(40), "HEAD"]) {
    writeFileSync(join(root, "inputs.json"), JSON.stringify({ sourceCommit }));
    assert.notEqual(spawnSync(process.execPath, ["-e", code]).status, 0);
  }
});
