import assert from "node:assert/strict";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

const script = fileURLToPath(
  new URL("./build-compute-worker.sh", import.meta.url),
);
const base = `registry.example/runtime@sha256:${"a".repeat(64)}`;
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
  git(
    root,
    "add",
    ".gitmodules",
    "source.txt",
    "genesis/agentic/compute/Dockerfile.source",
  );
  git(
    root,
    "update-index",
    "--add",
    "--cacheinfo",
    `160000,${pin},elohim/rakia`,
  );
  git(root, "commit", "-qm", "fixture root");
  const commit = git(root, "rev-parse", "HEAD");
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
  writeFileSync(join(bin, "buildctl"), "#!/bin/sh\nexit 0\n", { mode: 0o755 });
  writeFileSync(
    join(bin, "nerdctl"),
    `#!/usr/bin/env node
const fs=require('node:fs'); const cp=require('node:child_process'); const p=require('node:path');
const args=process.argv.slice(2); const context=args.at(-1); const repo=p.join(context,'repo');
const git=(where,...cmd)=>cp.execFileSync('git',['-C',where,...cmd],{encoding:'utf8'}).trim();
const fork=p.join(repo,'elohim/holochain-conductor');
fs.writeFileSync(process.env.BUILD_OBSERVATION,JSON.stringify({args,source:fs.readFileSync(p.join(repo,'source.txt'),'utf8'),privateFile:fs.existsSync(p.join(repo,'private-token')),root:git(repo,'rev-parse','HEAD'),rakia:git(p.join(repo,'elohim/rakia'),'rev-parse','HEAD'),remotes:git(repo,'remote'),config:fs.readFileSync(p.join(repo,'.git/config'),'utf8'),shallow:git(repo,'rev-parse','--is-shallow-repository'),sourceMtime:Math.floor(fs.statSync(p.join(repo,'source.txt')).mtimeMs/1000),commitTime:Number(git(repo,'show','-s','--format=%ct','HEAD')),fork:git(fork,'rev-parse','HEAD'),forkConfig:fs.readFileSync(p.join(fork,'.git/config'),'utf8'),forkSource:fs.readFileSync(p.join(fork,'crates/hc/Cargo.toml'),'utf8'),forkPrivate:fs.existsSync(p.join(fork,'private-token'))}));
`,
    { mode: 0o755 },
  );
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

test("source build refuses a pin without the executor before building", (t) => {
  const f = fixture(t, false);
  const result = build(f, ["--source", base, "compute-worker:test"]);
  assert.equal(result.status, 2);
  assert.match(
    result.stderr,
    /Pinned rakia does not contain the compute executor/,
  );
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
