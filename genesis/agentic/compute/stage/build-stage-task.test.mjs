import test from "node:test";
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { createHash } from "node:crypto";
import { join } from "node:path";
import {
  A2O_DIR,
  buildStageTask,
  parseScenarioNames,
  slugify,
} from "./build-stage-task.mjs";

const EXECUTOR =
  process.env.COMPUTE_EXECUTOR ||
  "/projects/.cargo-target-pool/family/dev/elohim__rakia/dev/debug/compute-executor";
const FEATURE = "features/dataplane/federation-version-convergence.feature";
const REQUESTER = "uhCAkTestRequesterKey";
const PROVIDER = "uhCAkTestProviderKey";
const DECLARED_LITERAL =
  "feedback_signal::a2o::dataplane::federation-version-convergence::two-doorways-that-disagree-about-a-page-converge-on-the-elected-version-without-anyone-re-upload";

test("build-stage-task builds a valid single-scenario stage envelope", async (t) => {
  const out = await mkdtemp(join(tmpdir(), "stage-build-"));
  t.after(() => rm(out, { recursive: true, force: true }));

  const result = await buildStageTask({
    feature: FEATURE,
    requester: REQUESTER,
    provider: PROVIDER,
    out,
    executor: EXECUTOR,
  });

  assert.equal(result.expectedTests.length, 1, "exactly one scenario in this feature");
  assert.match(result.expectedTests[0], /feedback_signal/);
  assert.equal(
    result.expectedTests[0],
    DECLARED_LITERAL,
    "slug must equal the literal pinned in §3 D6",
  );

  const featureBytes = await readFile(join(A2O_DIR, FEATURE));
  const featureSha256 = createHash("sha256").update(featureBytes).digest("hex");
  assert.equal(result.envelope.dna.sha256, featureSha256);
  assert.equal(result.envelope.binary.bytes > 0, true);

  const envelopeBytes = Buffer.byteLength(JSON.stringify(result.envelope));
  assert.equal(envelopeBytes < 65536, true, "envelope must stay under 64 KiB");

  // §4 constraint 1 / §9 task 1's single most valuable check: `run`'s typed parser must
  // accept the envelope shape and reach PAST task parsing to the runtime-image gate —
  // not fail with `unknown field` or `unsupported task`.
  assert.equal(result.probe.ok, true);
  assert.match(result.probe.message, /runtime image identity unavailable or mismatched/);
});

test("slugify matches the verbatim §3 D6 expression on the pinned scenario name", () => {
  const scenario =
    "two doorways that disagree about a page converge on the elected version without anyone re-uploading it";
  assert.equal(
    slugify(scenario),
    "two-doorways-that-disagree-about-a-page-converge-on-the-elected-version-without-anyone-re-upload",
  );
});

test("parseScenarioNames finds Scenario: and Scenario Outline: lines without a Gherkin parser", () => {
  const text = [
    "Feature: x",
    "  Background:",
    "    Given y",
    "  Scenario: first one",
    "    Given z",
    "  Scenario Outline: second <one>",
    "    Given w",
  ].join("\n");
  assert.deepEqual(parseScenarioNames(text), ["first one", "second <one>"]);
});
