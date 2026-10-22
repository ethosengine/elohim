import test from "node:test";
import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import {
  mkdtempSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  chmodSync,
  existsSync,
  rmSync,
  statSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
const bootstrap = join(here, "worker-start.sh");
const wrapper = join(here, "mesh-just.sh");
const rootOnly =
  process.getuid?.() !== 0
    ? "bootstrap needs root to grant guest report ownership"
    : false;

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "compute-bootstrap-"));
  chmodSync(root, 0o755);
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const image = join(root, "image"),
    state = join(root, "state"),
    source = join(image, "repo");
  mkdirSync(join(image, "mesh-bin"), { recursive: true });
  mkdirSync(join(source, "app/elohim-app/scripts"), { recursive: true });
  mkdirSync(join(source, "genesis/a2o"), { recursive: true });
  for (const name of [
    "elohim-storage",
    "doorway",
    "holochain",
    "hc",
    "iroh-relay",
    "relay-addr-beacon",
  ]) {
    writeFileSync(join(image, "mesh-bin", name), "#!/bin/sh\nexit 0\n", {
      mode: 0o755,
    });
  }
  writeFileSync(join(image, "elohim.happ"), "fixture-only");
  writeFileSync(
    join(image, "grant-bootstrap.mjs"),
    `if (process.env.FIXTURE_GRANT_REFUSE === '1') process.exit(23);\n`,
  );
  writeFileSync(join(source, ".gitignore"), "genesis/a2o/reports/\n");
  writeFileSync(join(source, "justfile"), "fixture:\n    true\n");
  writeFileSync(
    join(source, "app/elohim-app/scripts/hc-mesh.sh"),
    `#!/bin/bash
set -eu
test ! -e /proc/$$/fd/9 || { echo 'worker lock leaked to mesh'; exit 99; }
test "$MONGOD_BIN" = /usr/local/bin/mongod
test "$MONGO_PORT" = 37017
mkdir -p "$MESH_DIR"
printf '%s' "${"$"}{MESH_CONTINUE_INITIALIZATION_LOG_SHA256:-}" > "$MESH_DIR/continuation-digest"
printf '%s %s %s %s %s %s %s %s\\n' "$1" "$MESH_STORAGE_BASE_PORT" "$MESH_CONDUCTOR_BASE_PORT" "$MESH_P2P_BASE_PORT" "$MESH_METRICS_BASE_PORT" "$MESH_RESET" "$MESH_PORTAL" "${"$"}{PROVIDER_TEST_TOKEN:-scrubbed}" >> "$MESH_DIR/events"
if [ "$1" = start ]; then
  for peer in matthew jessica james; do
    mkdir -p "$MESH_DIR/$peer"
    [ -e "$MESH_DIR/$peer/identity.key" ] || printf 'preserved-private-identity' > "$MESH_DIR/$peer/identity.key"
    printf 'fixture configuration' > "$MESH_DIR/$peer/runtime-config.toml"
  done
fi
if [ "$1" = wait ] && [ -e "$COMPUTE_STATE_ROOT/fail-wait" ]; then exit 19; fi
`,
  );
  writeFileSync(
    join(image, "worker.mjs"),
    `import fs from 'node:fs';
fs.appendFileSync(process.env.COMPUTE_STATE_ROOT + '/worker-starts', process.cwd() + '\\n');
console.log('fixture worker invoked');
`,
  );
  const just = join(image, "fixture-just");
  writeFileSync(
    just,
    `#!/usr/bin/env node
console.log(JSON.stringify({args:process.argv.slice(2), mesh:process.env.MESH_DIR,
storage:process.env.MESH_STORAGE_BASE_PORT, conductor:process.env.MESH_CONDUCTOR_BASE_PORT,
alpha:process.env.E2E_DOORWAY_ALPHA,beta:process.env.E2E_DOORWAY_B,
noPrologue:process.env.MESH_ALLOW_NO_PROLOGUE,fenced:process.env.BERTH_FENCED}));
`,
    { mode: 0o755 },
  );
  const git = (...args) =>
    execFileSync("git", ["-C", source, ...args], { encoding: "utf8" }).trim();
  git("init", "-q");
  git("add", "justfile", ".gitignore", "app/elohim-app/scripts/hc-mesh.sh");
  git(
    "-c",
    "user.name=Fixture",
    "-c",
    "user.email=fixture@example.invalid",
    "commit",
    "-qm",
    "fixture",
  );
  const conductor = git("rev-parse", "HEAD");
  mkdirSync(join(source, "elohim/holochain-conductor"), { recursive: true });
  git(
    "update-index",
    "--add",
    "--cacheinfo",
    `160000,${conductor},elohim/holochain-conductor`,
  );
  git(
    "-c",
    "user.name=Fixture",
    "-c",
    "user.email=fixture@example.invalid",
    "commit",
    "-qm",
    "pin",
  );
  const hashes = ["holochain", "hc"].map((name) => {
    const line = execFileSync("sha256sum", [join(image, "mesh-bin", name)], {
      encoding: "utf8",
    });
    return `${line.split(" ")[0]}  /out/${name}`;
  });
  writeFileSync(
    join(image, "mesh-bin/FORK_BUILD_RECEIPT.txt"),
    `fork commit: ${conductor}\n${hashes.join("\n")}\n`,
  );
  const commit = git("rev-parse", "HEAD");
  const env = {
    ...process.env,
    COMPUTE_PRIVATE_MESH: "1",
    COMPUTE_IMAGE_ROOT: image,
    COMPUTE_STATE_ROOT: state,
    COMPUTE_JUST_BIN: just,
    PROVIDER_TEST_TOKEN: "must-not-reach-mesh",
  };
  const run = () => spawnSync("bash", [bootstrap], { env, encoding: "utf8" });
  return {
    root,
    image,
    state,
    source,
    git,
    commit,
    env,
    run,
    repo: join(state, "repos", commit),
  };
}

test(
  "bootstrap resumes identities, isolates ports, scrubs credentials and preserves immutable checkout",
  { skip: rootOnly },
  (t) => {
    const f = fixture(t);
    f.env.COMPUTE_CONTINUE_INITIALIZATION_LOG_SHA256 = "d".repeat(64);
    let run = f.run();
    assert.equal(run.status, 0, run.stderr);
    assert.equal(
      readFileSync(join(f.state, "mesh/continuation-digest"), "utf8"),
      "d".repeat(64),
    );
    const key = join(f.state, "mesh/matthew/identity.key");
    assert.equal(statSync(key).mode & 0o777, 0o600);
    assert.equal(statSync(join(f.repo, "genesis/a2o/reports")).uid, 65534);
    assert.equal(
      statSync(join(f.state, "mesh/matthew/runtime-config.toml")).mode & 0o777,
      0o606,
    );
    assert.match(
      readFileSync(join(f.state, "mesh/events"), "utf8"),
      /start 18090 14444 19701 19464 0 0 scrubbed/,
    );
    writeFileSync(key, "identity-kept-across-restart");
    run = f.run();
    assert.equal(run.status, 0, run.stderr);
    assert.equal(readFileSync(key, "utf8"), "identity-kept-across-restart");
    writeFileSync(join(f.repo, "justfile"), "modified source\n");
    run = f.run();
    assert.equal(run.status, 2);
    assert.match(run.stderr, /modified tracked source/);
  },
);

test(
  "new image commit gets another checkout without replacing earlier task source",
  { skip: rootOnly },
  (t) => {
    const f = fixture(t);
    assert.equal(f.run().status, 0);
    writeFileSync(join(f.source, "justfile"), "fixture:\n    echo next\n");
    f.git("add", "justfile");
    f.git(
      "-c",
      "user.name=Fixture",
      "-c",
      "user.email=fixture@example.invalid",
      "commit",
      "-qm",
      "next",
    );
    const next = f.git("rev-parse", "HEAD");
    const run = f.run();
    assert.equal(run.status, 0, run.stderr);
    assert.equal(
      readFileSync(join(f.repo, "justfile"), "utf8"),
      "fixture:\n    true\n",
    );
    assert.equal(
      readFileSync(
        join(f.state, "repos", next, ".compute-source-commit"),
        "utf8",
      ).trim(),
      next,
    );
  },
);

test(
  "readiness failure and reset request cannot start the worker",
  { skip: rootOnly },
  (t) => {
    const f = fixture(t);
    mkdirSync(f.state);
    writeFileSync(join(f.state, "fail-wait"), "fixture");
    const failed = f.run();
    assert.equal(failed.status, 19);
    assert.equal(existsSync(join(f.state, "worker-starts")), false);
    f.env.MESH_RESET = "1";
    assert.match(f.run().stderr, /MESH_RESET is forbidden/);
  },
);

test(
  "grant bootstrap refusal prevents both private and legacy worker startup",
  { skip: rootOnly },
  (t) => {
    const f = fixture(t);
    f.env.FIXTURE_GRANT_REFUSE = "1";
    assert.equal(f.run().status, 23);
    assert.equal(existsSync(join(f.state, "worker-starts")), false);
    f.env.COMPUTE_PRIVATE_MESH = "0";
    assert.equal(f.run().status, 23);
    assert.equal(existsSync(join(f.state, "worker-starts")), false);
  },
);

test(
  "wrapper exposes only the no-cast health feature and restores the same owned topology",
  { skip: rootOnly },
  (t) => {
    const f = fixture(t);
    const started = f.run();
    assert.equal(started.status, 0, started.stderr);
    const invoke = (args) =>
      spawnSync("bash", [wrapper, ...args], {
        env: f.env,
        cwd: f.repo,
        encoding: "utf8",
      });
    const run = invoke([
      "test",
      "mesh",
      "features/dataplane/peer-mesh.feature",
    ]);
    assert.equal(run.status, 0, run.stderr);
    const seen = JSON.parse(run.stdout);
    assert.equal(seen.storage, "18090");
    assert.equal(seen.conductor, "14444");
    assert.equal(seen.alpha, "http://localhost:18888");
    assert.equal(seen.beta, "http://localhost:18889");
    assert.equal(seen.noPrologue, "1");
    assert.equal(seen.fenced, "1");
    assert.match(
      run.stderr,
      /sequential worker admission and executor deadline/,
    );
    for (const args of [
      ["mesh", "stop"],
      ["test", "mesh", "features/auth/login.feature"],
      ["test", "mesh", "@e2e"],
    ])
      assert.equal(invoke(args).status, 2);
  },
);

test(
  "bootstrap and resume work with only SETUID SETGID CHOWN capabilities",
  { skip: rootOnly },
  (t) => {
    const f = fixture(t);
    const run = () =>
      spawnSync(
        "setpriv",
        ["--bounding-set=-all,+setuid,+setgid,+chown", "bash", bootstrap],
        { env: f.env, encoding: "utf8" },
      );
    let result = run();
    assert.equal(result.status, 0, result.stderr);
    const reports = join(f.repo, "genesis/a2o/reports");
    const guest = spawnSync(
      "setpriv",
      [
        "--reuid=65534",
        "--regid=65534",
        "--clear-groups",
        "bash",
        "-c",
        'umask 077; mkdir "$1/private-output"; printf actual > "$1/private-output/report"',
        "bash",
        reports,
      ],
      { encoding: "utf8" },
    );
    assert.equal(guest.status, 0, guest.stderr);
    result = run();
    assert.equal(result.status, 0, result.stderr);
    assert.equal(
      readFileSync(join(reports, "private-output/report"), "utf8"),
      "actual",
    );
  },
);

test("sanitized private mesh PATH retains system network diagnostics", (t) => {
  if (!existsSync("/usr/sbin/ss") && !existsSync("/sbin/ss")) {
    t.skip(
      "host has no sbin ss; final image smoke checks the pinned runtime base",
    );
    return;
  }
  const result = execFileSync(
    "bash",
    [
      "-c",
      'source "$1"; command -v ss; ss --version',
      "fixture",
      join(here, "mesh-env.sh"),
    ],
    {
      encoding: "utf8",
      env: { PATH: "/usr/bin:/bin" },
      timeout: 10000,
    },
  );
  assert.match(result, /ss utility/);
});
