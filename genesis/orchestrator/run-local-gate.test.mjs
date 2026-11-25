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
import { spawn, spawnSync } from "node:child_process";
import { setTimeout as delay } from "node:timers/promises";
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

function slotFixture(t) {
  const fx = fixture(t);
  const bin = join(fx.dir, "bin");
  const slotRoot = join(fx.dir, "slots");
  mkdirSync(join(fx.root, "genesis/agentic/bin"), { recursive: true });
  writeFileSync(join(fx.root, "genesis/agentic/bin/pool-lib.sh"), `
detect_family() { printf '%s\n' "\${GATE_TEST_FAMILY:-family}"; }
slot_path() {
  local workspace="\${2//\//_}"
  printf '%s/%s__%s__%s\n' "$GATE_TEST_SLOT_ROOT" "$1" "$workspace" "$3"
}
`);
  executable(join(bin, "just"), `#!/usr/bin/env bash
set -euo pipefail
now() { date +%s%N; }
printf 'enter\\t%s\\t%s\\n' "$GATE_TEST_LABEL" "$(now)" >> "$GATE_TEST_EVENTS"
if [[ "\${GATE_TEST_PROBE_LOCK:-0}" == 1 ]]; then
  if flock -n "$CARGO_TARGET_DIR/.gate.lock" -c true; then
    echo LOCK_NOT_HELD
    exit 97
  fi
  echo LOCK_HELD
fi
if [[ -n "\${GATE_TEST_READY:-}" ]]; then
  printf 'ready\\n' > "$GATE_TEST_READY"
  while [[ ! -e "$GATE_TEST_RELEASE" ]]; do sleep 0.02; done
fi
printf 'exit\\t%s\\t%s\\n' "$GATE_TEST_LABEL" "$(now)" >> "$GATE_TEST_EVENTS"
`);
  return { ...fx, bin, slotRoot, events: join(fx.dir, "events.tsv") };
}

function startSlotGate(t, fx, {
  projectName,
  workspace = "workspace-a",
  label = projectName,
  env = {},
} = {}) {
  const child = spawn("bash", [
    SCRIPT, fx.root, projectName, fx.projectDir, "root-just", "gate",
    workspace, "", "dev", "",
  ], {
    env: {
      ...process.env,
      PATH: `${fx.bin}:${BASE_PATH}`,
      RUSTUP_TOOLCHAIN: "",
      GATE_TEST_SLOT_ROOT: fx.slotRoot,
      GATE_TEST_EVENTS: fx.events,
      GATE_TEST_LABEL: label,
      ...env,
    },
    stdio: ["ignore", "pipe", "pipe"],
  });
  let stdout = "";
  let stderr = "";
  child.stdout.setEncoding("utf8");
  child.stderr.setEncoding("utf8");
  child.stdout.on("data", chunk => { stdout += chunk; });
  child.stderr.on("data", chunk => { stderr += chunk; });
  const closed = new Promise(resolve => child.once("close", (status, signal) => {
    resolve({ status, signal, stdout, stderr });
  }));
  t.after(() => {
    if (child.exitCode === null && child.signalCode === null) child.kill("SIGKILL");
  });
  return { child, closed, stdout: () => stdout, stderr: () => stderr };
}

async function waitUntil(predicate, message, timeoutMs = 3000) {
  const deadline = Date.now() + timeoutMs;
  while (!predicate()) {
    if (Date.now() >= deadline) assert.fail(message);
    await delay(10);
  }
}

function readEvents(path) {
  return readFileSync(path, "utf8").trim().split("\n").filter(Boolean).map(line => {
    const [kind, label, at] = line.split("\t");
    return { kind, label, at: BigInt(at) };
  });
}

test("two gates on one cargo slot never overlap and the waiter names the holder", async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "first.ready");
  const release = join(fx.dir, "first.release");
  const first = startSlotGate(t, fx, {
    projectName: "first",
    env: { GATE_LIVE_RUN: "run-first", GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "first gate never entered its recipe");

  const second = startSlotGate(t, fx, { projectName: "second" });
  await waitUntil(() => second.stdout().includes("cargo slot busy"), "second gate never reported the holder");
  assert.match(second.stdout(), new RegExp(
    `^  \\[second\\] cargo slot busy; holder: project=first worktree=${fx.root.replaceAll("/", "\\/")} `
      + "pid=[0-9]+ run=run-first started=[^ ]+; waiting up to 3600s$", "m",
  ));
  writeFileSync(release, "release\n");

  const [one, two] = await Promise.all([first.closed, second.closed]);
  assert.equal(one.status, 0, one.stderr);
  assert.equal(two.status, 0, two.stderr);
  const events = readEvents(fx.events);
  assert.deepEqual(events.map(({ kind, label }) => [kind, label]), [
    ["enter", "first"], ["exit", "first"], ["enter", "second"], ["exit", "second"],
  ]);
  assert.ok(events[1].at <= events[2].at, "the second recipe entered before the first exited");
});

test("killing a cargo slot holder releases the lock immediately", async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "holder.ready");
  const release = join(fx.dir, "never.release");
  const holder = startSlotGate(t, fx, {
    projectName: "holder",
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "holder never entered its recipe");
  const waiter = startSlotGate(t, fx, { projectName: "waiter" });
  await waitUntil(() => waiter.stdout().includes("cargo slot busy"), "waiter never observed contention");

  const killedAt = Date.now();
  holder.child.kill("SIGKILL");
  const waited = await Promise.race([
    waiter.closed,
    delay(1500).then(() => ({ timeout: true })),
  ]);
  assert.equal(waited.timeout, undefined, "waiter did not proceed when the killed holder released the lock");
  assert.equal(waited.status, 0, waited.stderr);
  assert.ok(Date.now() - killedAt < 1500);
  const killed = await holder.closed;
  assert.equal(killed.signal, "SIGKILL");
});

test("different workspace slots never wait on each other", async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "workspace-a.ready");
  const release = join(fx.dir, "workspace-a.release");
  const first = startSlotGate(t, fx, {
    projectName: "workspace-a-holder",
    workspace: "workspace-a",
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "workspace-a holder never entered its recipe");

  const second = startSlotGate(t, fx, { projectName: "workspace-b-gate", workspace: "workspace-b" });
  const result = await Promise.race([
    second.closed,
    delay(1500).then(() => ({ timeout: true })),
  ]);
  assert.equal(result.timeout, undefined, "a different workspace waited on the occupied slot");
  assert.equal(result.status, 0, result.stderr);
  assert.doesNotMatch(result.stdout, /cargo slot busy/);
  writeFileSync(release, "release\n");
  assert.equal((await first.closed).status, 0);
});

test("a cargo slot timeout refuses with the dedicated did-not-run status", async (t) => {
  const fx = slotFixture(t);
  const ready = join(fx.dir, "timeout-holder.ready");
  const release = join(fx.dir, "timeout-holder.release");
  const holder = startSlotGate(t, fx, {
    projectName: "timeout-holder",
    env: { GATE_TEST_READY: ready, GATE_TEST_RELEASE: release },
  });
  await waitUntil(() => {
    try { return readFileSync(ready, "utf8").trim() === "ready"; } catch { return false; }
  }, "timeout holder never entered its recipe");

  const waiter = startSlotGate(t, fx, {
    projectName: "timeout-waiter",
    env: { GATE_SLOT_LOCK_WAIT_S: "0.1" },
  });
  const result = await waiter.closed;
  assert.equal(result.status, 73, result.stderr);
  assert.match(result.stdout, /^  \[timeout-waiter\] cargo slot busy; holder: project=timeout-holder .*; waiting up to 0\.1s$/m);
  assert.match(result.stderr, /^gate timeout-waiter: REFUSED cargo slot stayed busy for 0\.1s; holder: project=timeout-holder .*$/m);
  writeFileSync(release, "release\n");
  assert.equal((await holder.closed).status, 0);
});

test("the cargo slot lock file descriptor reaches the recipe", async (t) => {
  const fx = slotFixture(t);
  const gate = startSlotGate(t, fx, {
    projectName: "lock-probe",
    env: { GATE_TEST_PROBE_LOCK: "1" },
  });
  const result = await gate.closed;
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^LOCK_HELD$/m);
  assert.doesNotMatch(result.stdout, /LOCK_NOT_HELD/);
});

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

// E12 (b): cc reads the C flags for one target from a triple-suffixed variable
// (CFLAGS_x86_64_unknown_linux_gnu), so a declared cargo env key may carry lowercase after its
// leading capital. Anything else still refuses before the recipe runs.
test("a_triple_suffixed_c_flags_key_is_exported_to_the_recipe", (t) => {
  const fx = fixture(t, '@echo "C=$CFLAGS_x86_64_unknown_linux_gnu"\n    @echo RAN');

  const result = run(fx, {
    env: { GATE_CARGO_ENV: JSON.stringify({ CFLAGS_x86_64_unknown_linux_gnu: "-march=x86-64" }) },
  });

  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /^C=-march=x86-64$/m);
  assert.match(result.stdout, /RAN/);
});

test("a_malformed_cargo_env_key_still_refuses", (t) => {
  for (const key of ["lowercase_first", "1LEADING_DIGIT", "HAS-DASH", "HAS SPACE", "_LEADING"]) {
    const fx = fixture(t);
    const result = run(fx, { env: { GATE_CARGO_ENV: JSON.stringify({ [key]: "1" }) } });
    assert.equal(result.status, 2, `${key}: ${result.stderr}`);
    assert.match(result.stderr, /refusing malformed cargo\.env key/);
    assert.doesNotMatch(result.stdout, /RAN/);
  }
});
