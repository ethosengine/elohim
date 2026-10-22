// One-shot Jenkins authority proof; execute with the installed a2o tsx binary.
import assert from "node:assert/strict";
import { withClusterTransport } from "./runner-cluster-transport.mjs";
import { execFileSync } from "node:child_process";
import {
  readFileSync,
  writeFileSync,
  chmodSync,
  mkdirSync,
  mkdtempSync,
  rmSync,
} from "node:fs";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";

export function captureEnv(worker, env) {
  assert.equal(env.RUN_ADAM_AUTHORITY_PROBE, "true");
  assert(env.ADMIN_KEY && env.DOORWAY_URL);
  assert.equal(worker.enabled, true);
  assert.equal(worker.runnerGrant?.enabled, true);
  assert.equal(worker.runnerGrant.stewardOfRecord, "human:matthew");
  assert.equal(worker.runnerGrant.fixtureProvider, "adam");
  const { provider, recipient } = worker.runnerGrant;
  for (const key of [provider, recipient])
    assert(/^uhCAk[A-Za-z0-9_-]{48}$/.test(key));
  assert.equal(worker.performer, provider);
  assert.notEqual(provider, recipient);
  assert(/^[a-zA-Z0-9./:_-]+@sha256:[a-f0-9]{64}$/.test(worker.image));
  for (const n of [worker.slice?.cpuMillis, worker.slice?.memoryBytes])
    assert(Number.isSafeInteger(n) && n > 0);
  for (const value of [env.PROBE_WORKLOAD, env.PROBE_NAMESPACE])
    assert(/^[a-z0-9][a-z0-9-]*$/.test(value || ""));
  return {
    ...env,
    COMPUTE_WORKER_ENABLED: "true",
    COMPUTE_WORKER_GRANT_ENABLED: "true",
    COMPUTE_WORKER_PROVIDER: provider,
    COMPUTE_WORKER_RECIPIENT: recipient,
    COMPUTE_WORKER_IMAGE: worker.image,
    COMPUTE_WORKER_CPU_MILLIS: String(worker.slice.cpuMillis),
    COMPUTE_WORKER_MEMORY_BYTES: String(worker.slice.memoryBytes),
    COMPUTE_WORKER_STARTUP_SECS: "60",
    RUNNER_IDENTIFIER: "jenkins-adam-authority-probe@alpha.elohim.host",
    RUNNER_SERVICE_AGENT_ID: "agent:ci-runner-jenkins-adam-authority-probe",
  };
}

async function main() {
  process.umask(0o077);
  const root = resolve(process.env.WORKSPACE || ".");
  const publicFile = join(root, "adam-authority-probe.json");
  const publicActivation = join(root, "adam-authority-activation.json");
  rmSync(publicFile, { force: true });
  rmSync(publicActivation, { force: true });
  const publicStatus = join(root, "adam-authority-status.txt");
  rmSync(publicStatus, { force: true });
  const worker = JSON.parse(
    readFileSync(
      join(root, "genesis/orchestrator/data/adam-compute-worker.json"),
    ),
  );
  const approvedEnv = captureEnv(worker, process.env); // Guard before registration or grant mutation.
  await withClusterTransport(
    async (env) => {
      const profilePath = join(
        root,
        "genesis/orchestrator/data/jenkins-peer-stage.json",
      );
      const profile = JSON.parse(readFileSync(profilePath));
      const { validateConfig } = await import(
        pathToFileURL(join(root, "genesis/scripts/ci/runner-stage.mjs"))
      );
      // Native action hash is read from activation below, never invented here.
      assert.equal(profile.provider, worker.performer);
      assert.equal(profile.runtimeImage, worker.image.split("@")[1]);
      assert.equal(profile.cpuMillis, worker.slice.cpuMillis);
      assert.equal(profile.memoryBytes, worker.slice.memoryBytes);
      const home = join(root, ".eprfs/status/adam-authority-probe");
      mkdirSync(home, { recursive: true, mode: 0o700 });
      const campaign = mkdtempSync(join(home, "attempt-")); // No cross-build resume or automatic retry.
      const capture = join(campaign, "capture");
      execFileSync(
        process.execPath,
        [
          join(root, "genesis/scripts/ci/runner-capture-activation.mjs"),
          env.PROBE_WORKLOAD,
          env.PROBE_NAMESPACE,
          capture,
        ],
        { env, stdio: ["ignore", "inherit", "inherit"] },
      );
      const activationPath = join(
        capture,
        `${env.PROBE_NAMESPACE}--${env.PROBE_WORKLOAD}`,
        "activation.json",
      );
      const activation = JSON.parse(readFileSync(activationPath));
      validateConfig(
        { ...profile, runnerGrant: activation.grantActionHash },
        profile.feature,
      );
      // This capture contains selected public evidence only, never the private journal.
      writeFileSync(
        publicActivation,
        JSON.stringify(activation, null, 2) + "\n",
        { mode: 0o644 },
      );
      chmodSync(publicActivation, 0o644); // Parent deliberately uses private umask077.
      // Suppress child errors as well as stdout: failed subprocess objects can include secrets.
      env.RUNNER_JWT = execFileSync(
        "bash",
        [join(root, "genesis/scripts/ci/runner-ensure.sh")],
        { env, encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
      ).trim();
      assert(env.RUNNER_JWT);
      execFileSync(
        join(root, "genesis/a2o/node_modules/.bin/tsx"),
        [
          join(root, "genesis/scripts/ci/runner-authority-probe-cli.mjs"),
          profilePath,
          activationPath,
          campaign,
          publicFile,
        ],
        { env, stdio: ["ignore", "inherit", "inherit"] },
      );
      writeFileSync(publicStatus, "READY: authority refusal probe verified\n", {
        mode: 0o644,
      });
      chmodSync(publicStatus, 0o644);
    },
    { env: approvedEnv },
  );
}
if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  main().catch(() => {
    try {
      const status = resolve(
        process.env.WORKSPACE || ".",
        "adam-authority-status.txt",
      );
      writeFileSync(
        status,
        "REFUSED: authority proof incomplete; no automatic retry\n",
        { mode: 0o644 },
      );
      chmodSync(status, 0o644);
    } catch {
      /* Preserve original failure if the workspace is unavailable. */
    }
    console.error(
      "Authority proof refused or failed. No automatic retry; interrupted temporary grants expire after 15 minutes. A new attempt requires a fresh explicit tagged push.",
    );
    process.exitCode = 1;
  });
}
