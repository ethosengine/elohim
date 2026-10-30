// collect-stage's brit put outcome against the verified-read brit-build-ref contract (brit
// 7e4a83115e): the two refusals `validate put` can print are expected, warn-only, deferred
// outcomes that name the ref and the reason; every other non-zero exit stays the plain failure
// it was. The put goes through the lib's real `putAttestation` into a stub brit-build-ref script
// that prints the contract's exact first lines — the stubs live under the scratch dir
// (COLLECT_STAGE_SCRATCH, else the OS tmpdir), never in the tree.
import test from "node:test";
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { chmod, mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

import {
  attestationRef,
  checkNameFor,
  classifyPutRefusal,
} from "../../../a2o/scripts/lib/household-attestation.ts";
import { collectStageEvidence } from "./collect-stage.mjs";

const SCRATCH = process.env.COLLECT_STAGE_SCRATCH || tmpdir();

const CONCURRENT_LINE =
  "Error: REFUSED concurrent update: ref=refs/notes/brit/validate/a2o-household/moved%3Acheck " +
  "expected=0123456789abcdef0123456789abcdef01234567 " +
  "observed=89abcdef0123456789abcdef0123456789abcdef attempts=3";
const WOULD_NOT_VERIFY_LINE =
  "Error: refusing to write refs/notes/brit/validate/a2o-household/some%3Acheck: " +
  'signature does not verify against agentId "deadbeef"';
const GIT_FAILED_LINE =
  "Error: git command failed: git notes --ref refs/notes/brit/validate/a2o-household/x list: fatal: not a git repository";

// ─── fixtures (the same shapes compute.test.mjs builds; kept local so this file stays under
// stage/ beside the module it tests) ─────────────────────────────────────────────────────────

function cucumberReport({ uri, scenarioName, passed }) {
  return [
    {
      uri,
      elements: [
        {
          type: "scenario",
          name: scenarioName,
          tags: [],
          steps: [{ result: { status: passed ? "passed" : "failed", duration: 1000000 } }],
        },
      ],
    },
  ];
}

function framedStdout(reportDoc) {
  const encoded = Buffer.from(JSON.stringify(reportDoc)).toString("base64");
  return [
    "running 1 test",
    "-----BEGIN ELOHIM STAGE REPORT-----",
    encoded,
    "-----END ELOHIM STAGE REPORT-----",
    "",
  ].join("\n");
}

async function makeStageFixture(t) {
  await mkdir(SCRATCH, { recursive: true });
  const dir = await mkdtemp(join(SCRATCH, "collect-stage-logs-"));
  t.after(() => rm(dir, { recursive: true, force: true }));

  const stage = {
    feature: "features/dataplane/fixture.feature",
    featureSha256: "f".repeat(64),
    concern: "fixture-concern",
    sut: "sha256:0123456789abcdef",
    sutParts: { a2o: "tree:abc" },
    scenarioNames: ["fixture scenario"],
  };
  const stdoutText = framedStdout(
    cucumberReport({ uri: stage.feature, scenarioName: "fixture scenario", passed: true }),
  );
  const stdoutPath = join(dir, "stdout.log");
  await writeFile(stdoutPath, stdoutText);

  const entry = { requestActionHash: "req-hash-1", stage, rung: "H", logPaths: [stdoutPath] };
  const status = {
    requestActionHash: "req-hash-1",
    taskCid: "task-cid-1",
    provider: "uhCAkProvider",
    requester: "uhCAkRequester",
    grantActionHash: "grant-action-hash",
    envelope: { project: "a2o-stage:fixture", dna: { sha256: stage.featureSha256 } },
    completion: {
      actionHash: "completion-action-hash",
      receiptCid: "receipt-cid-1",
      receipt: {
        logs: [
          { name: "stdout.log", sha256: createHash("sha256").update(stdoutText).digest("hex") },
        ],
      },
    },
    observed: { verified: true, scope: "measure-stage", grantCid: "grant-cid-1" },
  };
  return { stage, entry, status };
}

/** A stub brit-build-ref under the scratch dir printing `stderrLine` and exiting `exit`. */
async function stubBrit(t, name, { stderrLine = "", exit = 0 }) {
  await mkdir(SCRATCH, { recursive: true });
  const dir = await mkdtemp(join(SCRATCH, `brit-stub-${name}-`));
  t.after(() => rm(dir, { recursive: true, force: true }));
  const bin = join(dir, "brit-build-ref");
  const body = stderrLine
    ? `#!/bin/sh\nprintf '%s\\n' ${shellQuote(stderrLine)} >&2\nexit ${exit}\n`
    : `#!/bin/sh\nexit ${exit}\n`;
  await writeFile(bin, body);
  await chmod(bin, 0o700);
  return bin;
}

function shellQuote(s) {
  return `'${s.replaceAll("'", `'\\''`)}'`;
}

async function collectWith(t, bin) {
  const cwd = await mkdtemp(join(SCRATCH, "collect-stage-cwd-"));
  t.after(() => rm(cwd, { recursive: true, force: true }));
  const { stage, entry, status } = await makeStageFixture(t);
  const logged = [];
  const delivered = [];
  const evidence = await collectStageEvidence({
    entry,
    status,
    root: cwd,
    cwd,
    env: { PATH: process.env.PATH },
    log: (line) => logged.push(line),
    runners: {
      // The real lib putAttestation runs through the default sync runner into the stub.
      resolveBinary: (name) => (name === "brit-build-ref" ? bin : null),
      gitCommonDir: () => join(cwd, ".git"),
      deliverStageResult: async (args) => {
        delivered.push(args);
        return { reportPath: null, fulfill: { skipped: true }, delta: { skipped: true } };
      },
    },
  });
  return { evidence, logged, delivered, stage, check: checkNameFor(stage.concern, stage.sut) };
}

// ─── classifyPutRefusal ──────────────────────────────────────────────────────────────────

test("classifyPutRefusal reads the two contract refusals from the first stderr line only", () => {
  const fallback = attestationRef("c:x");
  const concurrent = classifyPutRefusal(`${CONCURRENT_LINE}\n\nCaused by:\n  something`, fallback);
  assert.equal(concurrent.kind, "concurrent-update");
  assert.equal(concurrent.ref, "refs/notes/brit/validate/a2o-household/moved%3Acheck");
  assert.match(concurrent.reason, /^ref=.* attempts=3$/);
  assert.equal(concurrent.line, CONCURRENT_LINE);

  const wouldNot = classifyPutRefusal(WOULD_NOT_VERIFY_LINE, fallback);
  assert.equal(wouldNot.kind, "would-not-verify");
  assert.equal(wouldNot.ref, "refs/notes/brit/validate/a2o-household/some%3Acheck");
  assert.equal(wouldNot.reason, 'signature does not verify against agentId "deadbeef"');

  // A concurrent line that carries no ref= falls back to the lib's own name for the ref.
  const noRef = classifyPutRefusal("Error: REFUSED concurrent update: attempts=3", fallback);
  assert.equal(noRef.ref, fallback);

  assert.equal(classifyPutRefusal(GIT_FAILED_LINE, fallback), null);
  assert.equal(classifyPutRefusal("", fallback), null);
  assert.equal(classifyPutRefusal(undefined, fallback), null);
  // Only the FIRST line is read: a refusal buried below another line is not one.
  assert.equal(classifyPutRefusal(`${GIT_FAILED_LINE}\n${CONCURRENT_LINE}`, fallback), null);
});

// ─── collectStageEvidence against the stub ──────────────────────────────────────────────────

test("a REFUSED concurrent update is a deferred, warn-only attestation outcome naming ref and reason; the stage still passes and delivers", async (t) => {
  const bin = await stubBrit(t, "concurrent", { stderrLine: CONCURRENT_LINE, exit: 1 });
  const { evidence, logged, delivered, check } = await collectWith(t, bin);

  assert.equal(evidence.done, true);
  assert.equal(evidence.verdict, "pass");
  assert.equal(delivered.length, 1, "delivery still runs after a deferred put");
  assert.equal(evidence.attestation.ok, false);
  assert.equal(evidence.attestation.deferred, true);
  assert.equal(evidence.attestation.kind, "concurrent-update");
  assert.equal(evidence.attestation.ref, "refs/notes/brit/validate/a2o-household/moved%3Acheck");
  assert.match(evidence.attestation.reason, /expected=0123456789abcdef.* attempts=3$/);
  assert.equal(evidence.attestation.check, check);
  assert.equal(evidence.attestation.status, 1);
  assert.equal(evidence.attestation.stderr, CONCURRENT_LINE);
  assert.equal(logged.length, 1);
  assert.match(logged[0], /^validation attestation deferred for /);
  assert.ok(logged[0].includes(check));
  assert.ok(logged[0].includes("concurrent-update on refs/notes/brit/validate/a2o-household/moved%3Acheck"));
  assert.ok(logged[0].includes("attempts=3"));
});

test("a `refusing to write <ref>: <reason>` is a deferred, warn-only attestation outcome naming ref and reason", async (t) => {
  const bin = await stubBrit(t, "would-not-verify", { stderrLine: WOULD_NOT_VERIFY_LINE, exit: 1 });
  const { evidence, logged, delivered } = await collectWith(t, bin);

  assert.equal(evidence.verdict, "pass");
  assert.equal(delivered.length, 1);
  assert.equal(evidence.attestation.deferred, true);
  assert.equal(evidence.attestation.kind, "would-not-verify");
  assert.equal(evidence.attestation.ref, "refs/notes/brit/validate/a2o-household/some%3Acheck");
  assert.equal(
    evidence.attestation.reason,
    'signature does not verify against agentId "deadbeef"',
  );
  assert.equal(logged.length, 1);
  assert.ok(
    logged[0].includes(
      "would-not-verify on refs/notes/brit/validate/a2o-household/some%3Acheck): " +
        'signature does not verify against agentId "deadbeef"',
    ),
  );
});

test("any other non-zero exit stays the plain failure it was: no deferred key, no warning line", async (t) => {
  const bin = await stubBrit(t, "git-failed", { stderrLine: GIT_FAILED_LINE, exit: 1 });
  const { evidence, logged, delivered } = await collectWith(t, bin);

  assert.equal(evidence.verdict, "pass");
  assert.equal(delivered.length, 1);
  assert.deepEqual(evidence.attestation, { ok: false, status: 1, stderr: GIT_FAILED_LINE });
  assert.equal(logged.length, 0);
});

test("a put that exits 0 is the unchanged ok outcome", async (t) => {
  const bin = await stubBrit(t, "ok", { exit: 0 });
  const { evidence, logged, check, stage } = await collectWith(t, bin);
  assert.equal(evidence.attestation.ok, true);
  assert.equal(evidence.attestation.check, check);
  assert.equal(typeof evidence.attestation.artifact, "string");
  assert.equal("deferred" in evidence.attestation, false);
  assert.equal(logged.length, 0);
  assert.equal(stage.concern, "fixture-concern");
});
