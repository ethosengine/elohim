import test from "node:test";
import assert from "node:assert/strict";
import {
  mkdtempSync,
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
    let reads = 0;
    const execute = (cmd, args, options) => {
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
        execute,
        now: () =>
          mode === "late-expiry" && reads++ > 0
            ? Date.parse(config.grant.validUntil)
            : now,
      });
    if (mode === "success") {
      go();
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

test("failure diagnostics are bounded read-only worker observations and redact credentials", (t) => {
  const root = mkdtempSync(join(tmpdir(), "activation-diagnostics-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const calls = [];
  captureDiagnostics({
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
  }
  assert(
    !existsSync(
      join(root, "compute-worker-evidence/alpha--adam/activation.json"),
    ),
  );
});

test("diagnostic read failure is recorded without exposing stderr or greening activation", (t) => {
  const root = mkdtempSync(join(tmpdir(), "activation-diagnostics-"));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  captureDiagnostics({
    root,
    workload: "adam",
    namespace: "alpha",
    execute() {
      throw Object.assign(new Error("secret error"), {
        status: 1,
        stderr: "secret stderr",
      });
    },
  });
  const content = readFileSync(
    join(root, "rollout-evidence/alpha--compute-worker--adam/current.log"),
    "utf8",
  );
  assert.equal(content, "diagnostic read unavailable (exit 1)\n");
});
