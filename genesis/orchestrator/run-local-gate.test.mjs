import assert from "node:assert/strict";
import {
  chmodSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { spawnSync } from "node:child_process";
import test from "node:test";

const SCRIPT = join(import.meta.dirname, "run-local-gate.sh");
const BASE_PATH = "/usr/local/bin:/usr/bin:/bin";

function executable(path, body = "#!/bin/sh\nexit 0\n") {
  mkdirSync(join(path, ".."), { recursive: true });
  writeFileSync(path, body);
  chmodSync(path, 0o755);
  return path;
}

function fixture(
  t,
  recipe = "@git rev-parse --show-toplevel 2>&1 || true\n    @echo RAN",
) {
  const dir = mkdtempSync(join(tmpdir(), "run-local-gate-"));
  const root = join(dir, "root");
  const projectDir = "project";
  mkdirSync(join(root, projectDir), { recursive: true });
  writeFileSync(join(root, "justfile"), `gate:\n    ${recipe}\n`);
  const ark = executable(join(dir, "ark"));
  t.after(() => rmSync(dir, { recursive: true, force: true }));
  return { dir, root, projectDir, ark };
}

function run(
  fx,
  {
    projectName = "fixture",
    projectDir = fx.projectDir,
    script = SCRIPT,
    env = {},
  } = {},
) {
  return spawnSync(
    "bash",
    [script, fx.root, projectName, projectDir, "root-just", "gate"],
    {
      encoding: "utf8",
      env: {
        ...process.env,
        PATH: BASE_PATH,
        RUSTUP_TOOLCHAIN: "",
        ARK_BIN: fx.ark,
        ...env,
      },
    },
  );
}

function git(cwd, ...args) {
  const result = spawnSync("git", args, {
    cwd,
    encoding: "utf8",
    env: { ...process.env, PATH: BASE_PATH },
  });
  assert.equal(result.status, 0, result.stderr);
  return result;
}

test("a_callers_ceiling_is_appended_and_honoured", (t) => {
  const fx = fixture(t);
  git(fx.dir, "init", "--quiet");

  const result = run(fx, { env: { GIT_CEILING_DIRECTORIES: fx.dir } });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /not a git repository/);
  assert.doesNotMatch(result.stdout, new RegExp(`^${fx.dir}$`, "m"));
  assert.match(result.stdout, /RAN/);
});

test("the_ceiling_starts_with_tmp_and_var_tmp", (t) => {
  const fx = fixture(t, '@echo "$GIT_CEILING_DIRECTORIES"\n    @echo RAN');

  const result = run(fx, { env: { GIT_CEILING_DIRECTORIES: fx.dir } });

  assert.equal(result.status, 0, result.stderr);
  assert.match(
    result.stdout,
    new RegExp(`/tmp:/var/tmp:${fx.dir.replaceAll("/", "\\/")}`),
  );
  assert.match(result.stdout, /RAN/);
});

test("a_tempdir_repo_still_finds_its_own_git", (t) => {
  const fx = fixture(t);
  git(fx.root, "init", "--quiet");

  const result = run(fx);

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, new RegExp(`^${fx.root}$`, "m"));
  assert.match(result.stdout, /RAN/);
});
