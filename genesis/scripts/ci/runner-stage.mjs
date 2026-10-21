#!/usr/bin/env node
// One opt-in peer stage. Run with genesis/a2o/node_modules/.bin/tsx.
// RUNNER_STAGE_FEATURE names one feature; RUNNER_STAGE_CONFIG names a reviewed JSON file:
// {provider, runnerGrant, repoRoot, nodeBin, justBin, writablePaths, runtimeImage,
//  cpuMillis, memoryBytes, timeoutSeconds}. Resources must match the provider's real slice.
// Paths describe the provider's checkout/tools. Secrets stay in ADMIN_KEY / RUNNER_JWT.
import { execFile } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { isAbsolute, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";
import { promisify } from "node:util";
import { isDeepStrictEqual } from "node:util";
import { main as runner } from "../../a2o/scripts/compute-runner.ts";
import {
  execute as nameArtifact,
  jsonCid,
} from "../../agentic/compute/cid-executor.mjs";
import { buildStageTask } from "../../agentic/compute/stage/build-stage-task.mjs";
import {
  decodeFramedReport,
  stageVerdict,
} from "../../agentic/compute/stage/collect-stage.mjs";

const exec = promisify(execFile);
const insist = (condition, why) => {
  if (!condition) throw new Error(why);
};
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");

export function validateConfig(config, feature) {
  insist(
    /^features\/[a-zA-Z0-9_./-]+\.feature$/.test(feature || "") &&
      !feature.split("/").includes(".."),
    "name one repository feature, not tags or a traversal",
  );
  for (const name of [
    "provider",
    "runnerGrant",
    "repoRoot",
    "nodeBin",
    "justBin",
    "runtimeImage",
  ])
    insist(
      typeof config[name] === "string" && config[name].length > 0,
      `missing provider ${name}`,
    );
  insist(
    /^sha256:[0-9a-f]{64}$/.test(config.runtimeImage),
    "provider runtimeImage must be a pinned sha256 digest",
  );
  insist(
    Array.isArray(config.writablePaths) && config.writablePaths.length > 0,
    "declare provider writablePaths",
  );
  // The builder renders these paths into a shell template: refuse shell syntax at this boundary.
  for (const path of [
    config.repoRoot,
    config.nodeBin,
    config.justBin,
    ...config.writablePaths,
  ])
    insist(
      typeof path === "string" &&
        isAbsolute(path) &&
        /^[a-zA-Z0-9_./-]+$/.test(path),
      "provider paths must be absolute simple paths",
    );
  for (const name of ["cpuMillis", "memoryBytes", "timeoutSeconds"])
    insist(
      Number.isSafeInteger(config[name]) && config[name] > 0,
      `declare a positive integer ${name} matching the provider slice`,
    );
  insist(
    config.timeoutSeconds <= 3600,
    "stage timeout exceeds the bounded CI hour",
  );
  return config;
}

export function verifyGrant(grant, config, requester) {
  insist(
    grant.state === "active" &&
      grant.forThisRunner === true &&
      grant.scope === "measure-stage" &&
      grant.provider === config.provider &&
      grant.grantActionHash === config.runnerGrant &&
      grant.recipient === grant.runner &&
      (!requester || grant.runner === requester),
    "runner has no active measure-stage grant from this provider",
  );
  return grant.runner;
}

/** The task is read through the runner's signed hosted-cell session, not a local fixture.
 * Recheck CIDs and bindings here because the hosted path bypasses storage's verify_status.
 */
export async function verifyCompletion({
  task,
  submitted,
  envelope,
  stage,
  stdout,
  stderr = Buffer.alloc(0),
}) {
  insist(
    task.requestActionHash === submitted.requestActionHash &&
      task.taskCid === submitted.taskCid,
    "completion is for a different request/task",
  );
  insist(
    (await jsonCid(envelope)) === task.taskCid &&
      isDeepStrictEqual(task.envelope, envelope),
    "completed envelope differs from submitted stage",
  );
  insist(
    !task.refusal && task.acceptance && task.completion,
    "provider refused or has not completed this stage",
  );
  const receipt = task.completion.receipt;
  insist(
    receipt && (await jsonCid(receipt)) === task.completion.receiptCid,
    "receipt CID mismatch",
  );
  for (const [field, expected] of Object.entries({
    taskCid: task.taskCid,
    requestAction: task.requestActionHash,
    grantAction: task.grantActionHash,
    provider: envelope.provider,
    requester: envelope.requester,
  }))
    insist(
      receipt[field] === expected && expected,
      `receipt ${field} binding mismatch`,
    );
  insist(
    task.grantActionHash === submitted.grantActionHash,
    "completion spent another grant",
  );
  for (const field of [
    "binary",
    "dna",
    "runtimeImage",
    "expectedTests",
    "project",
    "taskKind",
    "retention",
  ])
    insist(
      isDeepStrictEqual(receipt[field], envelope[field]),
      `receipt ${field} differs from stage`,
    );
  insist(
    receipt.schemaVersion === 1 &&
      ["passed", "failed"].includes(receipt.status),
    `provider did not finish tests: ${receipt.status}`,
  );
  for (const [name, bytes] of [
    ["stdout.log", stdout],
    ["stderr.log", stderr],
  ]) {
    const descriptors = receipt.logs?.filter((log) => log.name === name) ?? [];
    insist(
      descriptors.length === 1 &&
        descriptors[0].sha256 === digest(bytes) &&
        descriptors[0].bytes === bytes.length,
      `${name} does not match the receipt`,
    );
  }
  insist(
    !stderr.toString().includes("STAGE-PRECONDITION-UNMET"),
    "provider stage preconditions were unmet",
  );
  const framed = decodeFramedReport(stdout.toString());
  insist(
    framed &&
      Array.isArray(framed.doc) &&
      framed.doc.length === 1 &&
      framed.doc[0].uri === stage.feature,
    "report must name exactly the pinned feature",
  );
  const scenarios = (framed.doc[0].elements || []).filter(
    (e) => e.type === "scenario",
  );
  insist(
    scenarios.length === stage.scenarioNames.length &&
      new Set(scenarios.map((s) => s.name)).size === scenarios.length &&
      scenarios.every(
        (s) =>
          s.steps?.length > 0 &&
          s.steps.every((step) =>
            [
              "passed",
              "failed",
              "skipped",
              "pending",
              "undefined",
              "ambiguous",
            ].includes(step.result?.status),
          ),
      ),
    "report is empty, duplicated, or has steps without outcomes",
  );
  const result = stageVerdict(framed.doc, stage);
  insist(result.verdict !== "skip", result.reason || "stage verdict refused");
  insist(!result.reason, result.reason);
  if (result.verdict === "pass") {
    insist(
      scenarios.every((s) =>
        [...s.steps, ...(s.before || []), ...(s.after || [])].every(
          (step) => step.result?.status === "passed",
        ),
      ),
      "passing report contains unfinished steps or failed hooks",
    );
    insist(
      receipt.status === "passed" &&
        receipt.exitCode === 0 &&
        isDeepStrictEqual(
          [...receipt.observedTests].sort(),
          [...envelope.expectedTests].sort(),
        ),
      "passing report disagrees with executor completion/inventory",
    );
  }
  return { ...result, report: framed.buffer };
}

export async function runStage({
  env = process.env,
  root = resolve(import.meta.dirname, "../../.."),
  callRunner = runner,
  build = buildStageTask,
  ensure,
} = {}) {
  const report = join(
    root,
    "genesis/a2o/reports/cucumber-report-peer-stage.json",
  );
  // A workspace may be reused; an old success must never survive a refusal.
  await rm(report, { force: true });
  const config = validateConfig(
    JSON.parse(await readFile(env.RUNNER_STAGE_CONFIG, "utf8")),
    env.RUNNER_STAGE_FEATURE,
  );
  const out = join(root, "genesis/a2o/reports/peer-stage-ci");
  await mkdir(out, { recursive: true });
  const jwt =
    env.RUNNER_JWT ||
    (ensure
      ? await ensure()
      : (
          await exec(
            "bash",
            [join(root, "genesis/scripts/ci/runner-ensure.sh")],
            { env },
          )
        ).stdout.trim());
  insist(jwt, "runner session is missing");
  const runnerEnv = {
    ...env,
    RUNNER_JWT: jwt,
    COMPUTE_EXECUTOR: "cid-executor",
    RUNNER_ROOT: join(root, ".eprfs/status/ci-runner"),
  };
  const call = async (args) => {
    let output = "";
    const code = await callRunner(args, {
      env: runnerEnv,
      execute: nameArtifact,
      stdout: (chunk) => {
        output += chunk;
      },
    });
    insist(code === 0, `runner ${args[0]} failed`);
    return JSON.parse(output);
  };
  const requester = verifyGrant(
    await call(["grant-status", config.runnerGrant]),
    config,
  );
  const built = await build({
    ...config,
    feature: env.RUNNER_STAGE_FEATURE,
    requester,
    out,
    sourceRepoRoot: root,
    a2oDir: join(root, "genesis/a2o"),
    execute: nameArtifact,
    executor: "cid-executor",
    probeRun: false,
    // Provider runtime controls its own fixture/capability environment. CI never invents one.
    env,
  });
  insist(
    built.sut && built.sut !== "sha256:e3b0c44298fc1c14",
    "requester could not identify its source under test",
  );
  const submitted = await call([
    "submit",
    built.taskPath,
    built.runnerPath,
    built.dnaPath,
    "--grant",
    config.runnerGrant,
    "--provider",
    config.provider,
  ]);
  insist(
    submitted.requestActionHash &&
      submitted.taskCid &&
      submitted.grantActionHash === config.runnerGrant,
    "submission did not return the requested grant/task identity",
  );
  await writeFile(
    join(out, "submission.json"),
    JSON.stringify(submitted, null, 2),
  );
  const polled = await call([
    "poll",
    submitted.requestActionHash,
    "--timeout-seconds",
    String(config.timeoutSeconds + 600),
    "--out",
    out,
  ]);
  await writeFile(
    join(out, "completion.json"),
    JSON.stringify(polled.task, null, 2),
  );
  insist(
    ["stdout.log", "stderr.log"].every((name) =>
      polled.logPaths?.includes(join(out, name)),
    ),
    "provider completion did not materialize both logs for this request",
  );
  verifyGrant(
    await call(["grant-status", config.runnerGrant]),
    config,
    requester,
  );
  // Submission adds nonce and chunk descriptors; compare the exact published envelope.
  const envelope = JSON.parse(await readFile(built.taskPath, "utf8"));
  const stage = JSON.parse(await readFile(built.stagePath, "utf8"));
  const result = await verifyCompletion({
    task: polled.task,
    submitted,
    envelope,
    stage,
    stdout: await readFile(join(out, "stdout.log")),
    stderr: await readFile(join(out, "stderr.log")),
  });
  await writeFile(report, result.report);
  await writeFile(
    join(out, "verdict.json"),
    JSON.stringify(
      { verdict: result.verdict, scenarios: result.scenarios },
      null,
      2,
    ),
  );
  return result.verdict === "pass" ? 0 : 1;
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  runStage().then(
    (code) => {
      process.exitCode = code;
    },
    (error) => {
      process.stderr.write(`REFUSED peer stage: ${error.message}\n`);
      process.exitCode = 2;
    },
  );
}
