import assert from "node:assert/strict";
import { randomUUID, randomInt } from "node:crypto";
import { mkdir, readFile, writeFile, rename } from "node:fs/promises";
import { join } from "node:path";
import { pathToFileURL } from "node:url";

// provider(method,path,body) returns
// {status,body}; its implementation stays in CI and holds the local token in-pod.
// It must pin the pod/container incarnation and fail if that identity changes.
export async function probeAuthority({
  config,
  requester,
  otherRunner,
  root,
  out,
  env,
  provider,
  callRunner,
  build,
  activation,
  now = Date.now(),
  propagationMs = 120000,
  pause = (ms) => new Promise((r) => setTimeout(r, ms)),
}) {
  const key = (value) => /^uhCAk[A-Za-z0-9_-]{48}$/.test(value || "");
  assert(key(requester) && key(otherRunner) && key(config.provider));
  assert.notEqual(requester, otherRunner);
  assert.notEqual(requester, config.provider);
  assert(env.RUNNER_JWT, "use an ensured hosted probe runner session");
  assert.equal(config.feature, "features/dataplane/peer-mesh.feature");
  const load = (path) => import(pathToFileURL(join(root, path)).href);
  const { validateConfig } = await load("genesis/scripts/ci/runner-stage.mjs");
  validateConfig(config, config.feature);
  const { execute: nameArtifact } = await load(
    "genesis/agentic/compute/cid-executor.mjs",
  );
  callRunner ??= (await load("genesis/a2o/scripts/compute-runner.ts")).main;
  build ??= (await load("genesis/agentic/compute/stage/build-stage-task.mjs"))
    .buildStageTask;
  await mkdir(out, { recursive: true });
  const journalPath = join(out, "journal.json");
  const binding = JSON.parse(
    JSON.stringify({ config, requester, otherRunner, activation }),
  );
  let journal;
  try {
    journal = JSON.parse(await readFile(journalPath, "utf8"));
  } catch (error) {
    if (error.code !== "ENOENT") throw error;
    journal = { binding, entries: [] };
  }
  assert.deepEqual(
    journal.binding,
    JSON.parse(JSON.stringify(binding)),
    "journal belongs to another probe binding",
  );
  const save = async () => {
    await writeFile(`${journalPath}.next`, JSON.stringify(journal), {
      mode: 0o600,
    });
    await rename(`${journalPath}.next`, journalPath);
  };
  await save();
  const call = async (args) => {
    let stdout = "";
    const rc = await callRunner(args, {
      env: {
        ...env,
        COMPUTE_EXECUTOR: "cid-executor",
        RUNNER_ROOT: join(out, "private-runner"),
      },
      execute: nameArtifact,
      stdout: (chunk) => {
        stdout += chunk;
      },
    });
    assert.equal(rc, 0, `runner ${args[0]} failed`);
    return JSON.parse(stdout);
  };
  const grantState = async (grant, recipient, wanted) => {
    const deadline = Date.now() + propagationMs;
    for (;;) {
      let state;
      try {
        state = await call(["grant-status", grant.grantActionHash]);
      } catch {
        /* DHT propagation */
      }
      if (state?.state === wanted) {
        assert.equal(state.grantActionHash, grant.grantActionHash);
        assert.equal(state.cid, grant.grantCid);
        assert.equal(state.provider, config.provider);
        assert.equal(state.recipient, recipient);
        assert.equal(state.scope, "measure-stage");
        assert.equal(state.runner, requester);
        return;
      }
      assert(Date.now() < deadline, `signed ${wanted} grant unavailable`);
      await pause(2000);
    }
  };
  const evidence = [];
  let failure;
  try {
    for (const kind of ["wrong-recipient", "withdrawn"]) {
      const recipient = kind === "wrong-recipient" ? otherRunner : requester;
      let entry = journal.entries.find((e) => e.kind === kind);
      if (!entry) {
        // RFC3339 nanoseconds are retained by the native grant codec. Pin once
        // before mutation: retries use identical content, not a refreshed window.
        const issuedAt = `${new Date(now - 2000).toISOString().slice(0, 19)}.${String(randomInt(1000000000)).padStart(9, "0")}Z`;
        entry = {
          kind,
          declaration: {
            recipient,
            scope: "measure-stage",
            issuedAt,
            validFrom: new Date(now - 3000).toISOString(),
            validUntil: new Date(now + 900000).toISOString(),
            bounds: {
              epr_scope: ["*"],
              reach_ceiling: "commons",
              rate_per_hour: 2,
              rotation_ttl_days: 1,
            },
          },
        };
        journal.entries.push(entry);
        await save();
      }
      if (entry.evidence) {
        const recorded = entry.evidence;
        assert(
          Number.isFinite(Date.parse(recorded.observedAt)),
          "missing original denial observation time",
        );
        assert.equal(recorded.kind, kind);
        assert.equal(recorded.recipient, recipient);
        assert.equal(recorded.grantCid, entry.grant.grantCid);
        assert.equal(recorded.grantActionHash, entry.grant.grantActionHash);
        const current = await provider(
          "GET",
          `/api/v1/compute/tasks/${entry.submitted.requestActionHash}`,
        );
        assert.equal(current.status, 200);
        for (const field of [
          "requestActionHash",
          "taskCid",
          "grantActionHash",
          "requester",
          "provider",
        ]) {
          assert.equal(recorded[field], entry.submitted[field]);
          assert.equal(current.body[field], recorded[field]);
        }
        assert.equal(recorded.requester, requester);
        assert.equal(recorded.provider, config.provider);
        assert(
          !current.body.acceptance && !current.body.completion,
          "unauthorized task acquired execution evidence",
        );
        const admission = await provider(
          "GET",
          `/api/v1/economic-events/compute-admission:${recorded.requestActionHash}`,
        );
        assert.equal(admission.status, 404, "unauthorized admission exists");
        evidence.push({ ...recorded, verifiedAt: new Date().toISOString() });
        continue;
      }
      assert(
        !entry.cleaned,
        "previous failed probe cleaned; retain journal and use a fresh output directory",
      );
      let grant = entry.grant;
      if (!grant) {
        // A response lost after native issuance is recovered by exact reissue.
        const issued = await provider(
          "POST",
          "/api/v1/compute/grants",
          entry.declaration,
        );
        assert.equal(issued.status, 200);
        grant = issued.body;
      }
      assert.notEqual(
        grant.grantActionHash,
        config.runnerGrant,
        "production grant collision",
      );
      assert.match(grant.grantCid, /^uhCEk[A-Za-z0-9_-]{48}$/);
      assert.match(grant.grantActionHash, /^uhCkk[A-Za-z0-9_-]{48}$/);
      assert.equal(grant.provider, config.provider);
      assert.equal(grant.recipient, recipient);
      assert.equal(grant.state, "active");
      entry.grant = grant;
      await save();
      // Cleanup only exact identities returned by this uniquely scoped issuance.
      if (kind === "withdrawn") {
        const revoked = await provider("POST", "/api/v1/compute/grants", {
          grantCid: grant.grantCid,
          revoke: true,
        });
        assert.equal(revoked.status, 200);
        await grantState(grant, recipient, "withdrawn");
      } else await grantState(grant, recipient, "active");
      const stage =
        entry.stage ||
        (await build({
          ...config,
          requester,
          feature: config.feature,
          out: join(out, kind),
          sourceRepoRoot: root,
          a2oDir: join(root, "genesis/a2o"),
          executor: "cid-executor",
          execute: nameArtifact,
          probeRun: false,
          env,
        }));
      entry.stage = stage;
      await save();
      const submitted =
        entry.submitted ||
        (await call([
          "submit",
          stage.taskPath,
          stage.runnerPath,
          stage.dnaPath,
          "--grant",
          grant.grantActionHash,
          "--provider",
          config.provider,
        ]));
      assert.match(submitted.requestActionHash, /^uhCkk[A-Za-z0-9_-]{48}$/);
      assert.equal(submitted.grantActionHash, grant.grantActionHash);
      assert.equal(submitted.requester, requester);
      assert.equal(submitted.provider, config.provider);
      entry.submitted = submitted;
      await save();
      const path = `/api/v1/compute/tasks/${encodeURIComponent(submitted.requestActionHash)}`;
      const deadline = Date.now() + 120000;
      let seen;
      for (;;) {
        seen = await provider("GET", path);
        if (seen.status === 200) break;
        assert(
          [404, 500].includes(seen.status) && Date.now() < deadline,
          "provider did not observe request",
        );
        await pause(2000);
      }
      const check = (task) => {
        assert.equal(task.requestActionHash, submitted.requestActionHash);
        assert.equal(task.taskCid, submitted.taskCid);
        assert.equal(task.grantActionHash, grant.grantActionHash);
        assert.equal(task.requester, requester);
        assert.equal(task.provider, config.provider);
        assert(
          !task.acceptance && !task.completion,
          "unauthorized task acquired execution evidence",
        );
      };
      check(seen.body);
      const attemptId = randomUUID();
      const withinDeclaredWindow = () => {
        const instant = Date.now();
        assert(
          instant >= Date.parse(entry.declaration.validFrom) &&
            instant < Date.parse(entry.declaration.validUntil),
          "disposable grant validity window elapsed; do not renew this journal",
        );
      };
      for (const operation of ["accept", "authorize-launch"]) {
        withinDeclaredWindow();
        const response = await provider("POST", `${path}/${operation}`, {
          attemptId,
        });
        withinDeclaredWindow();
        assert.equal(response.status, 403, `${kind} ${operation}`);
        assert.equal(response.body.reason, "compute-grant-refused");
      }
      const observedAt = new Date().toISOString();
      const final = await provider("GET", path);
      assert.equal(final.status, 200);
      check(final.body);
      check(await call(["get", submitted.requestActionHash]));
      // The native economic-events handler passes this path suffix verbatim to
      // its DB lookup; percent-encoding ':' would query a different identifier.
      const admission = await provider(
        "GET",
        `/api/v1/economic-events/compute-admission:${submitted.requestActionHash}`,
      );
      assert.equal(admission.status, 404, "unauthorized admission exists");
      entry.evidence = {
        observedAt,
        verifiedAt: new Date().toISOString(),
        kind,
        provider: config.provider,
        requester,
        recipient,
        grantActionHash: grant.grantActionHash,
        grantCid: grant.grantCid,
        requestActionHash: submitted.requestActionHash,
        taskCid: submitted.taskCid,
        acceptStatus: 403,
        launchStatus: 403,
        reason: "compute-grant-refused",
        admissionStatus: 404,
      };
      evidence.push(entry.evidence);
      await save();
    }
  } catch (error) {
    failure = error;
  }
  const cleanup = [];
  for (const entry of journal.entries.filter((e) => e.grant && !e.cleaned)) {
    const grant = entry.grant;
    try {
      const response = await provider("POST", "/api/v1/compute/grants", {
        grantCid: grant.grantCid,
        revoke: true,
      });
      assert.equal(response.status, 200);
      await grantState(grant, entry.declaration.recipient, "withdrawn");
      entry.cleaned = true;
      await save();
    } catch (error) {
      cleanup.push(error);
    }
  }
  if (failure || cleanup.length)
    throw new AggregateError(
      [...(failure ? [failure] : []), ...cleanup],
      "authority probe or disposable cleanup failed",
    );
  return {
    claim:
      "wrong-recipient and already-withdrawn grants refused at Adam acceptance and launch endpoints",
    evidence,
  };
}
