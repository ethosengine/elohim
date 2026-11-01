import { test } from "node:test";
import assert from "node:assert/strict";
import { spawn, spawnSync } from "node:child_process";
import {
  chmodSync,
  cpSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const HERE = dirname(fileURLToPath(import.meta.url));
const ROOT = dirname(HERE);

function cleanEnv(home) {
  const env = { ...process.env };
  delete env.NODE_TEST_CONTEXT;
  for (const key of [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_PREFIX",
    "GIT_COMMON_DIR",
  ]) {
    delete env[key];
  }
  return {
    ...env,
    HOME: home,
    GIT_CONFIG_NOSYSTEM: "1",
    GIT_CONFIG_GLOBAL: "/dev/null",
  };
}

function fixture(t) {
  const dir = mkdtempSync(join(tmpdir(), "commit-msg-"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  const env = cleanEnv(dir);
  const husky = join(dir, ".husky");
  const shims = join(husky, "_");
  const orchestrator = join(dir, "genesis/orchestrator");
  mkdirSync(shims, { recursive: true });
  mkdirSync(orchestrator, { recursive: true });
  cpSync(join(HERE, "_/h"), join(shims, "h"));
  cpSync(join(HERE, "_/commit-msg"), join(shims, "commit-msg"));
  cpSync(join(HERE, "commit-msg"), join(husky, "commit-msg"));
  cpSync(
    join(ROOT, "genesis/orchestrator/gate-change-id.mjs"),
    join(orchestrator, "gate-change-id.mjs"),
  );
  for (const dependency of [
    "commit-tag-parser.mjs",
    "pipeline-registry.mjs",
    "manifest-utils.mjs",
  ]) {
    cpSync(
      join(ROOT, "genesis/orchestrator", dependency),
      join(orchestrator, dependency),
    );
  }
  chmodSync(join(shims, "commit-msg"), 0o755);
  chmodSync(join(husky, "commit-msg"), 0o755);

  const run = (...args) =>
    spawnSync("git", args, {
      cwd: dir,
      env,
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    });
  const git = (...args) => {
    const result = run(...args);
    assert.equal(result.status, 0, `git ${args.join(" ")}: ${result.stderr}`);
    return result;
  };
  const runHook = (args, extraEnv = env) =>
    new Promise((resolve, reject) => {
      const child = spawn("git", args, {
        cwd: dir,
        env: extraEnv,
        stdio: ["ignore", "pipe", "pipe"],
      });
      let stdout = "";
      let stderr = "";
      child.stdout.setEncoding("utf8").on("data", (chunk) => (stdout += chunk));
      child.stderr.setEncoding("utf8").on("data", (chunk) => (stderr += chunk));
      child.on("error", reject);
      child.on("close", (status) => resolve({ status, stdout, stderr }));
    });
  const hookGit = async (...args) => {
    const result = await runHook(args);
    assert.equal(result.status, 0, `git ${args.join(" ")}: ${result.stderr}`);
    return result;
  };
  git("init", "-q", "-b", "main");
  git("config", "user.name", "Commit Hook Test");
  git("config", "user.email", "commit-hook@example.test");
  git("config", "core.hooksPath", shims);
  return { dir, env, git, hookGit, runHook, orchestrator };
}

async function addCommit(fx, file, contents, message) {
  writeFileSync(join(fx.dir, file), contents);
  fx.git("add", file);
  return fx.hookGit("commit", "-m", message);
}

function message(fx) {
  return fx.git("log", "-1", "--format=%B").stdout;
}

function changeIds(bytes) {
  return bytes.match(/^Change-Id: (I[0-9a-f]{40})$/gm) || [];
}

function changeId(bytes) {
  return changeIds(bytes)[0]?.slice("Change-Id: ".length);
}

test("commit_mints_one_change_id", async (t) => {
  const fx = fixture(t);
  const committed = await addCommit(fx, "one.txt", "one\n", "one");
  const committedMessage = message(fx);
  assert.equal(
    changeIds(committedMessage).length,
    1,
    `message=${JSON.stringify(committedMessage)} stderr=${JSON.stringify(committed.stderr)}`,
  );
});

test("amend_keeps_the_trailer", async (t) => {
  const fx = fixture(t);
  await addCommit(fx, "one.txt", "one\n", "one");
  const before = changeId(message(fx));
  await fx.hookGit("commit", "--amend", "--no-edit");
  const after = message(fx);
  assert.equal(changeId(after), before);
  assert.equal(changeIds(after).length, 1);
});

test("rebase_keeps_one_trailer", async (t) => {
  const fx = fixture(t);
  await addCommit(fx, "root.txt", "root\n", "root");
  fx.git("branch", "topic");
  await addCommit(fx, "main.txt", "main\n", "main");
  fx.git("checkout", "-q", "topic");
  await addCommit(fx, "topic.txt", "topic\n", "topic");
  const before = changeId(message(fx));
  await fx.hookGit("rebase", "main");
  const after = message(fx);
  assert.equal(changeId(after), before);
  assert.equal(changeIds(after).length, 1);
});

test("an_existing_trailer_is_untouched", async (t) => {
  const fx = fixture(t);
  const existing = `I${"1".repeat(40)}`;
  writeFileSync(join(fx.dir, "one.txt"), "one\n");
  fx.git("add", "one.txt");
  await fx.hookGit("commit", "-m", "one", "-m", `Change-Id: ${existing}`);
  const committed = message(fx);
  assert.equal(changeId(committed), existing);
  assert.equal(changeIds(committed).length, 1);
});

test("a_missing_stamp_module_commits_and_prints_no_chain_position", async (t) => {
  const fx = fixture(t);
  rmSync(join(fx.orchestrator, "gate-change-id.mjs"));
  writeFileSync(join(fx.dir, "one.txt"), "one\n");
  fx.git("add", "one.txt");
  const committed = await fx.runHook(["commit", "-m", "one"]);
  assert.equal(committed.status, 0, committed.stderr);
  assert.match(committed.stderr, /this commit has no chain position/);
  assert.equal(changeIds(message(fx)).length, 0);
});

test("husky_0_skips", async (t) => {
  const fx = fixture(t);
  writeFileSync(join(fx.dir, "one.txt"), "one\n");
  fx.git("add", "one.txt");
  const skipped = await fx.runHook(["commit", "-m", "one"], {
    ...fx.env,
    HUSKY: "0",
  });
  assert.equal(skipped.status, 0, skipped.stderr);
  assert.equal(changeIds(message(fx)).length, 0);
});
