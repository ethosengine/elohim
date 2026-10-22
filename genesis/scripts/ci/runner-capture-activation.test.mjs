import test from "node:test";
import { execFileSync } from "node:child_process";
import assert from "node:assert/strict";
import {
  mkdtempSync,
  chmodSync,
  statSync,
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
  "transport-failure",
])
  test(`capture ${mode} has no stale success on refusal`, (t) => {
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
    let reads = 0;
    const execute = (cmd, args, options) => {
      assert.equal(options.env.KUBECONFIG, "/private/selected-context");
      if (cmd === "kubectl" && args.at(-1) === "name") {
        assert.equal(options.timeout, 20000);
        if (mode === "transport-failure")
          throw new Error("transport unavailable");
        return "pod/adam-0\n";
      }
      if (cmd === "bash") {
        assert.equal(options.env.COMPUTE_WORKER_PROVIDER, provider);
        assert.equal(options.env.COMPUTE_WORKER_IMAGE, pod.image);
        if (mode === "bootstrap-timeout") {
          writeFileSync(
            statusFile,
            "REFUSED compute worker receipt: startup budget 900s exhausted: worker bootstrap or grant receipt unavailable",
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
      if (process.getuid?.() === 0) {
        chmodSync(root, 0o755);
        execFileSync(
          process.execPath,
          ["-e", "require('node:fs').readFileSync(process.argv[1])", artifact],
          { uid: 65534, gid: 65534 },
        );
      }
      assert.equal(
        JSON.parse(readFileSync(artifact)).nativeStatusChecked,
        false,
      );
    } else {
      assert.throws(
        go,
        (error) =>
          mode !== "bootstrap-timeout" ||
          error.activationReason === "bootstrap-or-grant-receipt-timeout",
      );
      assert.equal(existsSync(artifact), false);
      assert.equal(existsSync(statusFile), false);
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
      return "bootstrap failed private-bootstrap-value eyJabc.payload.signature";
    },
  });
  assert.equal(calls.length, 4);
  for (const call of calls) {
    assert.equal(call.command, "kubectl");
    assert.equal(call.args[0], "--request-timeout=15s");
    assert.equal(call.opts.timeout, 20000);
    assert.equal(call.opts.maxBuffer, 262144);
    assert(["get", "logs", "exec"].includes(call.args[1]));
    if (call.args[1] === "exec") {
      assert.match(call.args.at(-1), /O_RDONLY \| fs.constants.O_NOFOLLOW/);
      assert.match(call.args.at(-1), /Math.min\(stat.size, 4096\)/);
      assert(!call.args.at(-1).includes("process.env"));
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
    if (process.getuid?.() === 0) {
      chmodSync(root, 0o755);
      execFileSync(
        process.execPath,
        ["-e", "require('node:fs').readFileSync(process.argv[1])", file],
        { uid: 65534, gid: 65534 },
      );
    }
  }
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
