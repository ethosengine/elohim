#!/usr/bin/env node
/**
 * The stage builder — see
 * genesis/docs/superpowers/specs/2026-09-08-peer-executed-stage-design.md §8 Task 1.
 *
 * Builds a content-addressed peer-executed a2o stage: the `binary` slot is a generated
 * shell script that speaks the pinned rakia executor's libtest protocol (D4); the `dna`
 * slot carries the feature file's own bytes, byte-identical (D5). Both are wired into a
 * compute-task.schema.json envelope whose only free string, `project`, carries the stage
 * identity (D2).
 *
 * Usage:
 *   build-stage-task.mjs --feature <path-relative-to-genesis/a2o> --requester <key>
 *     --provider <key> --out <dir> [--runtime-image sha256:<64 hex>] [--executor <path>]
 */
import { chmod, copyFile, mkdir, readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";
import { run } from "../common.mjs";

const HERE = dirname(fileURLToPath(import.meta.url));
export const A2O_DIR = resolve(HERE, "..", "..", "..", "a2o");
export const REPO_ROOT = resolve(A2O_DIR, "..", "..");
const TEMPLATE_PATH = join(HERE, "stage-runner.template.sh");

// Verbatim slug expression, §3 D6. Any drift here breaks the two-sided computation the
// requester and the guest script must agree on without communicating.
export const slugify = (s) =>
  s
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 96)
    .replace(/-+$/, "");

// A `Scenario:` / `Scenario Outline:` line scan — deliberately not a Gherkin parser (§9 task 1).
export function parseScenarioNames(featureText) {
  const names = [];
  for (const line of featureText.split("\n")) {
    const match = line.match(/^\s*Scenario(?: Outline)?:\s*(.+?)\s*$/);
    if (match) names.push(match[1]);
  }
  return names;
}

export function parseArgs(argv) {
  const out = {};
  for (let index = 0; index < argv.length; index++) {
    const arg = argv[index];
    if (arg.startsWith("--")) {
      const name = arg.slice(2);
      const next = argv[index + 1];
      out[name] = next;
      index++;
    }
  }
  return out;
}

async function which(name, fallback) {
  const env = process.env.PATH || "/usr/bin:/bin";
  for (const dir of env.split(":")) {
    const candidate = join(dir, name);
    try {
      await readFile(candidate);
      return candidate;
    } catch {
      // not here; keep looking
    }
  }
  return fallback;
}

async function executorImageDigest(executor) {
  const path = executor.includes("/") ? executor : await which(executor, executor);
  return `sha256:${createHash("sha256").update(await readFile(path)).digest("hex")}`;
}

function shellQuote(value) {
  return `'${String(value).replace(/'/g, `'\\''`)}'`;
}

// §5.3 the capacity grant — default paths for this household mesh. A caller may override
// with `--writable <path>` (repeatable) to target a different provider layout.
export function defaultWritablePaths(repoRoot) {
  return [
    join(repoRoot, "genesis", "a2o", "reports"),
    "/tmp/elohim-local-mesh/matthew/runtime-config.toml",
    "/tmp/elohim-local-mesh/jessica/runtime-config.toml",
    "/tmp/elohim-local-mesh/james/runtime-config.toml",
  ];
}

/**
 * Builds one stage into `out`: stage-runner.sh (binary), <basename>.feature (dna), task.json.
 * Returns the emitted envelope object plus the derived stage metadata.
 */
export async function buildStageTask({
  feature,
  requester,
  provider,
  out,
  runtimeImage,
  executor = process.env.COMPUTE_EXECUTOR || "compute-executor",
  repoRoot = REPO_ROOT,
  a2oDir = A2O_DIR,
  nodeBin,
  justBin,
  writablePaths,
  execute = run,
}) {
  if (!feature || !requester || !provider || !out)
    throw new Error(
      "build-stage-task requires --feature --requester --provider --out",
    );

  const featureAbs = resolve(a2oDir, feature);
  const featureBytes = await readFile(featureAbs);
  const featureText = featureBytes.toString("utf8");
  const featureSha256 = createHash("sha256").update(featureBytes).digest("hex");

  const scenarioNames = parseScenarioNames(featureText);
  if (scenarioNames.length === 0)
    throw new Error(`no Scenario:/Scenario Outline: lines found in ${featureAbs}`);
  const scenarioSlugs = scenarioNames.map(slugify);

  // `feature` is relative to genesis/a2o (e.g. "features/dataplane/<name>.feature") — the
  // D6 grammar's <feature-dir> excludes the leading "features/" segment.
  const parts = feature.split("/");
  const dirParts = parts.slice(0, -1);
  if (dirParts[0] === "features") dirParts.shift();
  const featureFile = parts[parts.length - 1];
  const featureDir = dirParts.join("/");
  const featureSlug = featureFile.replace(/\.feature$/, "");
  const testPrefix = `feedback_signal::a2o::${featureDir}::${featureSlug}`;
  const expectedTests = expectedTestsFor(testPrefix, scenarioSlugs);

  await mkdir(out, { recursive: true });

  // 3. render stage-runner.template.sh -> <out>/stage-runner.sh, mode 0755
  const resolvedNodeBin = nodeBin || (await which("node", "/usr/local/bin/node"));
  const resolvedJustBin = justBin || (await which("just", "/usr/local/bin/just"));
  const writable = writablePaths || defaultWritablePaths(repoRoot);
  const featureRel = feature; // already relative to genesis/a2o, as the runner expects
  const template = await readFile(TEMPLATE_PATH, "utf8");
  const rendered = template
    .replaceAll("@@STAGE_NAME@@", featureSlug)
    .replaceAll("@@REPO_ROOT@@", repoRoot)
    .replaceAll("@@FEATURE_REL@@", featureRel)
    .replaceAll("@@FEATURE_SHA256@@", featureSha256)
    .replaceAll("@@TEST_PREFIX@@", testPrefix)
    .replaceAll(
      "@@SCENARIO_SLUGS@@",
      scenarioSlugs.map(shellQuote).join(" "),
    )
    .replaceAll("@@WRITABLE_PATHS@@", writable.map(shellQuote).join(" "))
    .replaceAll("@@NODE_BIN@@", resolvedNodeBin)
    .replaceAll("@@JUST_BIN@@", resolvedJustBin);
  const runnerPath = join(out, "stage-runner.sh");
  await writeFile(runnerPath, rendered, { mode: 0o755 });
  await chmod(runnerPath, 0o755);

  // 4. copy the feature file byte-identically
  const dnaPath = join(out, featureFile);
  await copyFile(featureAbs, dnaPath);

  // 5. compute-executor artifact on both, to fill binary/dna descriptors
  const binaryDescriptor = JSON.parse(
    await execute(executor, ["artifact", runnerPath]),
  );
  const dnaDescriptor = JSON.parse(
    await execute(executor, ["artifact", dnaPath]),
  );

  // 6. emit task.json exactly as §5.1. `runtimeImage` default: on a household peer there is
  // no container image, so the honest value (§5.1) is the digest of the executor binary itself.
  const resolvedRuntimeImage = runtimeImage || (await executorImageDigest(executor));
  const envelope = {
    schemaVersion: 1,
    taskKind: "feedback_signal",
    requester,
    provider,
    project: `a2o-stage:${featureSlug}`,
    runtimeImage: resolvedRuntimeImage,
    binary: binaryDescriptor,
    dna: dnaDescriptor,
    expectedTests,
    resources: {
      cpuMillis: 8000,
      memoryBytes: 8589934592,
      maxPayloadBytes: 8388608,
      timeoutSeconds: 2400,
    },
    // Never an explicit JSON null (§4 constraint 2) — compute-executor cid dies on Unit.
    retention: { maxAgeSeconds: 86400, maxRuns: 5, expireWhen: "either" },
  };
  const taskPath = join(out, "task.json");
  await writeFile(taskPath, JSON.stringify(envelope, null, 2) + "\n", {
    mode: 0o600,
  });

  // 7. validate with `run`'s parser, not `cid`'s (§4 constraint 1, §9 task 1's single most
  // valuable check): the probe must reach PAST task parsing to the runtime-image check.
  const probe = await probeRunParser({ taskPath, executor, execute });

  return {
    envelope,
    taskPath,
    runnerPath,
    dnaPath,
    featureSha256,
    featureSlug,
    featureDir,
    testPrefix,
    scenarioNames,
    scenarioSlugs,
    expectedTests,
    probe,
  };
}

export function expectedTestsFor(testPrefix, scenarioSlugs) {
  return scenarioSlugs.map((slug) => `${testPrefix}::${slug}`);
}

// §4 constraint 1 / §9 task 1: `cid` runs over serde_json::Value and tolerates a shape
// `run`'s typed contract::Task would refuse. Validating with `cid` alone would let a broken
// envelope pass this builder and fail only later, at launch, on the provider. This probe
// invokes `run` with throwaway binary/dna/root/ark/action values so parsing is exercised
// without needing a live provider; a well-formed envelope reaches the runtime-image check.
export async function probeRunParser({ taskPath, executor, execute = run }) {
  try {
    await execute(executor, [
      "run",
      "--task",
      taskPath,
      "--binary",
      "/bin/true",
      "--dna",
      "/dev/null",
      "--root",
      "/tmp/stage-builder-probe-root",
      "--ark",
      "/bin/false",
      "--request-action",
      "X",
      "--grant-action",
      "Y",
    ]);
    throw new Error(
      "compute-executor run unexpectedly succeeded against a throwaway probe",
    );
  } catch (error) {
    const message = error.message || "";
    if (/unknown field|unsupported task/.test(message)) {
      throw new Error(
        `envelope rejected before reaching the runtime-image gate: ${message}`,
      );
    }
    if (!/runtime image identity unavailable or mismatched/.test(message)) {
      throw new Error(`unexpected compute-executor run probe result: ${message}`);
    }
    return { ok: true, message };
  }
}

export async function main(argv = process.argv.slice(2)) {
  const args = parseArgs(argv);
  const result = await buildStageTask({
    feature: args.feature,
    requester: args.requester,
    provider: args.provider,
    out: args.out ? resolve(args.out) : undefined,
    runtimeImage: args["runtime-image"],
    executor: args.executor || process.env.COMPUTE_EXECUTOR || "compute-executor",
  });
  console.log(
    JSON.stringify(
      {
        taskPath: result.taskPath,
        runnerPath: result.runnerPath,
        dnaPath: result.dnaPath,
        project: result.envelope.project,
        expectedTests: result.expectedTests,
        probe: result.probe,
      },
      null,
      2,
    ),
  );
}

if (
  process.argv[1] &&
  import.meta.url === pathToFileURL(resolve(process.argv[1])).href
) {
  main().catch((error) => {
    console.error(error.message);
    process.exitCode = 1;
  });
}
