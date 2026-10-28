import test from "node:test";
import { execFileSync } from "node:child_process";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  chmodSync,
  statSync,
  symlinkSync,
  mkdirSync,
  writeFileSync,
  readFileSync,
  existsSync,
  rmSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import {
  captureActivation,
  captureDiagnostics,
  readPersistedLogs,
  verifyDeclaration,
} from "./runner-capture-activation.mjs";
// Namespace UID 0 does not imply that Node can set a child's UID/GID to 65534.
function mapsGuest(mapping) {
  return mapping
    .trim()
    .split("\n")
    .some((row) => {
      const fields = row.trim().split(/\s+/).map(Number);
      if (fields.length !== 3 || !fields.every(Number.isSafeInteger))
        return false;
      const [inside, outside, count] = fields;
      return (
        inside >= 0 &&
        outside >= 0 &&
        count > 0 &&
        65534 >= inside &&
        65534 - inside < count
      );
    });
}

function distinctUidPrerequisite(uid, platform, readProc) {
  if (uid !== 0)
    return "distinct-UID read prerequisite unavailable: root is required";
  if (platform !== "linux") return false;
  const missing = ["uid_map", "gid_map"].filter(
    (name) => !mapsGuest(readProc(name)),
  );
  if (missing.length) {
    return `distinct-UID read prerequisite unavailable: ID 65534 is not mapped in ${missing.map((name) => `/proc/self/${name}`).join(" and ")}`;
  }
  // Node/libuv ignores setgroups failure; errors setting UID/GID still fail the test.
  return false;
}

const distinctUidOnly = distinctUidPrerequisite(
  process.getuid?.(),
  process.platform,
  (name) => readFileSync(`/proc/self/${name}`, "utf8"),
);
const fullMap = "0 0 4294967295\n";
const singleMap = "0 0 1\n";
const prerequisite = (uidMap, gidMap) =>
  distinctUidPrerequisite(0, "linux", (name) => {
    assert(["uid_map", "gid_map"].includes(name));
    return { uid_map: uidMap, gid_map: gidMap }[name];
  });

test("distinct-UID prerequisite uses both inside namespace mappings", () => {
  assert.equal(prerequisite(fullMap, fullMap), false);
  assert.equal(
    prerequisite("0 1000 1\n65534 2000 1\n", "65534 3000 1\n"),
    false,
  );
  assert.match(prerequisite(singleMap, singleMap), /uid_map.*gid_map/);
  assert.match(prerequisite(singleMap, fullMap), /uid_map/);
  assert.doesNotMatch(prerequisite(singleMap, fullMap), /gid_map/);
  assert.match(prerequisite(fullMap, singleMap), /gid_map/);
  assert.doesNotMatch(prerequisite(fullMap, singleMap), /uid_map/);
});

test("distinct-UID mapping guards range boundaries and malformed rows", () => {
  for (const mapping of ["65534 42 1", "65533 42 2", "0 42 65535"]) {
    assert.equal(mapsGuest(mapping), true, mapping);
  }
  for (const mapping of [
    "0 65534 1",
    "65535 42 1",
    "65533 42 1",
    "65534 42 0",
    "0 0 65534",
    "",
    "invalid",
    "65534 -1 1",
    "65534 0 1.5",
    "65534 0 1 2",
  ]) {
    assert.equal(mapsGuest(mapping), false, mapping);
  }
});

test("distinct-UID prerequisite retains non-root and non-Linux behavior", () => {
  const unread = () => assert.fail("namespace maps must not be read");
  assert.match(
    distinctUidPrerequisite(1000, "linux", unread),
    /root is required/,
  );
  assert.match(
    distinctUidPrerequisite(undefined, "win32", unread),
    /root is required/,
  );
  assert.equal(distinctUidPrerequisite(0, "darwin", unread), false);
});

test("distinct-UID prerequisite does not hide proc read errors", () => {
  const error = new Error("uid_map read failed");
  assert.throws(
    () =>
      distinctUidPrerequisite(0, "linux", () => {
        throw error;
      }),
    (actual) => actual === error,
  );
});

const now = Date.parse("2026-10-22T04:00:00Z");
const provider = "uhCAk" + "a".repeat(48),
  recipient = "uhCAk" + "b".repeat(48);
const declaration = {
  enabled: true,
  stewardOfRecord: "human:matthew",
  fixtureProvider: "adam",
  provider,
  recipient,
  grant: {
    recipient,
    scope: "measure-stage",
    issuedAt: "2026-10-22T03:00:00Z",
    validFrom: "2026-10-22T03:00:00Z",
    validUntil: "2026-10-23T03:00:00Z",
    bounds: {
      epr_scope: ["*"],
      reach_ceiling: "commons",
      rate_per_hour: 4,
      rotation_ttl_days: 1,
    },
  },
};
const pod = {
  uid: "uid",
  containerId: "containerd://id",
  restartCount: 0,
  image: "registry/worker@sha256:" + "a".repeat(64),
};
const activation = {
  provider,
  recipient,
  state: "active",
  worker: { podUid: pod.uid, ...pod },
};
test("verified timestamps explicitly describe bootstrap observation, not fresh native state", () => {
  const result = verifyDeclaration(
    activation,
    declaration,
    pod,
    declaration,
    now,
  );
  assert.equal(result.validUntil, declaration.grant.validUntil);
  assert.equal(result.issuedAt, declaration.grant.issuedAt);
  assert.equal(result.nativeStatusChecked, false);
  assert.match(result.stateEvidence, /bootstrap-receipt/);
});
test("changed declaration or incarnation refuses even with same parties and image", () => {
  for (const change of [
    (d) => (d.grant.validUntil = "2026-10-23T02:00:00Z"),
    (d) => (d.grant.bounds.rate_per_hour = 5),
    (d) => (d.recipient = provider),
  ]) {
    const d = structuredClone(declaration);
    change(d);
    assert.throws(() =>
      verifyDeclaration(activation, d, pod, declaration, now),
    );
  }
  for (const field of ["uid", "containerId", "restartCount", "image"])
    assert.throws(() =>
      verifyDeclaration(
        activation,
        declaration,
        { ...pod, [field]: "changed" },
        declaration,
        now,
      ),
    );
});
for (const mode of [
  "success",
  "expired",
  "future",
  "malformed",
  "inverted",
  "changed",
  "late-expiry",
  "read-failure",
  "bootstrap-timeout",
  "worker-state-timeout",
  "capacity-mismatch",
  "transport-failure",
  "first-attempt-refusal",
])
  test(`capture ${mode} has no stale success on refusal`, async (t) => {
    const root = mkdtempSync(join(tmpdir(), "capture-window-"));
    t.after(() => rmSync(root, { recursive: true, force: true }));
    const config = structuredClone(declaration);
    if (mode === "expired") config.grant.validUntil = "2026-10-22T03:30:00Z";
    if (mode === "future") config.grant.validFrom = "2026-10-22T05:00:00Z";
    if (mode === "malformed") config.grant.validUntil = "tomorrow";
    if (mode === "inverted") config.grant.validUntil = "2026-10-22T02:00:00Z";
    mkdirSync(join(root, "genesis/orchestrator/data"), { recursive: true });
    writeFileSync(
      join(root, "genesis/orchestrator/data/adam-compute-worker.json"),
      JSON.stringify({
        enabled: true,
        image: pod.image,
        performer: provider,
        runnerGrant: config,
      }),
    );
    const artifact = join(
      root,
      "compute-worker-evidence/ns--adam/activation.json",
    );
    mkdirSync(join(root, "compute-worker-evidence/ns--adam"), {
      recursive: true,
    });
    writeFileSync(artifact, "stale");
    const statusFile = join(
      root,
      "compute-worker-evidence/ns--adam/status.txt",
    );
    writeFileSync(statusFile, "READY stale");
    chmodSync(join(root, "compute-worker-evidence"), 0o700);
    chmodSync(join(root, "compute-worker-evidence/ns--adam"), 0o700);
    chmodSync(statusFile, 0o600);
    if (mode === "first-attempt-refusal")
      rmSync(join(root, "compute-worker-evidence"), { recursive: true });
    let reads = 0;
    const execute = (cmd, args, options) => {
      assert.equal(options.env.KUBECONFIG, "/private/selected-context");
      if (cmd === "kubectl" && args.at(-1) === "name") {
        assert.equal(options.timeout, 20000);
        if (["transport-failure", "first-attempt-refusal"].includes(mode))
          throw new Error("transport unavailable");
        return "pod/adam-0\n";
      }
      if (cmd === "bash") {
        assert.equal(options.env.COMPUTE_WORKER_PROVIDER, provider);
        assert.equal(options.env.COMPUTE_WORKER_IMAGE, pod.image);
        const refusal = {
          "bootstrap-timeout":
            "startup budget 900s exhausted: worker bootstrap or grant receipt unavailable",
          "worker-state-timeout":
            "startup budget 900s exhausted: worker missing, not Ready, terminating, or image differs",
          "capacity-mismatch":
            "public receipt, running worker, or enforced capacity differs",
        }[mode];
        if (refusal) {
          writeFileSync(
            statusFile,
            `REFUSED compute worker receipt: ${refusal}`,
          );
          throw new Error("private child error");
        }
        writeFileSync(artifact, JSON.stringify(activation));
        writeFileSync(statusFile, "READY original capture");
        return "";
      }
      assert.equal(cmd, "kubectl");
      assert(args.includes("get"));
      assert(!args.includes("exec"));
      const query = args.at(-1);
      assert.match(query, /COMPUTE_RUNNER_GRANT_JSON/);
      assert(!query.includes("BOOTSTRAP_KEY"));
      if (mode === "read-failure") throw new Error("SECRET sentinel");
      const d = structuredClone(config);
      if (mode === "changed") d.grant.bounds.rate_per_hour = 9;
      return [
        pod.uid,
        pod.containerId,
        pod.restartCount,
        pod.image,
        JSON.stringify(d),
      ].join("\n");
    };
    const go = () =>
      captureActivation({
        root,
        workload: "adam",
        namespace: "ns",
        env: { KUBECONFIG: "/private/selected-context" },
        execute,
        now: () =>
          mode === "late-expiry" && reads++ > 0
            ? Date.parse(config.grant.validUntil)
            : now,
      });
    if (mode === "success") {
      go();
      assert.equal(statSync(artifact).mode & 0o777, 0o644);
      assert.equal(statSync(statusFile).mode & 0o777, 0o644);
      await t.test(
        "activation.json is readable by UID/GID 65534",
        { skip: distinctUidOnly },
        () => {
          chmodSync(root, 0o755);
          execFileSync(
            process.execPath,
            [
              "-e",
              "require('node:fs').readFileSync(process.argv[1])",
              artifact,
            ],
            { uid: 65534, gid: 65534 },
          );
        },
      );
      assert.equal(
        JSON.parse(readFileSync(artifact)).nativeStatusChecked,
        false,
      );
    } else {
      const reason =
        {
          "bootstrap-timeout": "bootstrap-or-grant-receipt-timeout",
          "worker-state-timeout": "worker-state-or-image-timeout",
          "capacity-mismatch": "receipt-worker-or-capacity-mismatch",
        }[mode] || "declaration-window-or-incarnation-unproven";
      assert.throws(go, (error) => error.activationReason === reason);
      assert.equal(existsSync(artifact), false);
      assert.equal(readFileSync(statusFile, "utf8"), `REFUSED: ${reason}\n`);
      assert.equal(statSync(statusFile).mode & 0o777, 0o644);
      assert.equal(
        statSync(join(root, "compute-worker-evidence")).mode & 0o777,
        0o755,
      );
      assert.equal(
        statSync(join(root, "compute-worker-evidence/ns--adam")).mode & 0o777,
        0o755,
      );
    }
  });

test("failure diagnostics are bounded read-only worker observations and redact credentials", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "activation-diagnostics-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const calls = [];
  const oldDirectory = join(
    root,
    "rollout-evidence/alpha--compute-worker--adam",
  );
  mkdirSync(oldDirectory, { recursive: true, mode: 0o700 });
  writeFileSync(join(oldDirectory, "current.log"), "old", { mode: 0o600 });
  await captureDiagnostics({
    root,
    workload: "adam",
    namespace: "alpha",
    env: { ADMIN_KEY: "private-bootstrap-value" },
    execute(command, args, opts) {
      calls.push({ command, args, opts });
      if (args.includes("elohim-node"))
        return "Request error: missing field private-provider-detail\nRequest error: timeout private-value";
      return "bootstrap failed private-bootstrap-value eyJabc.payload.signature";
    },
  });
  assert.equal(calls.length, 5);
  for (const call of calls) {
    assert.equal(call.command, "kubectl");
    assert.equal(call.args[0], "--request-timeout=15s");
    assert.equal(call.opts.timeout, 20000);
    assert.equal(call.opts.maxBuffer, 262144);
    assert(["get", "logs", "exec"].includes(call.args[1]));
    if (call.args[1] === "exec") {
      assert.match(call.args.at(-1), /O_RDONLY \| fs.constants.O_NOFOLLOW/);
      assert.match(call.args.at(-1), /Math.min\(stat.size, 4096\)/);
      assert.match(call.args.at(-1), /issuance intentionally not attempted/);
      assert(!call.args.at(-1).includes("JSON.stringify(env)"));
    }
    assert(!call.args.join(" ").includes(".spec"));
  }
  for (const name of [
    "state.txt",
    "current.log",
    "previous.log",
    "mesh-startup.log",
  ]) {
    const content = readFileSync(
      join(root, "rollout-evidence/alpha--compute-worker--adam", name),
      "utf8",
    );
    assert(!content.includes("private-bootstrap-value"));
    assert(!content.includes("eyJabc"));
    assert(content.includes("bootstrap failed"));
    const file = join(oldDirectory, name);
    assert.equal(statSync(file).mode & 0o777, 0o644);
    await t.test(
      `${name} is readable by UID/GID 65534`,
      { skip: distinctUidOnly },
      () => {
        chmodSync(root, 0o755);
        execFileSync(
          process.execPath,
          ["-e", "require('node:fs').readFileSync(process.argv[1])", file],
          { uid: 65534, gid: 65534 },
        );
      },
    );
  }
  const providerErrors = JSON.parse(
    readFileSync(join(oldDirectory, "provider-errors.json"), "utf8"),
  );
  assert.deepEqual(providerErrors.categories, [
    "native-wire-shape",
    "native-timeout",
  ]);
  assert.equal(providerErrors.grantRequestCorrelationProven, false);
  assert(!JSON.stringify(providerErrors).includes("private-"));
  assert(
    !existsSync(
      join(root, "compute-worker-evidence/alpha--adam/activation.json"),
    ),
  );
});

test("diagnostic read failure retains redacted stderr without greening activation", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "activation-diagnostics-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  await captureDiagnostics({
    root,
    workload: "adam",
    namespace: "alpha",
    env: { ADMIN_KEY: "secret-bootstrap-value" },
    execute() {
      throw Object.assign(new Error("secret error"), {
        status: 1,
        stderr: "Forbidden request secret-bootstrap-value",
      });
    },
  });
  const content = readFileSync(
    join(root, "rollout-evidence/alpha--compute-worker--adam/current.log"),
    "utf8",
  );
  assert.equal(
    content,
    "diagnostic read unavailable (exit 1)\nForbidden request [REDACTED]",
  );
});

test("CLI transport refusal removes stale activation and READY before any cluster command", (t) => {
  const root = mkdtempSync(join(tmpdir(), "capture-transport-refusal-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const directory = join(
    root,
    "compute-worker-evidence/elohim-alpha--elohim-adam-alpha",
  );
  mkdirSync(directory, { recursive: true });
  writeFileSync(join(directory, "activation.json"), '{"state":"active"}');
  writeFileSync(join(directory, "status.txt"), "READY");
  assert.throws(() =>
    execFileSync(
      process.execPath,
      [
        new URL("./runner-capture-activation.mjs", import.meta.url).pathname,
        "elohim-adam-alpha",
        "elohim-alpha",
      ],
      {
        env: {
          ...process.env,
          WORKSPACE: root,
          KUBECONFIG: join(root, "missing-explicit-config"),
        },
        stdio: "pipe",
      },
    ),
  );
  assert(!existsSync(join(directory, "activation.json")));
  assert.match(
    readFileSync(join(directory, "status.txt"), "utf8"),
    /^REFUSED:/,
  );
});

function logObserver(overrides = {}) {
  let time = 0;
  const calls = [];
  const image = "registry/worker@sha256:" + "a".repeat(64);
  return {
    calls,
    image,
    options: {
      args: [
        "exec",
        "adam-0",
        "-n",
        "alpha",
        "-c",
        "compute-worker",
        "--",
        "node",
        "-e",
        "fixed",
      ],
      options: {},
      pod: "adam-0",
      namespace: "alpha",
      expectedImage: image,
      expectedUid: "uid",
      now: () => time,
      pause: async (ms) => {
        time += ms;
      },
      budgetMs: 5000,
      execute: (_command, args, options) => {
        calls.push({ args, options });
        return args.includes("get")
          ? `uid\n${image}\nstarted\n3\ncontainer`
          : "size=3 mtime=2026-10-22T00:00:00Z\nlog\nunavailable";
      },
      ...overrides,
    },
  };
}
test("persisted log retry observes running without requiring Ready and survives exec race", async () => {
  const f = logObserver();
  let observations = 0,
    execs = 0;
  f.options.execute = (_cmd, args) => {
    if (args.includes("get"))
      return `uid\n${f.image}\n${++observations > 1 ? "started" : ""}\n3\ncontainer`;
    if (++execs === 1)
      throw Object.assign(new Error("race"), {
        stderr: 'container not found ("compute-worker")',
      });
    return "size=3 mtime=2026-10-22T00:00:00Z\nlog\nunavailable";
  };
  const result = await readPersistedLogs(f.options);
  assert.equal(execs, 2);
  assert.match(result, /diagnostic observation only.*restartCount=3/);
  assert.match(result, /mtime=.*\nlog\nunavailable/);
});
test("persisted log retry respects deadline and request limits", async () => {
  const f = logObserver({ budgetMs: 2500 });
  f.options.execute = (_cmd, args, options) => {
    f.calls.push(options.timeout);
    return `uid\n${f.image}\n\n3\ncontainer`;
  };
  await assert.rejects(readPersistedLogs(f.options), /deadline/);
  assert.deepEqual(f.calls, [2500, 1500, 500]);
});
test("provider reads fit exec budget while identity reads and total deadline stay bounded", async () => {
  for (const budgetMs of [20000, 2500]) {
    const f = logObserver({ budgetMs });
    await readPersistedLogs(f.options);
    assert.equal(f.calls.length, 3);
    const limits =
      budgetMs === 20000 ? [4000, 13000, 4000] : [2500, 2500, 2500];
    assert.deepEqual(
      f.calls.map((call) => call.options.timeout),
      limits,
    );
    assert.equal(
      f.calls[1].args[0],
      `--request-timeout=${Math.min(12000, budgetMs)}ms`,
    );
    assert.equal(
      f.calls[0].args[0],
      `--request-timeout=${Math.min(3000, budgetMs)}ms`,
    );
    assert(f.calls.every((call) => call.options.killSignal === "SIGKILL"));
  }
});
test("provider transport timeout retains already emitted public proof and refuses activation", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "provider-timeout-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  await captureDiagnostics({
    root,
    workload: "adam",
    namespace: "alpha",
    env: {},
    execute(_command, args) {
      if (args.includes("exec"))
        throw Object.assign(new Error("timed out"), {
          stdout: 'recovery-proof {"status":"verified"}\n',
          stderr: "transport timeout",
        });
      return "";
    },
  });
  const directory = join(root, "rollout-evidence/alpha--compute-worker--adam");
  assert.match(
    readFileSync(join(directory, "mesh-startup.log"), "utf8"),
    /recovery-proof.*verified/,
  );
  assert.match(readFileSync(join(directory, "status.txt"), "utf8"), /^REFUSED/);
  assert(!existsSync(join(directory, "activation.json")));
});
for (const failure of ["Forbidden", "Unauthorized", "configuration missing"]) {
  test(`persisted log ${failure} refuses immediately`, async () => {
    let calls = 0;
    const f = logObserver({
      execute() {
        calls++;
        throw new Error(failure);
      },
    });
    await assert.rejects(readPersistedLogs(f.options), new RegExp(failure));
    assert.equal(calls, 1);
  });
}
for (const field of ["uid", "image"]) {
  test(`persisted log ${field} change refuses`, async () => {
    const f = logObserver();
    let calls = 0;
    f.options.execute = () => {
      calls++;
      return `${calls > 1 && field === "uid" ? "replacement" : "uid"}\n${calls > 1 && field === "image" ? "different" : f.image}\n\n3\ncontainer`;
    };
    await assert.rejects(readPersistedLogs(f.options), /changed/);
    assert.equal(calls, 2);
  });
}

test("persisted logs refuse replacement during exec before publishing output", async () => {
  const f = logObserver();
  let gets = 0;
  f.options.execute = (_cmd, args) =>
    args.includes("get")
      ? `${++gets > 1 ? "replacement" : "uid"}\n${f.image}\nstarted\n3\ncontainer`
      : "must not publish";
  await assert.rejects(readPersistedLogs(f.options), /changed during read/);
});

test("diagnostics passes its original observed pod identity to persisted reader", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "diagnostic-binding-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const f = logObserver();
  let gets = 0;
  const directory = await captureDiagnostics({
    root,
    workload: "adam",
    namespace: "alpha",
    retryLogs: true,
    expectedImage: f.image,
    execute(_cmd, args) {
      if (args.includes("get"))
        return ++gets === 1
          ? "original\nRunning"
          : `replacement\n${f.image}\nstarted\n3\ncontainer`;
      assert(!args.includes("exec"));
      return "logs";
    },
  });
  assert.match(
    readFileSync(join(directory, "mesh-startup.log"), "utf8"),
    /identity changed/,
  );
  assert.match(readFileSync(join(directory, "status.txt"), "utf8"), /^REFUSED/);
});

test("persisted identity inventory emits bounded hashes without key or credential bytes", async (t) => {
  const root = mkdtempSync(join(tmpdir(), "identity-observation-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  let script;
  await captureDiagnostics({
    root,
    workload: "adam",
    namespace: "alpha",
    env: {},
    execute(_command, args) {
      if (args.includes("exec")) script = args.at(-1);
      return "";
    },
  });
  const mesh = join(root, "mesh");
  const credentials = join(mesh, "matthew/closed-chain-fence/credentials");
  mkdirSync(credentials, { recursive: true });
  mkdirSync(join(mesh, "initialization-continuation/consumed"), {
    recursive: true,
  });
  mkdirSync(join(mesh, "jessica"));
  writeFileSync(join(mesh, "matthew/identity.key"), "private-transport-key");
  writeFileSync(join(mesh, "matthew/content.db"), "private-database-bytes");
  writeFileSync(
    join(mesh, "matthew/runtime-config.toml"),
    "private-config-token",
  );
  writeFileSync(
    join(mesh, "initialization-continuation/apps-before.jsonl"),
    "public-identity-baseline",
  );
  writeFileSync(
    join(credentials, "a".repeat(78) + "-" + "b".repeat(78) + ".json"),
    "private-signing-credential",
  );
  symlinkSync(
    join(mesh, "matthew/identity.key"),
    join(mesh, "matthew/iroh.key"),
  );
  writeFileSync(join(mesh, "jessica/identity.key"), Buffer.alloc(65537, 120));
  const output = execFileSync(
    process.execPath,
    ["-e", script.replaceAll("/var/lib/compute/mesh/", mesh + "/")],
    { encoding: "utf8" },
  );
  for (const secret of [
    "private-transport-key",
    "private-database-bytes",
    "private-config-token",
    "private-signing-credential",
    "public-identity-baseline",
  ])
    assert(!output.includes(secret));
  const rows = output
    .split("\n")
    .filter((line) => line.startsWith("identity-observation "))
    .map((line) => JSON.parse(line.slice(21)));
  const row = (path) => rows.find((item) => item.path === path);
  assert.match(row("matthew/identity.key").sha256, /^[a-f0-9]{64}$/);
  assert.equal(row("matthew/iroh.key").unavailable, true);
  assert.equal(row("matthew/iroh.key").missing, undefined);
  assert.equal(row("jessica/iroh.key").missing, true);
  assert.equal(row("jessica/iroh.key").unavailable, undefined);
  assert.equal(row("jessica/identity.key").unavailable, true);
  assert.equal(row("matthew/content.db").sha256, undefined);
  assert.equal(
    row("matthew/closed-chain-fence/credentials").credentialCount,
    1,
  );
  assert.equal(row("initialization-continuation/consumed").kind, "directory");
  assert.match(
    row("initialization-continuation/apps-before.jsonl").sha256,
    /^[a-f0-9]{64}$/,
  );
});
