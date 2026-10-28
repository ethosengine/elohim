import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  rmSync,
  statSync,
  symlinkSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

const script = fileURLToPath(
  new URL("./build-compute-worker.sh", import.meta.url),
);
const base = `registry.example/runtime@sha256:${"a".repeat(64)}`;
const nativePaths = [
  "elohim/elohim-storage/src",
  "doorway/doorway-service/src",
];
const initialEpoch = 1700000000;
function git(directory, ...args) {
  const result = spawnSync(
    "git",
    [
      "-C",
      directory,
      "-c",
      "user.name=Fixture",
      "-c",
      "user.email=fixture@example.invalid",
      ...args,
    ],
    { encoding: "utf8" },
  );
  assert.equal(result.status, 0, result.stderr);
  return result.stdout.trim();
}
function commitAt(directory, message, epoch) {
  const result = spawnSync(
    "git",
    [
      "-C",
      directory,
      "-c",
      "user.name=Fixture",
      "-c",
      "user.email=fixture@example.invalid",
      "commit",
      "-qm",
      message,
    ],
    {
      encoding: "utf8",
      env: {
        ...process.env,
        GIT_AUTHOR_DATE: `${epoch} +0000`,
        GIT_COMMITTER_DATE: `${epoch} +0000`,
      },
    },
  );
  assert.equal(result.status, 0, result.stderr);
  return git(directory, "rev-parse", "HEAD");
}
function fixture(t, executor = true) {
  const root = mkdtempSync(join(tmpdir(), "compute-build-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  git(root, "init", "-q");
  const rakia = join(root, "elohim/rakia");
  mkdirSync(join(rakia, "rakia-executor"), { recursive: true });
  git(rakia, "init", "-q");
  const path = executor ? "rakia-executor/Cargo.toml" : "placeholder";
  writeFileSync(join(rakia, path), "committed executor\n");
  git(rakia, "add", path);
  git(rakia, "commit", "-qm", "fixture executor");
  const pin = git(rakia, "rev-parse", "HEAD");
  const fork = join(root, "elohim/holochain-conductor");
  mkdirSync(join(fork, "crates/hc"), { recursive: true });
  git(fork, "init", "-q");
  writeFileSync(join(fork, "crates/hc/Cargo.toml"), "committed fork CLI\n");
  git(fork, "add", "crates/hc/Cargo.toml");
  git(fork, "commit", "-qm", "fixture fork");
  const forkPin = git(fork, "rev-parse", "HEAD");
  const remotes = join(root, "fixture-remotes");
  mkdirSync(remotes);
  const rakiaRemote = join(remotes, "rakia.git");
  const forkRemote = join(remotes, "fork.git");
  git(root, "clone", "--bare", "-q", rakia, rakiaRemote);
  git(root, "clone", "--bare", "-q", fork, forkRemote);
  writeFileSync(
    join(root, ".gitmodules"),
    `[submodule "elohim/rakia"]\n\tpath = elohim/rakia\n\turl = ${rakiaRemote}\n[submodule "elohim/holochain-conductor"]\n\tpath = elohim/holochain-conductor\n\turl = ${forkRemote}\n\tupdate = none\n`,
  );
  git(
    root,
    "update-index",
    "--add",
    "--cacheinfo",
    `160000,${forkPin},elohim/holochain-conductor`,
  );
  writeFileSync(
    join(fork, "crates/hc/Cargo.toml"),
    "dirty fork must not ship\n",
  );
  writeFileSync(join(fork, "private-token"), "must not ship\n");
  git(
    fork,
    "config",
    "remote.private.url",
    "https://credential@do-not-copy.invalid/fork",
  );
  mkdirSync(join(root, "genesis/agentic/compute"), { recursive: true });
  writeFileSync(
    join(root, "genesis/agentic/compute/Dockerfile.source"),
    "FROM scratch\n",
  );
  writeFileSync(join(root, "source.txt"), "committed source\n");
  for (const path of nativePaths) {
    mkdirSync(join(root, path), { recursive: true });
    writeFileSync(join(root, path, "lib.rs"), "// committed native source\n");
  }
  git(
    root,
    "add",
    ".gitmodules",
    "source.txt",
    "genesis/agentic/compute/Dockerfile.source",
    ...nativePaths,
  );
  git(
    root,
    "update-index",
    "--add",
    "--cacheinfo",
    `160000,${pin},elohim/rakia`,
  );
  const commit = commitAt(root, "fixture root", initialEpoch);
  writeFileSync(join(root, "source.txt"), "dirty source must not ship\n");
  writeFileSync(join(root, "private-token"), "must not ship\n");
  git(
    root,
    "config",
    "remote.private.url",
    "https://credential@do-not-copy.invalid/repo",
  );
  const bin = join(root, "test-bin");
  mkdirSync(bin);
  const builderStub = `#!/usr/bin/env node
const fs=require('node:fs'); const cp=require('node:child_process'); const p=require('node:path');
const args=process.argv.slice(2); const builder=p.basename(process.argv[1]);
fs.appendFileSync(process.env.BUILD_CALLS,JSON.stringify({builder,args})+'\\n');
if(builder==='buildctl' && args.includes('debug')) process.exit(Number(process.env.BUILD_DEBUG_EXIT||0));
const context=builder==='buildctl'?args[args.indexOf('--local')+1].slice('context='.length):args.at(-1); const repo=p.join(context,'repo');
const git=(where,...cmd)=>cp.execFileSync('git',['-C',where,...cmd],{encoding:'utf8'}).trim();
const fork=p.join(repo,'elohim/holochain-conductor');
const times=JSON.parse(fs.readFileSync(p.join(repo,'.compute-source-times.json'),'utf8'));
const native=times.sources.map(entry=>({...entry,mtime:Math.floor(fs.statSync(p.join(repo,entry.path,'lib.rs')).mtimeMs/1000),source:fs.readFileSync(p.join(repo,entry.path,'lib.rs'),'utf8')}));
fs.writeFileSync(process.env.BUILD_OBSERVATION,JSON.stringify({builder,args,times,native,source:fs.readFileSync(p.join(repo,'source.txt'),'utf8'),privateFile:fs.existsSync(p.join(repo,'private-token')),root:git(repo,'rev-parse','HEAD'),rakia:git(p.join(repo,'elohim/rakia'),'rev-parse','HEAD'),remotes:git(repo,'remote'),config:fs.readFileSync(p.join(repo,'.git/config'),'utf8'),shallow:git(repo,'rev-parse','--is-shallow-repository'),sourceMode:fs.statSync(p.join(repo,'source.txt')).mode&0o777,repoMode:fs.statSync(repo).mode&0o777,sourceMtime:Math.floor(fs.statSync(p.join(repo,'source.txt')).mtimeMs/1000),commitTime:Number(git(repo,'show','-s','--format=%ct','HEAD')),fork:git(fork,'rev-parse','HEAD'),forkConfig:fs.readFileSync(p.join(fork,'.git/config'),'utf8'),forkSource:fs.readFileSync(p.join(fork,'crates/hc/Cargo.toml'),'utf8'),forkPrivate:fs.existsSync(p.join(fork,'private-token'))}));
if(builder==='buildctl' && process.env.BUILD_SKIP_METADATA!=='1') fs.writeFileSync(args[args.indexOf('--metadata-file')+1],JSON.stringify({'containerimage.digest':'sha256:'+'b'.repeat(64)}));
process.exit(Number(process.env.BUILD_EXIT||0));
`;
  for (const builder of ["nerdctl", "docker", "buildctl"])
    writeFileSync(join(bin, builder), builderStub, { mode: 0o755 });
  return { root, bin, commit, pin, forkPin, rakiaRemote, forkRemote };
}
function build(f, args, extra = {}) {
  return spawnSync("bash", [script, ...args], {
    cwd: f.root,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${f.bin}:${process.env.PATH}`,
      COMPUTE_WORKER_BUILDER: "nerdctl",
      BUILD_OBSERVATION: join(f.root, "observed.json"),
      BUILD_CALLS: join(f.root, "calls.jsonl"),
      ...extra,
    },
  });
}

test("source build uses exact committed gitlinks and excludes worktree state and credentials", (t) => {
  const f = fixture(t);
  const result = build(f, ["--source", base, "compute-worker:test"]);
  assert.equal(result.status, 0, result.stderr);
  const observation = JSON.parse(
    readFileSync(join(f.root, "observed.json"), "utf8"),
  );
  assert.equal(observation.source, "committed source\n");
  assert.equal(observation.privateFile, false);
  assert.equal(observation.root, f.commit);
  assert.equal(observation.rakia, f.pin);
  assert.equal(observation.fork, f.forkPin);
  assert.equal(observation.forkSource, "committed fork CLI\n");
  assert.equal(observation.forkPrivate, false);
  assert.doesNotMatch(observation.forkConfig, /credential|private/);
  assert.equal(observation.remotes, "");
  assert.equal(observation.shallow, "true");
  assert.equal(observation.sourceMtime, observation.commitTime);
  assert.doesNotMatch(observation.config, /credential|private/);
  assert.deepEqual(observation.args.slice(0, 3), ["-n", "k8s.io", "build"]);
  assert.ok(observation.args.includes(`SOURCE_COMMIT=${f.commit}`));
  assert.ok(observation.args.includes(`CONDUCTOR_COMMIT=${f.forkPin}`));
});

test("explicit Docker and default nerdctl retain local build arguments", (t) => {
  for (const builder of ["docker", ""]) {
    const f = fixture(t);
    const result = build(f, ["--source", base, "compute-worker:test"], {
      COMPUTE_WORKER_BUILDER: builder,
    });
    assert.equal(result.status, 0, result.stderr);
    const observation = JSON.parse(
      readFileSync(join(f.root, "observed.json"), "utf8"),
    );
    const prefix = builder === "docker" ? ["build"] : ["-n", "k8s.io", "build"];
    assert.deepEqual(observation.args.slice(0, prefix.length), prefix);
    assert.deepEqual(observation.args.slice(prefix.length, -5), [
      "--build-arg",
      `WORKER_BASE_IMAGE=${base}`,
      "--build-arg",
      `SOURCE_COMMIT=${f.commit}`,
      "--build-arg",
      `RAKIA_COMMIT=${f.pin}`,
      "--build-arg",
      `CONDUCTOR_COMMIT=${f.forkPin}`,
    ]);
    assert.equal(observation.args.at(-5), "--tag");
    assert.equal(observation.args.at(-4), "compute-worker:test");
    assert.equal(observation.args.at(-3), "--file");
    assert.equal(observation.source, "committed source\n");
    assert(!observation.args.includes("--output"));
    assert(!observation.args.includes("--metadata-file"));
  }
});

test("buildctl publishes the exact sanitized source with explicit platform and digest metadata", (t) => {
  const f = fixture(t);
  const metadata = join(f.root, "publication metadata.json");
  writeFileSync(metadata, "old success must not survive");
  const tag = "registry.example/compute-worker:assembly";
  const result = build(f, ["--source", base, tag], {
    COMPUTE_WORKER_BUILDER: "buildctl",
    COMPUTE_WORKER_METADATA_FILE: metadata,
    BUILDKIT_HOST: "unix:///test/buildkit.sock",
  });
  assert.equal(result.status, 0, result.stderr);
  const observation = JSON.parse(
    readFileSync(join(f.root, "observed.json"), "utf8"),
  );
  const context = observation.args[
    observation.args.indexOf("--local") + 1
  ].slice("context=".length);
  assert.deepEqual(observation.args, [
    "--addr",
    "unix:///test/buildkit.sock",
    "build",
    "--frontend",
    "dockerfile.v0",
    "--progress",
    "plain",
    "--local",
    `context=${context}`,
    "--local",
    `dockerfile=${join(context, "repo/genesis/agentic/compute")}`,
    "--opt",
    "filename=Dockerfile.source",
    "--opt",
    "platform=linux/amd64",
    "--opt",
    `build-arg:WORKER_BASE_IMAGE=${base}`,
    "--opt",
    `build-arg:SOURCE_COMMIT=${f.commit}`,
    "--opt",
    `build-arg:RAKIA_COMMIT=${f.pin}`,
    "--opt",
    `build-arg:CONDUCTOR_COMMIT=${f.forkPin}`,
    "--output",
    `type=image,name=${tag},push=true`,
    "--metadata-file",
    metadata,
  ]);
  assert.equal(observation.root, f.commit);
  assert.equal(observation.rakia, f.pin);
  assert.equal(observation.fork, f.forkPin);
  assert.equal(observation.source, "committed source\n");
  assert.equal(observation.privateFile, false);
  assert.equal(observation.forkPrivate, false);
  assert.doesNotMatch(
    observation.config + observation.forkConfig,
    /credential|private/,
  );
  assert.deepEqual(JSON.parse(readFileSync(metadata, "utf8")), {
    "containerimage.digest": `sha256:${"b".repeat(64)}`,
  });
  assert.equal(statSync(metadata).mode & 0o777, 0o600);
  const calls = readFileSync(join(f.root, "calls.jsonl"), "utf8")
    .trim()
    .split("\n")
    .map(JSON.parse);
  assert.deepEqual(
    calls.map((call) => call.builder),
    ["buildctl", "buildctl"],
  );
  assert.deepEqual(calls[0].args, [
    "--addr",
    "unix:///test/buildkit.sock",
    "debug",
    "workers",
  ]);
});

test("buildctl refuses missing, relative and symlink metadata paths before publishing", (t) => {
  for (const kind of ["missing", "relative", "symlink"]) {
    const f = fixture(t);
    const target = join(f.root, "retained.json");
    writeFileSync(target, "retained");
    const link = join(f.root, "metadata-link.json");
    symlinkSync(target, link);
    const metadata =
      kind === "symlink" ? link : kind === "relative" ? "relative.json" : "";
    const result = build(
      f,
      ["--source", base, "registry.example/worker:test"],
      {
        COMPUTE_WORKER_BUILDER: "buildctl",
        COMPUTE_WORKER_METADATA_FILE: metadata,
      },
    );
    assert.equal(result.status, 2, result.stderr);
    assert.match(result.stderr, /absolute COMPUTE_WORKER_METADATA_FILE/);
    assert.equal(readFileSync(target, "utf8"), "retained");
    assert.throws(() => readFileSync(join(f.root, "calls.jsonl")), {
      code: "ENOENT",
    });
  }
});

test("buildctl cannot inject exporter attributes through the output image", (t) => {
  const f = fixture(t);
  const result = build(f, ["--source", base, "worker:test,push=false"], {
    COMPUTE_WORKER_BUILDER: "buildctl",
    COMPUTE_WORKER_METADATA_FILE: join(f.root, "metadata.json"),
  });
  assert.equal(result.status, 2, result.stderr);
  assert.match(result.stderr, /must not contain commas or whitespace/);
  assert.throws(() => readFileSync(join(f.root, "calls.jsonl")), {
    code: "ENOENT",
  });
});

test("failed BuildKit publication removes stale and partial metadata and preserves failure", (t) => {
  for (const failure of ["BUILD_DEBUG_EXIT", "BUILD_EXIT"]) {
    const f = fixture(t);
    const metadata = join(f.root, "metadata.json");
    writeFileSync(metadata, "old success");
    const result = build(
      f,
      ["--source", base, "registry.example/worker:test"],
      {
        COMPUTE_WORKER_BUILDER: "buildctl",
        COMPUTE_WORKER_METADATA_FILE: metadata,
        [failure]: "37",
      },
    );
    assert.equal(result.status, 37, result.stderr);
    assert.throws(() => readFileSync(metadata), { code: "ENOENT" });
  }
});

test("buildctl success without metadata is refused", (t) => {
  const f = fixture(t);
  const metadata = join(f.root, "metadata.json");
  const result = build(f, ["--source", base, "registry.example/worker:test"], {
    COMPUTE_WORKER_BUILDER: "buildctl",
    COMPUTE_WORKER_METADATA_FILE: metadata,
    BUILD_SKIP_METADATA: "1",
  });
  assert.equal(result.status, 2, result.stderr);
  assert.match(result.stderr, /BuildKit did not write metadata/);
  assert.throws(() => readFileSync(metadata), { code: "ENOENT" });
});

test("buildctl dry run does not publish or modify existing metadata", (t) => {
  const f = fixture(t);
  const metadata = join(f.root, "metadata.json");
  writeFileSync(metadata, "retained");
  const result = build(f, ["--source", base, "registry.example/worker:test"], {
    COMPUTE_WORKER_BUILDER: "buildctl",
    COMPUTE_WORKER_METADATA_FILE: metadata,
    COMPUTE_WORKER_BUILD_DRY_RUN: "1",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /compute worker build: buildctl/);
  assert.equal(readFileSync(metadata, "utf8"), "retained");
  assert.throws(() => readFileSync(join(f.root, "calls.jsonl")), {
    code: "ENOENT",
  });
});

test("source build refuses a pin without the executor before building", (t) => {
  const f = fixture(t, false);
  const result = build(f, ["--source", base, "compute-worker:test"]);
  assert.equal(result.status, 2);
  assert.match(
    result.stderr,
    /Pinned rakia does not contain the compute executor/,
  );
});

test("a later harness commit retains verified native tree times and committed bytes", (t) => {
  const f = fixture(t);
  writeFileSync(join(f.root, "source.txt"), "later harness\n");
  git(f.root, "add", "source.txt");
  const commit = commitAt(f.root, "harness only", initialEpoch + 100);
  writeFileSync(
    join(f.root, nativePaths[0], "lib.rs"),
    "// dirty native source\n",
  );
  const result = build(f, ["--source", base, "compute-worker:test", commit]);
  assert.equal(result.status, 0, result.stderr);
  const observation = JSON.parse(
    readFileSync(join(f.root, "observed.json"), "utf8"),
  );
  assert.equal(observation.times.sourceCommit, commit);
  assert.equal(observation.sourceMtime, initialEpoch + 100);
  assert.deepEqual(
    observation.native.map((entry) => entry.path),
    nativePaths,
  );
  for (const entry of observation.native) {
    assert.equal(
      entry.tree,
      git(f.root, "rev-parse", `${commit}:${entry.path}`),
    );
    assert.equal(entry.lastChangeCommit, f.commit);
    assert.equal(entry.epoch, initialEpoch);
    assert.equal(entry.mtime, initialEpoch);
    assert.equal(entry.source, "// committed native source\n");
  }
});

test("a real native change advances only its source time even after later harness work", (t) => {
  const f = fixture(t);
  writeFileSync(join(f.root, nativePaths[0], "lib.rs"), "// native change\n");
  git(f.root, "add", nativePaths[0]);
  const nativeCommit = commitAt(f.root, "native change", initialEpoch + 200);
  writeFileSync(join(f.root, "source.txt"), "later harness\n");
  git(f.root, "add", "source.txt");
  const commit = commitAt(f.root, "harness only", initialEpoch + 300);
  const result = build(f, ["--source", base, "compute-worker:test", commit]);
  assert.equal(result.status, 0, result.stderr);
  const observation = JSON.parse(
    readFileSync(join(f.root, "observed.json"), "utf8"),
  );
  assert.equal(observation.sourceMtime, initialEpoch + 300);
  assert.equal(observation.native[0].lastChangeCommit, nativeCommit);
  assert.equal(observation.native[0].mtime, initialEpoch + 200);
  assert.equal(observation.native[0].source, "// native change\n");
  assert.equal(observation.native[1].lastChangeCommit, f.commit);
  assert.equal(observation.native[1].mtime, initialEpoch);
});

test("an ambiguous shallow history boundary refuses before the image builder", (t) => {
  const f = fixture(t);
  for (const offset of [100, 200]) {
    writeFileSync(join(f.root, "source.txt"), `harness ${offset}\n`);
    git(f.root, "add", "source.txt");
    commitAt(f.root, "harness only", initialEpoch + offset);
  }
  const shallow = join(f.root, "shallow-checkout");
  git(f.root, "clone", "--depth=2", "-q", pathToFileURL(f.root).href, shallow);
  assert.equal(git(shallow, "rev-parse", "--is-shallow-repository"), "true");
  const result = build({ ...f, root: shallow }, [
    "--source",
    base,
    "compute-worker:test",
  ]);
  assert.equal(result.status, 2, result.stderr);
  assert.match(
    result.stderr,
    /native source time: .*shallow history boundary; deepen the checkout/,
  );
  assert.throws(() => readFileSync(join(shallow, "observed.json")), {
    code: "ENOENT",
  });
});

test("bounded shallow history is sufficient when native changes are inside its boundary", (t) => {
  const f = fixture(t);
  for (const path of nativePaths) {
    writeFileSync(join(f.root, path, "lib.rs"), "// native change\n");
  }
  git(f.root, "add", ...nativePaths);
  const nativeCommit = commitAt(f.root, "native changes", initialEpoch + 100);
  writeFileSync(join(f.root, "source.txt"), "later harness\n");
  git(f.root, "add", "source.txt");
  commitAt(f.root, "harness only", initialEpoch + 200);
  const shallow = join(f.root, "shallow-checkout");
  git(f.root, "clone", "--depth=3", "-q", pathToFileURL(f.root).href, shallow);
  assert.equal(git(shallow, "rev-parse", "--is-shallow-repository"), "true");
  const result = build({ ...f, root: shallow }, [
    "--source",
    base,
    "compute-worker:test",
  ]);
  assert.equal(result.status, 0, result.stderr);
  const observation = JSON.parse(
    readFileSync(join(shallow, "observed.json"), "utf8"),
  );
  for (const entry of observation.native) {
    assert.equal(entry.lastChangeCommit, nativeCommit);
    assert.equal(entry.mtime, initialEpoch + 100);
  }
});

test("uninitialized submodules fetch exact pins using only committed URLs", (t) => {
  const f = fixture(t);
  const fork = join(f.root, "elohim/holochain-conductor");
  git(fork, "add", "crates/hc/Cargo.toml");
  git(fork, "commit", "-qm", "upstream advanced after pin");
  git(fork, "push", "-q", f.forkRemote, "HEAD:refs/heads/next");
  git(f.forkRemote, "symbolic-ref", "HEAD", "refs/heads/next");
  // Match checkout scm when the fork declares update=none, and exercise rakia's
  // same fallback. Neither the dirty manifest nor local URL overrides may win.
  rmSync(join(f.root, "elohim/rakia"), { recursive: true });
  rmSync(join(f.root, "elohim/holochain-conductor"), { recursive: true });
  mkdirSync(join(f.root, "elohim/holochain-conductor"));
  writeFileSync(join(f.root, ".gitmodules"), "dirty manifest must not ship\n");
  for (const path of ["elohim/rakia", "elohim/holochain-conductor"]) {
    git(
      f.root,
      "config",
      `submodule.${path}.url`,
      "https://credential@do-not-fetch.invalid/repo",
    );
  }
  const result = build(f, ["--source", base, "compute-worker:test"]);
  assert.equal(result.status, 0, result.stderr);
  const observation = JSON.parse(
    readFileSync(join(f.root, "observed.json"), "utf8"),
  );
  assert.equal(observation.rakia, f.pin);
  assert.equal(observation.fork, f.forkPin);
  assert.equal(observation.forkSource, "committed fork CLI\n");
  assert.equal(observation.forkPrivate, false);
  assert.equal(observation.remotes, "");
  assert.doesNotMatch(observation.config, /credential|private/);
  assert.doesNotMatch(observation.forkConfig, /credential|private|remote/);
});

test("fetched rakia pin must still contain the executor", (t) => {
  const f = fixture(t, false);
  rmSync(join(f.root, "elohim/rakia"), { recursive: true });
  const result = build(f, ["--source", base, "compute-worker:test"]);
  assert.equal(result.status, 2);
  assert.match(
    result.stderr,
    /Pinned rakia does not contain the compute executor/,
  );
});

test("floating runtime image is refused", (t) => {
  const f = fixture(t);
  const result = build(f, [
    "--source",
    "runtime:latest",
    "compute-worker:test",
  ]);
  assert.equal(result.status, 2);
  assert.match(result.stderr, /pinned by digest/);
});

test("dry run prepares and validates immutable sources without invoking builder", (t) => {
  const f = fixture(t);
  const result = build(f, ["--source", base, "compute-worker:test"], {
    COMPUTE_WORKER_BUILD_DRY_RUN: "1",
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /compute worker build: nerdctl -n k8s.io build/);
});

test("source image RUN bodies parse before a costly native build", () => {
  const dockerfile = fileURLToPath(
    new URL("../../genesis/agentic/compute/Dockerfile.source", import.meta.url),
  );
  const lines = readFileSync(dockerfile, "utf8").split("\n");
  let checked = 0;
  for (let i = 0; i < lines.length; i++) {
    if (!lines[i].startsWith("RUN ")) continue;
    let body = lines[i].slice(4);
    while (body.endsWith("\\")) body = body.slice(0, -1) + " " + lines[++i];
    const result = spawnSync("bash", ["-n"], {
      input: body,
      encoding: "utf8",
      timeout: 5000,
    });
    assert.equal(result.status, 0, result.stderr);
    checked++;
  }
  assert(checked > 0);
});

test("source snapshot is guest-readable even under a private caller umask", (t) => {
  const f = fixture(t);
  const previous = process.umask(0o077);
  let result;
  try {
    result = build(f, ["--source", base, "compute-worker:test"]);
  } finally {
    process.umask(previous);
  }
  assert.equal(result.status, 0, result.stderr);
  const observation = JSON.parse(
    readFileSync(join(f.root, "observed.json"), "utf8"),
  );
  assert.equal(observation.sourceMode & 0o044, 0o044);
  assert.equal(observation.repoMode & 0o055, 0o055);
});
