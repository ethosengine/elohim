import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { validateInputs } from "./package-compute-worker.mjs";
const rebuildInput = JSON.parse(
  readFileSync(new URL("./edge-1672-inputs.json", import.meta.url)),
);
const input = {
  ...rebuildInput,
  sourceCommit: "04ef84c0798e63d5e0a3e95b55f4a75c71077ac5",
  tools:
    "harbor.ethosengine.com/ethosengine/elohim-compute-worker@sha256:33d11def5d2982b42fa00ca9138eb78ae5643307ebc4b79822404fc63ec510ef",
};
delete input.toolsBase;
test("all recovered inputs are immutable; source remains original tools checkout", () =>
  assert.equal(
    validateInputs(input).sourceCommit,
    "04ef84c0798e63d5e0a3e95b55f4a75c71077ac5",
  ));
test("reviewed rebuild pins an existing immutable source commit", () => {
  const selected = validateInputs(rebuildInput);
  assert.match(selected.sourceCommit, /^[0-9a-f]{40}$/);
  assert.equal(
    execFileSync("git", ["cat-file", "-t", selected.sourceCommit], {
      cwd: fileURLToPath(new URL("../../..", import.meta.url)),
      encoding: "utf8",
    }).trim(),
    "commit",
  );
  assert.ok(selected.toolsBase);
});
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
  statSync,
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
const repo = "harbor.ethosengine.com/ethosengine/elohim-compute-worker";
const sha = (bytes) =>
  "sha256:" + createHash("sha256").update(bytes).digest("hex");
function attempt(t, mode = "", requested = input) {
  const root = mkdtempSync(join(tmpdir(), "package-ci-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  for (const dir of [
    "bin",
    "registry",
    "proof",
    "auth",
    "genesis/agentic/compute",
    "compute-worker-image",
  ])
    mkdirSync(join(root, dir), { recursive: true });
  const source =
    typeof requested === "object" ? requested.sourceCommit : input.sourceCommit;
  const proof = `compute-worker registry export preflight\nassembly=${assembly}\n`;
  writeFileSync(
    join(root, "proof/export-proof.txt"),
    mode === "preflight-content" ? "wrong-proof" : proof,
  );
  execFileSync("tar", [
    "-czf",
    join(root, "layer.tgz"),
    "-C",
    join(root, "proof"),
    "export-proof.txt",
  ]);
  const layer = readFileSync(join(root, "layer.tgz"));
  const layerDescriptor = {
    mediaType: "application/vnd.oci.image.layer.v1.tar+gzip",
    digest: sha(layer),
    size: layer.length,
  };
  writeFileSync(join(root, "registry", sha(layer).slice(7)), layer);
  const images = {};
  for (const kind of ["preflight", "tools", "final"]) {
    const labels =
      kind === "preflight"
        ? {
            "org.elohim.export-preflight":
              mode === "preflight-label" ? "wrong" : assembly,
          }
        : {
            "org.opencontainers.image.revision":
              kind === "tools" && mode === "label" ? "wrong" : source,
            ...(kind === "final"
              ? {
                  "org.elohim.assembly.revision":
                    mode === "final-label" ? "wrong" : assembly,
                }
              : {}),
          };
    const config = Buffer.from(
      JSON.stringify({
        architecture: "amd64",
        os: "linux",
        config: { Labels: labels, Env: ["SECRET=PRIVATE_CONFIG"] },
      }) + "\n",
    );
    const manifest = Buffer.from(
      JSON.stringify({
        schemaVersion: 2,
        mediaType: "application/vnd.oci.image.manifest.v1+json",
        config: {
          mediaType: "application/vnd.oci.image.config.v1+json",
          digest: sha(config),
          size: config.length,
        },
        layers: kind === "preflight" ? [layerDescriptor] : [],
      }) + "\n",
    );
    writeFileSync(join(root, "registry", sha(config).slice(7)), config);
    writeFileSync(join(root, "registry", sha(manifest).slice(7)), manifest);
    images[kind] = { image: `${repo}@${sha(manifest)}`, digest: sha(manifest) };
  }
  writeFileSync(join(root, "images.json"), JSON.stringify(images));
  const config =
    typeof requested === "object" && requested.tools
      ? { ...requested, tools: images.tools.image }
      : requested;
  writeFileSync(
    join(root, "inputs.json"),
    typeof config === "string" ? config : JSON.stringify(config),
  );
  writeFileSync(
    join(root, "genesis/agentic/compute/Dockerfile.mesh"),
    "FROM pinned\n",
  );
  for (const name of ["image.json", "registry-preflight.json"])
    writeFileSync(join(root, "compute-worker-image", name), "stale-success");
  const fake = `#!${process.execPath}
const fs=require('node:fs'),path=require('node:path'),cp=require('node:child_process');
const cmd=path.basename(process.argv[1]),a=process.argv.slice(2),root=process.env.FAKE_ROOT,mode=process.env.FAKE_MODE;
const images=JSON.parse(fs.readFileSync(path.join(root,'images.json')));
fs.appendFileSync(path.join(root,'calls'),JSON.stringify({cmd,a,builder:process.env.COMPUTE_WORKER_BUILDER,metadata:process.env.COMPUTE_WORKER_METADATA_FILE,dockerConfig:process.env.DOCKER_CONFIG})+'\\n');
if(cmd==='timeout'){const r=cp.spawnSync('/usr/bin/timeout',a,{stdio:'inherit'});process.exit(r.status??1);}
if(cmd==='git'){console.log(a.includes('--show-toplevel')?root:'${assembly}');process.exit(0);}
if(cmd==='nerdctl')process.exit(99);
if(cmd==='oras'){
 if(a[a.indexOf('--registry-config')+1]!==path.join(root,'auth/config.json'))process.exit(41);
 if(a[0]==='pull'){fs.writeFileSync(path.join(a[a.indexOf('--output')+1],'elohim.happ'),'exact-pinned-happ');process.exit(0);}
 const ref=a[a.indexOf('--registry-config')-1],digest=ref.split('@')[1];
 if(mode==='registry'&&digest===images.preflight.digest)process.exit(42);
 let bytes=fs.readFileSync(path.join(root,'registry',digest.slice(7)));
 if(mode==='manifest-bytes'&&a[0]==='manifest')bytes=Buffer.concat([bytes,Buffer.from(' ')]);
 process.stdout.write(bytes);process.exit(0);
}
if(cmd==='bash'){
 if(mode==='source-build')process.exit(25);
 if(process.env.COMPUTE_WORKER_BUILDER!=='buildctl')process.exit(43);
 fs.writeFileSync(process.env.COMPUTE_WORKER_METADATA_FILE,JSON.stringify({'containerimage.digest':images.tools.digest}));process.exit(0);
}
if(cmd==='buildctl'){
 const context=a.find(v=>v.startsWith('context=')).slice(8),filename=a.find(v=>v.startsWith('filename=')).slice(9);
 const kind=filename==='Dockerfile.base-check'?'base':context.endsWith('/preflight')?'preflight':'final';
 if(kind==='base'){fs.copyFileSync(path.join(context,filename),path.join(root,'base-check'));if(a.includes('--output'))process.exit(44);if(mode==='base-build')process.exit(45);process.exit(0);}
 if(mode===kind+'-build')process.exit(23);
 if(kind==='final')fs.copyFileSync(path.join(context,'inputs.json'),path.join(root,'build-inputs.json'));
 if(mode==='metadata'&&kind==='preflight')process.exit(0);
 fs.writeFileSync(a[a.indexOf('--metadata-file')+1],JSON.stringify({'containerimage.digest':images[kind].digest}));process.exit(0);
}
process.exit(98);
`;
  writeFileSync(join(root, "bin/fake"), fake);
  chmodSync(join(root, "bin/fake"), 0o755);
  for (const name of ["git", "nerdctl", "oras", "bash", "buildctl", "timeout"])
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
      timeout: 15000,
      env: {
        ...process.env,
        PATH: join(root, "bin") + ":" + process.env.PATH,
        FAKE_ROOT: root,
        FAKE_MODE: mode,
        DOCKER_CONFIG: join(root, "auth"),
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
    config,
    images,
    success: existsSync(join(root, "compute-worker-image/image.json")),
  };
}
test("integration publishes by BuildKit and verifies immutable registry bytes without host image operations", (t) => {
  const r = attempt(t);
  assert.equal(r.result.status, 0, r.result.stderr);
  const report = JSON.parse(
    readFileSync(join(r.root, "compute-worker-image/image.json")),
  );
  assert.equal(report.image, r.images.final.image);
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
  assert(!r.calls.some((c) => c.cmd === "nerdctl" || c.cmd === "bash"));
  const builds = r.calls.filter((c) => c.cmd === "buildctl");
  assert.equal(builds.length, 2);
  assert(
    builds.every((c) =>
      c.a.some((v) => v.startsWith("type=image,") && v.endsWith(",push=true")),
    ),
  );
  for (const key of ["tools", "storage", "doorway", "relay", "beacon"])
    assert(builds[1].a.some((v) => v.endsWith("=" + r.config[key])));
  const preflight = JSON.parse(
    readFileSync(join(r.root, "compute-worker-image/registry-preflight.json")),
  );
  assert.equal(preflight.outcome, "passed");
  // Jenkins archives these as another user: public artifacts are 0755/0644.
  assert.equal(statSync(join(r.root, "compute-worker-image")).mode & 0o777, 0o755);
  for (const name of ["image.json", "inputs.json", "registry-preflight.json"])
    assert.equal(
      statSync(join(r.root, "compute-worker-image", name)).mode & 0o777,
      0o644,
      name,
    );
  assert.equal(preflight.image, r.images.preflight.image);
  assert(!JSON.stringify(preflight).includes("PRIVATE_CONFIG"));
  assert(!r.result.stdout.includes("PRIVATE_CONFIG"));
  for (const c of r.calls.filter((c) => c.cmd === "timeout")) {
    assert.deepEqual(c.a.slice(0, 2), ["--signal=TERM", "--kill-after=5"]);
    assert(Number(c.a[2]) > 0 && Number(c.a[2]) <= 4800);
  }
  const reads = r.calls.filter((c) => c.cmd === "oras" && c.a[0] !== "pull");
  assert(reads.every((c) => c.a.some((v) => v.startsWith(repo + "@sha256:"))));
});
for (const mode of [
  "preflight-build",
  "registry",
  "manifest-bytes",
  "metadata",
  "preflight-label",
  "preflight-content",
])
  test(`preflight ${mode} failure clears success and prevents cold compilation`, (t) => {
    const r = attempt(t, mode, rebuildInput);
    assert.notEqual(r.result.status, 0);
    assert.equal(r.success, false);
    assert(
      r.result.stderr.includes(
        `phase=${mode === "preflight-build" ? "preflight-export" : "preflight-verify"};`,
      ),
    );
    assert(
      !existsSync(join(r.root, "compute-worker-image/registry-preflight.json")),
    );
    assert(!r.calls.some((c) => c.cmd === "bash"));
    assert(
      !r.calls.some(
        (c) =>
          c.cmd === "buildctl" &&
          c.a.includes("filename=Dockerfile.base-check"),
      ),
    );
  });
for (const mode of [
  "base-build",
  "source-build",
  "label",
  "final-build",
  "final-label",
])
  test(`rebuild ${mode} failure leaves no successful artifact`, (t) => {
    const r = attempt(t, mode, rebuildInput);
    assert.notEqual(r.result.status, 0);
    assert.equal(r.success, false);
    const phase = {
      "base-build": "base-check",
      "source-build": "tools-export",
      label: "tools-verify",
      "final-build": "final-export",
      "final-label": "final-verify",
    }[mode];
    assert(r.result.stderr.includes(`phase=${phase};`));
  });
for (const [name, config] of [
  ["malformed", "{"],
  ["image", { ...input, image: "forbidden" }],
  ["assemblyCommit", { ...input, assemblyCommit: assembly }],
  ["floating", { ...input, storage: "latest" }],
])
  test(`rejects ${name} before external image operations and clears stale evidence`, (t) => {
    const r = attempt(t, "", config);
    assert.notEqual(r.result.status, 0);
    assert.equal(r.success, false);
    assert(r.calls.every((c) => ["git", "timeout"].includes(c.cmd)));
  });
test("explicit compatible-base rebuild retains source preparation and selects direct publication only here", (t) => {
  const r = attempt(t, "", rebuildInput);
  assert.equal(r.result.status, 0, r.result.stderr);
  const sourceBuild = r.calls.find((c) => c.cmd === "bash");
  assert.equal(sourceBuild.builder, "buildctl");
  assert(sourceBuild.metadata.startsWith("/tmp/compute-worker-package-"));
  assert(!existsSync(sourceBuild.metadata));
  assert.deepEqual(sourceBuild.a.slice(1), [
    "--source",
    rebuildInput.toolsBase,
    `${repo}:tools-rebuild-${assembly}`,
    rebuildInput.sourceCommit,
  ]);
  const base = r.calls.find(
    (c) =>
      c.cmd === "buildctl" && c.a.includes("filename=Dockerfile.base-check"),
  );
  assert(base && !base.a.includes("--output"));
  assert(r.calls.indexOf(base) < r.calls.indexOf(sourceBuild));
  assert(
    readFileSync(join(r.root, "base-check"), "utf8").includes("x86-64(-v2)?"),
  );
  const report = JSON.parse(
    readFileSync(join(r.root, "compute-worker-image/image.json")),
  );
  assert.equal(report.tools, r.images.tools.image);
  assert.equal(report.base, rebuildInput.toolsBase);
  assert.equal(report.toolsBase, undefined);
  assert(!r.calls.some((c) => c.cmd === "nerdctl"));
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

test("tools source choice is exclusive and immutable", () => {
  assert.throws(() => validateInputs({ ...rebuildInput, tools: input.tools }));
  assert.throws(() =>
    validateInputs({
      ...rebuildInput,
      toolsBase:
        "harbor.ethosengine.com/devspaces/udi-plus-mem-rust-nix:v2-latest",
    }),
  );
  assert.throws(() =>
    validateInputs({ ...rebuildInput, base: rebuildInput.toolsBase }),
  );
});
test("ordinary publication selects the same reviewed CPU-compatible base", () => {
  const jenkins = readFileSync(
    new URL("../../../elohim/holochain/Jenkinsfile", import.meta.url),
    "utf8",
  );
  const publish = jenkins
    .split("def publishComputeWorkerIfRequested() {")[1]
    .split("pipeline {")[0];
  assert(
    publish.includes(
      "readJSON file: 'genesis/scripts/ci/edge-1672-inputs.json'",
    ),
  );
  assert(
    publish.includes(
      'withEnv(["COMPUTE_WORKER_BASE_IMAGE=${workerInputs.toolsBase}"])',
    ),
  );
});

for (const mode of [
  "success",
  "install-failure",
  "login-failure",
  "packaging-failure",
])
  test(`shell ${mode} shares private registry auth outside the workspace and cleans it`, (t) => {
    const root = mkdtempSync(join(tmpdir(), "package-auth-"));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    mkdirSync(join(root, "bin"));
    mkdirSync(join(root, "workspace-tmp"));
    mkdirSync(join(root, "compute-worker-image"));
    const outputs = ["image.json", "inputs.json", "registry-preflight.json"];
    for (const name of outputs)
      writeFileSync(join(root, "compute-worker-image", name), "stale-success");
    const fake = `#!${process.execPath}
const fs=require('node:fs'),path=require('node:path');
const cmd=path.basename(process.argv[1]),a=process.argv.slice(2),root=process.env.FAKE_ROOT;
if(cmd==='git'){console.log(root);process.exit(0);}
if(cmd==='curl')process.exit(1);
const config=process.env.DOCKER_CONFIG;
if(!config?.startsWith('/tmp/compute-worker-tools.')||config.startsWith(root))process.exit(40);
if((fs.statSync(config).mode&0o777)!==0o700)process.exit(41);
fs.appendFileSync(path.join(root,'calls'),JSON.stringify({cmd,a,config})+'\\n');
if(cmd==='oras'){
 if(a[a.indexOf('--registry-config')+1]!==path.join(config,'config.json'))process.exit(42);
 if(fs.readFileSync(0,'utf8')!==process.env.HARBOR_PASSWORD)process.exit(43);
 fs.writeFileSync(path.join(config,'config.json'),'PRIVATE_AUTH');
 process.exit(process.env.FAKE_MODE==='login-failure'?1:0);
}
if(cmd==='node'){
 if(fs.readFileSync(path.join(config,'config.json'),'utf8')!=='PRIVATE_AUTH')process.exit(44);
 process.exit(process.env.FAKE_MODE==='packaging-failure'?1:0);
}
process.exit(99);
`;
    writeFileSync(join(root, "bin/fake"), fake);
    chmodSync(join(root, "bin/fake"), 0o755);
    for (const name of [
      "git",
      "node",
      "curl",
      ...(mode === "install-failure" ? [] : ["oras"]),
    ])
      symlinkSync("fake", join(root, "bin", name));
    for (const name of ["mktemp", "rm", "mkdir", "timeout"])
      symlinkSync(`/usr/bin/${name}`, join(root, "bin", name));
    const result = spawnSync(
      "/bin/bash",
      [fileURLToPath(new URL("./package-compute-worker.sh", import.meta.url))],
      {
        encoding: "utf8",
        timeout: 10000,
        env: {
          ...process.env,
          PATH: join(root, "bin"),
          TMPDIR: join(root, "workspace-tmp"),
          FAKE_ROOT: root,
          FAKE_MODE: mode,
          HARBOR_USERNAME: "fixture",
          HARBOR_PASSWORD: "PRIVATE_PASSWORD",
        },
      },
    );
    assert.equal(result.status === 0, mode === "success", result.stderr);
    for (const name of outputs)
      assert(!existsSync(join(root, "compute-worker-image", name)));
    if (mode === "install-failure") return;
    const calls = readFileSync(join(root, "calls"), "utf8")
      .trim()
      .split("\n")
      .map(JSON.parse);
    assert.equal(calls.length, mode === "login-failure" ? 1 : 2);
    assert(calls.every((c) => c.config === calls[0].config));
    assert(!existsSync(calls[0].config));
    assert(
      !JSON.stringify({
        calls,
        stdout: result.stdout,
        stderr: result.stderr,
      }).includes("PRIVATE_"),
    );
  });
