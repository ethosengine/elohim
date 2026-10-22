#!/usr/bin/env node
import assert from "node:assert/strict";
import { readFile, writeFile, chmod, mkdir, rm } from "node:fs/promises";
import { resolve, join } from "node:path";
import { pathToFileURL } from "node:url";
import { probeAuthority } from "./runner-authority-probe.mjs";
import { providerTransport } from "./runner-authority-transport.mjs";

// CLI: CONFIG ACTIVATION PRIVATE_JOURNAL_DIRECTORY PUBLIC_EVIDENCE_FILE
try {
  const [configPath, activationPath, privatePath, evidencePath, extra] =
    process.argv.slice(2);
  assert(configPath && activationPath && privatePath && evidencePath && !extra);
  await rm(evidencePath, { force: true });
  assert.equal(
    process.env.RUN_ADAM_AUTHORITY_PROBE,
    "true",
    "explicit one-shot probe opt-in required",
  );
  assert(
    process.env.RUNNER_JWT && process.env.ADMIN_KEY,
    "ensure a distinct probe service-agent session",
  );
  const root = resolve(
    process.env.PROBE_REPO_ROOT || join(import.meta.dirname, "../../.."),
  );
  const activation = JSON.parse(await readFile(activationPath, "utf8"));
  const config = JSON.parse(await readFile(configPath, "utf8"));
  assert(
    activation.declarationProvenance === "operator-approved-config" &&
      activation.stewardOfRecord === "human:matthew",
  );
  assert(
    activation.fixtureProvider === "adam" && activation.state === "active",
  );
  assert.equal(config.provider, activation.provider);
  assert.equal(config.runtimeImage, activation.worker.image.split("@")[1]);
  assert.equal(config.cpuMillis, activation.capacity.declaredCpuMillis);
  assert.equal(config.memoryBytes, activation.capacity.declaredMemoryBytes);
  assert(
    !config.runnerGrant || config.runnerGrant === activation.grantActionHash,
  );
  config.runnerGrant = activation.grantActionHash;
  const load = (path) => import(pathToFileURL(join(root, path)).href);
  const { main: runner } = await load("genesis/a2o/scripts/compute-runner.ts");
  const readProduction = async () => {
    let stdout = "";
    assert.equal(
      await runner(["grant-status", activation.grantActionHash], {
        env: process.env,
        stdout: (value) => {
          stdout += value;
        },
      }),
      0,
    );
    const status = JSON.parse(stdout);
    assert.equal(status.state, "active");
    assert.equal(status.provider, activation.provider);
    assert.equal(status.recipient, activation.recipient);
    assert.equal(status.cid, activation.grantCid);
    assert.equal(status.scope, "measure-stage");
    assert.notEqual(
      status.runner,
      activation.recipient,
      "probe must use its own service-agent identity",
    );
    return status.runner;
  };
  const requester = await readProduction();
  const out = resolve(privatePath);
  await mkdir(out, { recursive: true, mode: 0o700 });
  const provider = providerTransport(activation, undefined, config);
  const result = await probeAuthority({
    config,
    requester,
    otherRunner: activation.recipient,
    root,
    out,
    env: process.env,
    activation,
    provider,
  });
  assert.equal(await readProduction(), requester);
  await writeFile(
    evidencePath,
    JSON.stringify(
      {
        ...result,
        productionGrantUnchanged: true,
        verificationCompletedAt: new Date().toISOString(),
        worker: activation.worker,
      },
      null,
      2,
    ),
    { mode: 0o644 },
  );
  await chmod(evidencePath, 0o644); // Public whitelist must cross Jenkins container UIDs despite umask077.
  console.log("Adam authority refusal probe passed; public evidence written");
} catch {
  // Never echo a child error/response that may contain credentials.
  console.error(
    "Adam authority refusal probe failed; journal is private to this pod; no automatic retry",
  );
  process.exitCode = 1;
}
