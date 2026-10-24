import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import {
  chmodSync,
  existsSync,
  mkdirSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  symlinkSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { test } from "node:test";

const script =
  process.env.EPR_BOOTSTRAP_TEST_SCRIPT ||
  resolve("genesis/agentic/native-epr-bootstrap.mjs");
function fixture(t, mode = "xdg") {
  const base = mkdtempSync(
    join(process.env.EPR_TRUST_TEST_TMPDIR || tmpdir(), "native-trust-"),
  );
  t.after(() => rmSync(base, { recursive: true, force: true }));
  const root = join(base, "checkout");
  const alias = join(base, "alias");
  const bin = join(base, "bin");
  const calls = join(base, "calls");
  mkdirSync(join(root, "elohim/eprfs"), { recursive: true });
  mkdirSync(join(root, ".git"));
  mkdirSync(bin);
  symlinkSync(root, alias);
  writeFileSync(
    join(root, "elohim/eprfs/build-manifest.json"),
    JSON.stringify({
      gate: {
        projects: {
          eprfs: {
            dir: "elohim/eprfs",
            run: { cargo: { rustflags: "", targetDir: join(base, "target") } },
          },
        },
      },
    }),
  );
  for (const name of ["git", "cargo"]) {
    const path = join(bin, name);
    writeFileSync(
      path,
      `#!${process.execPath}\n` +
        `
const fs = require('node:fs');
const args = process.argv.slice(2);
const config = Array.from({length: Number(process.env.GIT_CONFIG_COUNT || 0)}, (_, i) => [process.env['GIT_CONFIG_KEY_' + i], process.env['GIT_CONFIG_VALUE_' + i]]);
fs.appendFileSync(${JSON.stringify(calls)}, JSON.stringify({name:${JSON.stringify(name)}, args, config}) + '\\n');
if (${JSON.stringify(name)} === 'cargo') { console.error('fixture cargo reached'); process.exit(42); }
if (!args.includes('safe.directory=' + ${JSON.stringify(root)})) { console.error('dubious ownership'); process.exit(128); }
console.log(args.includes('--path-format=absolute') ? '.git' : args.includes('status') ? '' : 'a'.repeat(40));
`,
    );
    chmodSync(path, 0o755);
  }
  const env = {
    ...process.env,
    PATH: `${bin}:${dirname(process.execPath)}`,
    HOME: join(base, "home"),
    XDG_CONFIG_HOME: join(base, "xdg"),
  };
  delete env.ELOHIM_DEVICE_KEY_FILE;
  let trust = join(env.XDG_CONFIG_HOME, "elohim/device/trusted-checkouts");
  if (mode === "explicit") {
    env.ELOHIM_DEVICE_KEY_FILE = join(base, "device/key");
    trust = join(base, "device/trusted-checkouts");
  } else if (mode === "home") {
    env.XDG_CONFIG_HOME = "relative-ignored";
    trust = join(env.HOME, ".config/elohim/device/trusted-checkouts");
  }
  mkdirSync(dirname(trust), { recursive: true });
  return {
    root,
    base,
    trust,
    calls,
    run: () =>
      spawnSync(process.execPath, [script, alias, "--worker"], {
        env,
        encoding: "utf8",
      }),
  };
}
for (const mode of ["xdg", "explicit", "home"]) {
  test(`bootstrap honors exact canonical device trust before every git call (${mode})`, (t) => {
    const f = fixture(t, mode);
    writeFileSync(f.trust, `${f.root}\n`);
    const r = f.run();
    assert.match(r.stderr, /fixture cargo reached/);
    const calls = readFileSync(f.calls, "utf8")
      .trim()
      .split("\n")
      .map(JSON.parse);
    for (const call of calls.filter((c) => c.name === "git"))
      assert.ok(call.args.includes(`safe.directory=${f.root}`));
    assert.ok(
      calls
        .find((c) => c.name === "cargo")
        .config.some(
          ([key, value]) => key === "safe.directory" && value === f.root,
        ),
    );
    assert.ok(
      calls.every(
        (c) =>
          !c.args.includes("safe.directory=*") &&
          !c.config.some(([, v]) => v === "*"),
      ),
    );
  });
}
test("bootstrap untrusted checkout prints both native and pre-native cures", (t) => {
  const f = fixture(t);
  writeFileSync(f.trust, `${f.base}\n*\n`);
  const r = f.run();
  assert.match(r.stderr, /epr device trust/);
  assert.match(r.stderr, /git config --global --add safe.directory/);
  const calls = readFileSync(f.calls, "utf8")
    .trim()
    .split("\n")
    .map(JSON.parse);
  assert.ok(
    calls.every((c) => !c.args.some((a) => a.startsWith("safe.directory="))),
  );
});

test("bootstrap unreadable declaration grants no trust and prints the cures", (t) => {
  const f = fixture(t);
  mkdirSync(f.trust);
  const r = f.run();
  assert.match(r.stderr, /epr device trust/);
  assert.match(r.stderr, /git config --global --add safe.directory/);
  const calls = readFileSync(f.calls, "utf8")
    .trim()
    .split("\n")
    .map(JSON.parse);
  assert.ok(
    calls.every((c) => !c.args.some((a) => a.startsWith("safe.directory="))),
  );
});

test("hookless launcher queues the existing bootstrap and returns EX_TEMPFAIL", async function hooklessLauncherQueuesTheExistingBootstrap(t) {
  // The launcher beside the candidate epr, as the epr-follows-head steps find it.
  const candidate = process.env.EPR_BIN ?? "/tmp/eprfs-gate-target/debug/epr";
  const launcher =
    process.env.BIN_LAUNCH_BIN ?? join(dirname(candidate), "bin-launch");
  if (!existsSync(launcher)) {
    t.skip(
      `no built launcher at ${launcher}: build elohim-device-runtime or set BIN_LAUNCH_BIN`,
    );
    return;
  }
  const f = fixture(t);
  writeFileSync(f.trust, `${f.root}\n`);
  const scripts = join(f.root, "genesis/agentic");
  mkdirSync(scripts, { recursive: true });
  symlinkSync(script, join(scripts, "native-epr-bootstrap.mjs"));
  const bin = join(f.base, "bin");
  symlinkSync(launcher, join(bin, "epr"));
  writeFileSync(
    join(bin, "bin-tools.json"),
    JSON.stringify({
      version: 1,
      tools: {
        epr: {
          bins: ["epr"],
          source: { kind: "repository" },
          candidateEnv: { epr: ["EPR_BIN"] },
        },
      },
    }),
  );
  const cargo = join(bin, "cargo");
  writeFileSync(
    cargo,
    `#!${process.execPath}\nconst fs = require('node:fs'); const path = require('node:path');
fs.appendFileSync(${JSON.stringify(f.calls)}, 'build\\n');
const dir = path.join(process.env.CARGO_TARGET_DIR, 'debug'); fs.mkdirSync(dir, {recursive:true});
fs.writeFileSync(path.join(dir, 'epr'), '#!${process.execPath}\\n' + ${JSON.stringify("const fs = require('node:fs'); fs.appendFileSync(" + JSON.stringify(f.calls) + ", JSON.stringify(process.argv.slice(2)) + '\\n'); console.log(JSON.stringify({clean:true,cid:'fixture'}));")});
fs.chmodSync(path.join(dir, 'epr'), 0o755);
`,
  );
  chmodSync(cargo, 0o755);
  const env = {
    ...process.env,
    PATH: `${bin}:${dirname(process.execPath)}:/usr/bin:/bin`,
    EPR_REPO_ROOT: f.root,
    EPR_STORE: join(f.base, "private-store"),
    EPR_BIN_TOOLS: join(bin, "bin-tools.json"),
    HOME: join(f.base, "home"),
    XDG_CONFIG_HOME: join(f.base, "xdg"),
  };
  delete env.EPR_BIN;
  delete env.ELOHIM_DEVICE_KEY_FILE;
  const result = spawnSync(join(bin, "epr"), ["--version"], {
    env,
    cwd: f.root,
    encoding: "utf8",
    timeout: 2000,
  });
  assert.equal(result.status, 75, result.stderr);
  assert.match(result.stderr, /bootstrapping: retry shortly/);
  const deadline = Date.now() + 10000;
  let calls = "";
  while (Date.now() < deadline) {
    try {
      calls = readFileSync(f.calls, "utf8");
    } catch {}
    if (calls.includes("install-launcher")) break;
    await new Promise((resolve) => setTimeout(resolve, 50));
  }
  assert.match(calls, /build/);
  assert.match(calls, /migrate/);
  assert.match(calls, /land/);
  assert.match(calls, /install-launcher/);
});
