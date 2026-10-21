import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, rm, readFile, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { probeAuthority } from "./runner-authority-probe.mjs";

// Syntactic keys are test data only; no network/session is constructed here.
const key = (c) => `uhCAk${c.repeat(48)}`;
const action = (c) => `uhCkk${c.repeat(48)}`;
const root = resolve(
  process.env.PROBE_REPO_ROOT || join(import.meta.dirname, "../../.."),
);
async function fixture(t, fault) {
  const out = await mkdtemp(join(tmpdir(), "authority-probe-"));
  t.after(() => rm(out, { recursive: true, force: true }));
  const requester = key("A"),
    otherRunner = key("B"),
    providerKey = key("C");
  const grants = [],
    tasks = [],
    cleanup = [],
    operations = [];
  let current;
  const options = {
    root,
    out,
    requester,
    otherRunner,
    propagationMs: 0,
    env: { RUNNER_JWT: "fixture-secret" },
    config: {
      provider: providerKey,
      runnerGrant: action("P"),
      repoRoot: "/provider/repo",
      nodeBin: "/usr/bin/node",
      justBin: "/provider/mesh-just.sh",
      runtimeImage: `sha256:${"a".repeat(64)}`,
      writablePaths: ["/provider/reports"],
      cpuMillis: 8000,
      memoryBytes: 25769803776,
      timeoutSeconds: 1800,
      feature: "features/dataplane/peer-mesh.feature",
    },
    build: async () => ({
      taskPath: "/task",
      runnerPath: "/runner",
      dnaPath: "/dna",
    }),
    callRunner: async (args, deps) => {
      let response;
      if (args[0] === "grant-status") {
        const found = grants.find((g) => g.grantActionHash === args[1]);
        response = {
          ...found,
          cid: found.grantCid,
          scope: "measure-stage",
          runner: requester,
          state: found.state === "revoked" ? "withdrawn" : found.state,
        };
      } else if (args[0] === "submit") {
        // Assert the safety property at the only transition that exposes work.
        assert(current.recipient !== requester || current.state === "revoked");
        response = {
          requestActionHash: action(String(tasks.length + 1)),
          taskCid: `cid-${tasks.length}`,
          requester,
          provider: providerKey,
          grantActionHash: current.grantActionHash,
        };
        tasks.push(response);
      } else
        response = tasks.find((task) => task.requestActionHash === args[1]);
      deps.stdout(JSON.stringify(response));
      return 0;
    },
    provider: async (method, path, body) => {
      if (path === "/api/v1/compute/grants") {
        if (body.revoke) {
          assert(grants.some((g) => g.grantCid === body.grantCid));
          cleanup.push(body.grantCid);
          grants.find((g) => g.grantCid === body.grantCid).state =
            fault === "withdrawal-not-observed" ? "active" : "revoked";
          return { status: 200, body: { state: "revoked" } };
        }
        current = {
          grantCid: `uhCEk${String(grants.length + 1).repeat(48)}`,
          grantActionHash: action(String(grants.length + 1)),
          provider: providerKey,
          recipient: body.recipient,
          state: "active",
        };
        grants.push(current);
        return { status: 200, body: { ...current } };
      }
      if (path.includes("/economic-events/")) {
        // Like the actual handler: no percent decoding before exact DB lookup.
        const id = path.slice("/api/v1/economic-events/".length);
        const exists =
          fault === "admission-exists" &&
          id === `compute-admission:${tasks.at(-1).requestActionHash}`;
        return { status: exists ? 200 : 404, body: {} };
      }
      if (method === "POST") {
        operations.push(path.split("/").at(-1));
        return {
          status: fault === "accepted" ? 200 : 403,
          body: { reason: "compute-grant-refused" },
        };
      }
      return {
        status: 200,
        body: tasks.find((task) => path.endsWith(task.requestActionHash)),
      };
    },
  };
  return { options, tasks, grants, cleanup, operations };
}
test("real signed grant-status stays active for a different recipient", async () => {
  const { grantStatus } = await import(
    pathToFileURL(join(root, "genesis/a2o/scripts/lib/compute-runner.ts")).href
  );
  const now = Date.now(),
    provider = key("C"),
    other = key("B");
  const status = grantStatus({
    grantActionHash: action("G"),
    cid: "native-entry",
    author: provider,
    policy: {
      provider,
      recipient: other,
      scope: "measure-stage",
      valid_from: new Date(now - 1000).toISOString(),
      valid_until: new Date(now + 60000).toISOString(),
    },
    links: [{ author: provider, state: "active" }],
    now,
  });
  assert.equal(status.state, "active");
  assert.equal(status.recipient, other);
  assert.notEqual(status.recipient, key("A"));
});
test("interrupted issuance resumes with identical declaration and no second native grant", async (t) => {
  const f = await fixture(t);
  const native = f.options.provider;
  let declaration,
    issued,
    dropped = false;
  f.options.provider = async (method, path, body) => {
    if (path === "/api/v1/compute/grants" && !body.revoke) {
      if (declaration && JSON.stringify(body) === declaration) return issued;
      const response = await native(method, path, body);
      if (!dropped) {
        declaration = JSON.stringify(body);
        issued = response;
        dropped = true;
        throw new Error("response lost after native grant committed");
      }
      return response;
    }
    return native(method, path, body);
  };
  await assert.rejects(probeAuthority(f.options), AggregateError);
  const journal = JSON.parse(
    await readFile(join(f.options.out, "journal.json"), "utf8"),
  );
  assert.equal(JSON.stringify(journal.entries[0].declaration), declaration);
  assert.equal(journal.entries[0].grant, undefined);
  assert.deepEqual(journal.entries[0].declaration.bounds.epr_scope, ["*"]);
  const result = await probeAuthority({ ...f.options, now: Date.now() + 1000 });
  assert.equal(result.evidence.length, 2);
  assert.equal(f.grants.length, 2);
});
test("only never-authorized native requests are exposed and evidence omits credentials", async (t) => {
  const f = await fixture(t);
  const result = await probeAuthority(f.options);
  assert.equal(result.evidence.length, 2);
  assert.deepEqual(f.operations, [
    "accept",
    "authorize-launch",
    "accept",
    "authorize-launch",
  ]);
  assert.equal(new Set(f.cleanup).size, 2);
  assert(!JSON.stringify(result).includes("fixture-secret"));
});
for (const fault of [
  "accepted",
  "admission-exists",
  "withdrawal-not-observed",
]) {
  test(`refuses ${fault} and cleans only created disposable grants`, async (t) => {
    const f = await fixture(t, fault);
    await assert.rejects(probeAuthority(f.options), AggregateError);
    assert(f.cleanup.length > 0);
    if (fault === "withdrawal-not-observed") assert.equal(f.tasks.length, 1);
  });
}

test("completed replay verifies both exact tasks and admission IDs without new mutations", async (t) => {
  const f = await fixture(t);
  const original = await probeAuthority(f.options);
  const native = f.options.provider,
    reads = [];
  f.options.provider = async (method, path, body) => {
    assert.equal(method, "GET", "completed replay must not mutate");
    reads.push(path);
    return native(method, path, body);
  };
  f.options.build = async () => assert.fail("completed replay must not build");
  f.options.callRunner = async () =>
    assert.fail("completed replay must not submit or reissue");
  const replay = await probeAuthority(f.options);
  assert.deepEqual(
    replay.evidence.map((e) => e.observedAt),
    original.evidence.map((e) => e.observedAt),
  );
  assert.equal(reads.length, 4);
  for (const e of original.evidence) {
    assert(reads.includes(`/api/v1/compute/tasks/${e.requestActionHash}`));
    assert(
      reads.includes(
        `/api/v1/economic-events/compute-admission:${e.requestActionHash}`,
      ),
    );
  }
  assert.equal(f.grants.length, 2);
  assert.equal(f.tasks.length, 2);
});
for (const fault of [
  "provider-unavailable",
  "binding-drift",
  "acceptance",
  "admission",
]) {
  test(`completed replay refuses ${fault}`, async (t) => {
    const f = await fixture(t);
    await probeAuthority(f.options);
    const native = f.options.provider;
    f.options.provider = async (method, path, body) => {
      assert.equal(method, "GET");
      if (fault === "provider-unavailable")
        throw new Error("incarnation unavailable");
      const response = await native(method, path, body);
      if (path.includes("/tasks/")) {
        if (fault === "binding-drift")
          return {
            ...response,
            body: { ...response.body, provider: key("D") },
          };
        if (fault === "acceptance")
          return {
            ...response,
            body: { ...response.body, acceptance: { exists: true } },
          };
      } else if (fault === "admission") return { status: 200, body: {} };
      return response;
    };
    await assert.rejects(probeAuthority(f.options), AggregateError);
    assert.equal(f.tasks.length, 2);
  });
}
test("recorded withdrawn grant cannot resume refusal proof after its validity window", async (t) => {
  const f = await fixture(t);
  await probeAuthority(f.options);
  const path = join(f.options.out, "journal.json");
  const journal = JSON.parse(await readFile(path, "utf8"));
  const entry = journal.entries.find((e) => e.kind === "withdrawn");
  delete entry.evidence;
  entry.cleaned = false;
  entry.declaration.validUntil = new Date(Date.now() - 1000).toISOString();
  await writeFile(path, JSON.stringify(journal));
  const operations = f.operations.length;
  await assert.rejects(probeAuthority(f.options), (error) => {
    assert(error instanceof AggregateError);
    assert(error.errors.some((e) => /validity window elapsed/.test(e.message)));
    return true;
  });
  assert.equal(
    f.operations.length,
    operations,
    "no refusal operation after expiry",
  );
  assert.equal(f.grants.length, 2);
});
test("expiry during a refusal call cannot be published as authority proof", async (t) => {
  const f = await fixture(t);
  const native = f.options.provider,
    clock = Date.now;
  const instant = clock();
  f.options.provider = async (method, path, body) => {
    const response = await native(method, path, body);
    if (method === "POST" && path.endsWith("/accept"))
      Date.now = () => instant + 960000;
    return response;
  };
  try {
    await assert.rejects(probeAuthority(f.options), (error) => {
      assert(
        error.errors.some((e) => /validity window elapsed/.test(e.message)),
      );
      return true;
    });
  } finally {
    Date.now = clock;
  }
  assert.deepEqual(f.operations, ["accept"]);
  const journal = JSON.parse(
    await readFile(join(f.options.out, "journal.json"), "utf8"),
  );
  assert(journal.entries.every((entry) => !entry.evidence));
});
