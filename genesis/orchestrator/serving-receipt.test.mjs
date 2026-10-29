import { test } from "node:test";
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
import { spawnSync } from "node:child_process";
import { createPrivateKey, sign as edSign } from "node:crypto";
import { fileURLToPath } from "node:url";
import {
  createSutProbe,
  resolveComponent,
  normalizeSutIdentity,
  isGovernancePath,
  DEFAULT_SUT_COMPONENTS,
} from "../a2o/scripts/lib/sut.ts";
import {
  encodeValidationNode,
  sutArtifactCidBytes,
  sutOf,
  checkNameFor,
  RECEIPT_SCHEMA,
} from "../a2o/scripts/lib/household-attestation.ts";
import {
  validateReceipt,
  checkReports,
  requiredReceipts,
  peerRequestsOf,
  prefetchPeerStatuses,
  boundStatus,
  RECEIPT_STORIES,
} from "./scripts/serving-receipt.mjs";

const names = [
  "browser",
  "peer SSR",
  "doorway SSR",
  "recovery",
  "broken bundle",
];
const expected = {
  storage: "tree:storage",
  doorway: "tree:doorway",
  renderer: "tree:renderer",
  a2o: "tree:tests",
};
function receipt() {
  return {
    env: { lane: "household", processControl: true, sutParts: { ...expected } },
    summary: {
      byConcern: {
        "doorway-failover": {
          scenarios: names.map((name) => ({
            name,
            status: "passed",
            durationMs: 12,
            surface: "features/dataplane/epr-app-deliverability.feature",
          })),
        },
      },
    },
  };
}
test("accepts every measured station on the current household source", () =>
  assert.equal(validateReceipt(receipt(), expected, names), true));
for (const status of ["pending", "skipped", "failed", "undefined"]) {
  test(`refuses a ${status} SSR or browser station`, () => {
    const report = receipt();
    report.summary.byConcern["doorway-failover"].scenarios[2].status = status;
    assert.equal(validateReceipt(report, expected, names), false);
  });
}
test("refuses omitted browser, duplicate station, wrong feature and zero duration", () => {
  for (const change of [
    (s) => s.pop(),
    (s) => s.push(s[0]),
    (s) => {
      s[0].surface = "features/other.feature";
    },
    (s) => {
      s[0].durationMs = 0;
    },
  ]) {
    const report = receipt();
    change(report.summary.byConcern["doorway-failover"].scenarios);
    assert.equal(validateReceipt(report, expected, names), false);
  }
});
test("refuses different source, unknown source, and fleet receipts", () => {
  assert.equal(
    validateReceipt(receipt(), { ...expected, storage: "tree:new" }, names),
    false,
  );
  assert.equal(
    validateReceipt(receipt(), { ...expected, storage: null }, names),
    false,
  );
  const report = receipt();
  report.env.lane = "alpha-fleet";
  assert.equal(validateReceipt(report, expected, names), false);
});
test("renderer changes invalidate prior delivery receipts", () => {
  assert.ok(
    DEFAULT_SUT_COMPONENTS.some(
      (c) => c.name === "renderer" && c.path === "elohim/elohim-render",
    ),
  );
  assert.equal(
    validateReceipt(
      receipt(),
      { ...expected, renderer: "tree:new-renderer" },
      names,
    ),
    false,
  );
  const old = receipt();
  delete old.env.sutParts.renderer;
  assert.equal(validateReceipt(old, expected, names), false);
});
test("storage service alone cannot bypass mandatory receipt; caller file survives", () => {
  // Hook git environment may make a relative GIT_WORK_TREE follow the gate cwd.
  const root = fileURLToPath(new URL("../../", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "serving-receipt-"));
  try {
    const changed = join(dir, "changed");
    for (const path of [
      "elohim/elohim-storage/src/services/content_service.rs",
      "doorway/doorway-service/src/server/http.rs",
      "elohim/elohim-render/src/shim/node_builtins.rs",
      "elohim/elohim-storage/src/sync/projector.rs",
      "elohim/elohim-storage/src/routes/apps.rs",
      "genesis/a2o/scripts/browser-shell.ts",
      "genesis/a2o/scripts/verify-served-shell.ts",
      "genesis/a2o/scripts/lib/sut.ts",
    ]) {
      writeFileSync(changed, `${path}\n`);
      const result = spawnSync(
        "bash",
        [
          join(root, "genesis/orchestrator/scripts/t2-receipt.sh"),
          "--changed",
          changed,
          "--reports",
          dir,
        ],
        // Report files only: the real repo may hold a signed household attestation for the
        // current source (brit-build-ref on PATH writes one per receipt lane), which would
        // admit these paths and hide the refusal under test.
        {
          cwd: root,
          encoding: "utf8",
          env: { ...process.env, SERVING_RECEIPT_ATTESTATIONS: "0" },
        },
      );
      assert.equal(result.status, 1, path);
      assert.match(result.stderr, /NO-SERVING-RECEIPT/);
      assert.equal(existsSync(changed), true);
    }
    writeFileSync(changed, "app/unrelated.ts\n");
    assert.equal(
      spawnSync(
        "bash",
        [
          join(root, "genesis/orchestrator/scripts/t2-receipt.sh"),
          "--changed",
          changed,
          "--reports",
          dir,
        ],
        { cwd: root },
      ).status,
      0,
    );
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("default producer source identities accept a complete report despite newer unrelated reports", () => {
  // Hook git environment may make a relative GIT_WORK_TREE follow the gate cwd.
  const root = fileURLToPath(new URL("../../", import.meta.url));
  const dir = mkdtempSync(join(tmpdir(), "serving-valid-"));
  try {
    const feature = readFileSync(
      join(
        root,
        "genesis/a2o/features/dataplane/epr-app-deliverability.feature",
      ),
      "utf8",
    );
    const currentNames = [...feature.matchAll(/^\s*Scenario:\s*(.+)$/gm)].map(
      (m) => m[1].trim(),
    );
    const report = receipt();
    report.env.sutParts = Object.fromEntries(
      DEFAULT_SUT_COMPONENTS.filter((c) => c.path).map((c) => [
        c.name,
        resolveComponent(createSutProbe(root, {}), c),
      ]),
    );
    report.summary.byConcern["doorway-failover"].scenarios = currentNames.map(
      (name) => ({
        name,
        status: "passed",
        durationMs: 12,
        surface: "features/dataplane/epr-app-deliverability.feature",
      }),
    );
    writeFileSync(
      join(dir, "sprint-report-household-a.json"),
      JSON.stringify(report),
    );
    writeFileSync(join(dir, "sprint-report-household-z.json"), "{malformed");
    assert.equal(checkReports(root, dir), true);
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("actual pre-push caller propagates mandatory refusal in default and strict modes", () => {
  // Hook git environment may make a relative GIT_WORK_TREE follow the gate cwd.
  const root = fileURLToPath(new URL("../../", import.meta.url));
  const hook = readFileSync(join(root, ".husky/pre-push.bash"), "utf8");
  const start = hook.indexOf("# ── T2 receipt");
  const end = hook.indexOf("# ── Project filter", start);
  assert.ok(
    start >= 0 && end > start,
    "exercise the actual hook's receipt block",
  );
  const block = hook.slice(start, end);
  const dir = mkdtempSync(join(tmpdir(), "receipt-caller-"));
  try {
    mkdirSync(join(dir, "genesis/orchestrator/scripts"), { recursive: true });
    writeFileSync(
      join(dir, "genesis/orchestrator/scripts/t2-receipt.sh"),
      'printf "%s" "$2" > receipt-path\nexit "$RECEIPT_STATUS"\n',
    );
    for (const mode of ["", "strict"]) {
      for (const status of [0, 1, 2]) {
        const result = spawnSync(
          "bash",
          ["-c", `${block}printf caller-continued`],
          {
            cwd: dir,
            encoding: "utf8",
            env: {
              ...process.env,
              CHANGED: "doorway/doorway-service/src/server/http.rs",
              T2_RECEIPT: mode,
              RECEIPT_STATUS: String(status),
              TMPDIR: dir,
            },
          },
        );
        assert.equal(
          result.status,
          status === 0 ? 0 : 1,
          `mode=${mode || "default"}, receipt=${status}`,
        );
        assert.equal(result.stdout.includes("caller-continued"), status === 0);
        const list = readFileSync(join(dir, "receipt-path"), "utf8");
        assert.equal(
          existsSync(list),
          false,
          "caller cleans its temporary list on every outcome",
        );
      }
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

test("actual hook anchors Git identity before nested gates change directory", () => {
  const root = fileURLToPath(new URL("../../", import.meta.url));
  const hook = readFileSync(join(root, ".husky/pre-push.bash"), "utf8");
  const start = hook.indexOf("# ── Anchor Git context");
  const end = hook.indexOf("# ── Git context anchored", start);
  assert.ok(start >= 0 && end > start);
  const block = hook.slice(start, end);
  const dir = mkdtempSync(join(tmpdir(), "hook-git-context-"));
  const cleanEnv = Object.fromEntries(
    Object.entries(process.env).filter(([key]) => !key.startsWith("GIT_")),
  );
  try {
    const initialized = spawnSync("git", ["init", "--quiet", dir], {
      env: cleanEnv,
    });
    assert.equal(initialized.status, 0);
    mkdirSync(join(dir, "nested"));
    for (const gitEnv of [
      { GIT_DIR: join(dir, ".git") },
      { GIT_DIR: ".git", GIT_WORK_TREE: "." },
      {},
    ]) {
      const result = spawnSync(
        "bash",
        [
          "-c",
          `${block}
cd nested || exit 1
git rev-parse --show-toplevel
git rev-parse --absolute-git-dir
`,
        ],
        {
          cwd: dir,
          encoding: "utf8",
          env: { ...cleanEnv, ...gitEnv },
        },
      );
      assert.equal(result.status, 0, result.stderr);
      assert.deepEqual(result.stdout.trim().split("\n"), [
        dir,
        join(dir, ".git"),
      ]);
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// ── Lane C2: the not-ready-window story admits its own paths, strict ──────────
const REPO = fileURLToPath(new URL("../../", import.meta.url));
const REFUSES_FAST_PATHS = [
  "scripts/ci/fleet-write-readiness.sh",
  "scripts/ci/lib/readiness-faces.sh",
  "elohim/elohim-storage/src/services/release_adoption/mod.rs",
];
const NO_GIT_ENV = Object.fromEntries(
  Object.entries(process.env).filter(([k]) => !k.startsWith("GIT_")),
);

function featureNames(root, story) {
  const text = readFileSync(join(root, story.feature), "utf8");
  return [...text.matchAll(/^\s*Scenario:\s*(.+)$/gm)].map((m) => m[1].trim());
}

function currentParts(root) {
  const probe = createSutProbe(root, {});
  return Object.fromEntries(
    DEFAULT_SUT_COMPONENTS.filter((c) => c.path).map((c) => [
      c.name,
      resolveComponent(probe, c),
    ]),
  );
}

function storyReport(root, story, parts = currentParts(root)) {
  return {
    env: { lane: "household", processControl: true, sutParts: parts },
    summary: {
      byConcern: {
        [story.concern]: {
          scenarios: featureNames(root, story).map((name) => ({
            name,
            status: "passed",
            durationMs: 9,
            surface: story.feature.replace(/^genesis\/a2o\//, ""),
          })),
        },
      },
    },
  };
}

function runT2(root, changedPaths, reportsDir, env = {}) {
  const changed = join(reportsDir, "changed.txt");
  writeFileSync(changed, changedPaths.join("\n") + "\n");
  return spawnSync(
    "bash",
    [
      join(root, "genesis/orchestrator/scripts/t2-receipt.sh"),
      "--changed",
      changed,
      "--reports",
      reportsDir,
    ],
    {
      cwd: root,
      encoding: "utf8",
      // Hermetic: this workspace's own attestations must not satisfy a test's refusal.
      env: { ...process.env, SERVING_RECEIPT_ATTESTATIONS: "0", ...env },
    },
  );
}

test("each changed serving path requires the story that witnesses it", () => {
  for (const path of REFUSES_FAST_PATHS)
    assert.deepEqual(requiredReceipts([path]), [["refusesFast"]], path);
  assert.deepEqual(requiredReceipts(["scripts/ci/stage-spa-blob.sh"]), [
    ["deliverability", "refusesFast"],
  ]);
  assert.deepEqual(
    requiredReceipts([
      "doorway/doorway-service/src/server/http.rs",
      "scripts/ci/fleet-write-readiness.sh",
      "",
    ]),
    [["deliverability"], ["refusesFast"]],
  );
  assert.deepEqual(requiredReceipts(undefined), [["deliverability"]]);
});

test("the refuses-fast story is a receipt only with all four stations under its own concern", () => {
  const story = RECEIPT_STORIES.refusesFast;
  const four = ["s1", "s2", "s3", "s4"];
  const report = {
    env: { lane: "household", processControl: true, sutParts: { ...expected } },
    summary: {
      byConcern: {
        [story.concern]: {
          scenarios: four.map((name) => ({
            name,
            status: "passed",
            durationMs: 1,
            surface: "features/dataplane/app-delivery-refuses-fast.feature",
          })),
        },
      },
    },
  };
  assert.equal(validateReceipt(report, expected, four, story), true);
  report.summary.byConcern[story.concern].scenarios.pop();
  assert.equal(
    validateReceipt(report, expected, four.slice(0, 3), story),
    false,
    "fewer than four stations is not the story",
  );
  const full = structuredClone(report);
  full.summary.byConcern[story.concern].scenarios.push({
    name: "s4",
    status: "passed",
    durationMs: 1,
    surface: "features/dataplane/app-delivery-refuses-fast.feature",
  });
  assert.equal(
    validateReceipt(full, expected, four),
    false,
    "not a deliverability receipt",
  );
  full.summary.byConcern["doorway-failover"] =
    full.summary.byConcern[story.concern];
  delete full.summary.byConcern[story.concern];
  assert.equal(
    validateReceipt(full, expected, four, story),
    false,
    "wrong concern",
  );
});

test("the C2 paths are refused without a receipt in default and strict modes, naming the story", () => {
  const dir = mkdtempSync(join(tmpdir(), "c2-refused-"));
  try {
    for (const path of [
      ...REFUSES_FAST_PATHS,
      "scripts/ci/stage-spa-blob.sh",
    ]) {
      for (const mode of ["", "strict"]) {
        const result = runT2(REPO, [path], dir, { T2_RECEIPT: mode });
        assert.equal(result.status, 1, `${path} mode=${mode || "default"}`);
        assert.match(result.stderr, /NO-SERVING-RECEIPT/);
        assert.match(result.stderr, /app-delivery-refuses-fast\.feature/);
        assert.doesNotMatch(result.stdout + result.stderr, /NO-T2-RECEIPT/);
      }
    }
  } finally {
    rmSync(dir, { recursive: true, force: true });
  }
});

// These run the real t2-receipt.sh inside a committed fixture repo: the live checkout's source
// identity moves under concurrent sessions, and a receipt test must not race it.
test("a current-source refuses-fast report admits stage-spa-blob.sh and the readiness paths, not a render change", () => {
  withFixture((root, dir) => {
    writeFileSync(
      join(dir, "sprint-report-household-c2.json"),
      JSON.stringify(storyReport(root, RECEIPT_STORIES.refusesFast)),
    );
    for (const path of [
      "scripts/ci/stage-spa-blob.sh",
      ...REFUSES_FAST_PATHS,
    ]) {
      const result = runT2(root, [path], dir);
      assert.equal(result.status, 0, `${path}: ${result.stderr}`);
      assert.match(
        result.stdout,
        /T2 serving receipt: sprint-report-household-c2\.json; every app-delivery-refuses-fast\.feature station passed/,
      );
      assert.doesNotMatch(
        result.stdout + result.stderr,
        /NO-T2-RECEIPT|NO-SERVING-RECEIPT/,
      );
    }
    const mixed = runT2(
      root,
      [
        "scripts/ci/fleet-write-readiness.sh",
        "elohim/elohim-render/src/lib.rs",
      ],
      dir,
    );
    assert.equal(
      mixed.status,
      1,
      "a render change still needs the deliverability story",
    );
    assert.match(mixed.stderr, /epr-app-deliverability\.feature/);
  });
});

test("a deliverability receipt admits stage-spa-blob.sh but not the readiness probe", () => {
  withFixture((root, dir) => {
    writeFileSync(
      join(dir, "sprint-report-household-d.json"),
      JSON.stringify(storyReport(root, RECEIPT_STORIES.deliverability)),
    );
    assert.equal(runT2(root, ["scripts/ci/stage-spa-blob.sh"], dir).status, 0);
    assert.equal(
      runT2(root, ["scripts/ci/fleet-write-readiness.sh"], dir).status,
      1,
    );
  });
});

test("the bash leg prints the attestation receipt line for a stage-spa-blob.sh change", () => {
  withFixture((root, dir) => {
    const story = RECEIPT_STORIES.refusesFast;
    const node = householdNode(root, story);
    putRef(root, story.concern, JSON.parse(node.resultSummary).sut, node);
    const result = runT2(root, ["scripts/ci/stage-spa-blob.sh"], dir, {
      SERVING_RECEIPT_ATTESTATIONS: "1",
    });
    assert.equal(result.status, 0, result.stderr);
    assert.match(
      result.stdout,
      /T2 serving receipt: brit validation attestation push-delivers-within-budget\//,
    );
    assert.doesNotMatch(
      result.stdout + result.stderr,
      /NO-T2-RECEIPT|NO-SERVING-RECEIPT/,
    );
  });
});

// ── Lane F2: a signed household ValidationAttestation is the preferred receipt ─
// VECTOR is a real `brit-build-ref validate put` output under a throwaway key.
const VECTOR = JSON.parse(
  readFileSync(
    new URL(
      "../a2o/scripts/__tests__/fixtures/brit-validation-attestation.json",
      import.meta.url,
    ),
    "utf8",
  ),
);
const PKCS8 = Buffer.from("302e020100300506032b657004220420", "hex");

function signNode(node, seedHex) {
  const key = createPrivateKey({
    key: Buffer.concat([PKCS8, Buffer.from(seedHex, "hex")]),
    format: "der",
    type: "pkcs8",
  });
  const unsigned = { ...node, signature: "" };
  return {
    ...node,
    signature: edSign(null, encodeValidationNode(unsigned), key).toString(
      "hex",
    ),
  };
}

function git(root, args, input) {
  return spawnSync("git", args, {
    cwd: root,
    encoding: "utf8",
    input,
    env: NO_GIT_ENV,
  });
}

/** A throwaway repo holding every sut source path and both stories, with a workspace key. */
function fixtureRepo() {
  const root = mkdtempSync(join(tmpdir(), "f2-repo-"));
  git(root, ["init", "-q"]);
  for (const c of DEFAULT_SUT_COMPONENTS.filter((c) => c.path)) {
    const file = /\.(ya?ml|sh)$/.test(c.path) ? c.path : join(c.path, "marker");
    mkdirSync(join(root, file, ".."), { recursive: true });
    writeFileSync(join(root, file), `${c.name}\n`);
  }
  for (const file of [
    ...Object.values(RECEIPT_STORIES).map((story) => story.feature),
    "genesis/orchestrator/scripts/t2-receipt.sh",
    "genesis/orchestrator/scripts/serving-receipt.mjs",
    "genesis/a2o/scripts/lib/sut.ts",
    "genesis/a2o/scripts/lib/household-attestation.ts",
    // The attestation names its session through the shared resolver and its registry.
    "genesis/a2o/src/framework/session.ts",
    "genesis/agentic/agent-runtimes.json",
  ]) {
    mkdirSync(join(root, file, ".."), { recursive: true });
    writeFileSync(join(root, file), readFileSync(join(REPO, file), "utf8"));
  }
  git(root, ["add", "-A"]);
  git(root, [
    "-c",
    "user.name=t",
    "-c",
    "user.email=t@t",
    "commit",
    "-q",
    "-m",
    "fixture",
  ]);
  mkdirSync(join(root, ".git/brit"), { recursive: true });
  writeFileSync(
    join(root, ".git/brit/agent-key"),
    Buffer.from(VECTOR.seedHex, "hex"),
  );
  return root;
}

function putRef(root, concern, leaf, node) {
  const blob = git(
    root,
    ["hash-object", "-w", "--stdin"],
    JSON.stringify(node),
  ).stdout.trim();
  const ref = `refs/notes/brit/validate/a2o-household/${concern}/${leaf.replace(":", "%3A")}`;
  assert.equal(git(root, ["update-ref", ref, blob]).status, 0);
}

function householdNode(root, story, overrides = {}) {
  const parts = currentParts(root);
  const sut = sutOf(parts);
  const summary = {
    schema: RECEIPT_SCHEMA,
    concern: story.concern,
    sut,
    sutParts: parts,
    lane: "household",
    processControl: true,
    runId: "r",
    report: "sprint-report-household-r.json",
    reach: "trusted",
    moored: { session: "s", principal: "root", recipient: {} },
    scenarios: storyReport(root, story, parts).summary.byConcern[story.concern]
      .scenarios,
    ...overrides,
  };
  return signNode(
    {
      artifactCid: [...sutArtifactCidBytes(parts)],
      checkName: checkNameFor(story.concern, sut),
      findingsCid: null,
      result: "pass",
      resultSummary: JSON.stringify(summary),
      signature: "",
      ttlSec: null,
      validatedAt: "2026-09-24T00:00:00Z",
      validatorId: VECTOR.node.validatorId,
      validatorVersion: "a2o/build-sprint-report@1",
    },
    VECTOR.seedHex,
  );
}

function withFixture(fn) {
  const root = fixtureRepo();
  const reports = mkdtempSync(join(tmpdir(), "f2-reports-"));
  try {
    fn(root, reports);
  } finally {
    rmSync(root, { recursive: true, force: true });
    rmSync(reports, { recursive: true, force: true });
  }
}

function check(root, reports, paths, peerStatuses, env) {
  const out = [];
  const ok = checkReports(root, reports, paths, {
    attestations: true,
    peerStatuses,
    env,
    log: (l) => out.push(l),
    warn: (l) => out.push(l),
  });
  return { ok, out: out.join("\n") };
}

test("a signed current-source attestation is admitted as the receipt, and preferred over a report", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    assert.equal(
      check(root, reports, ["scripts/ci/stage-spa-blob.sh"]).ok,
      false,
    );
    writeFileSync(
      join(reports, "sprint-report-household-z.json"),
      JSON.stringify(storyReport(root, story)),
    );
    const node = householdNode(root, story);
    putRef(root, story.concern, JSON.parse(node.resultSummary).sut, node);
    const { ok, out } = check(root, reports, [
      "scripts/ci/fleet-write-readiness.sh",
    ]);
    assert.equal(ok, true, out);
    assert.match(
      out,
      /T2 serving receipt: brit validation attestation push-delivers-within-budget\/sha256:[0-9a-f]{16} \(reach trusted; signed by workspace [0-9a-f]{12}…\)/,
    );
    assert.doesNotMatch(out, /sprint-report-household-z/);
  });
});

test("an attestation that is tampered, foreign-keyed, failed, incomplete or stale is not a receipt", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    const good = householdNode(root, story);
    const summary = JSON.parse(good.resultSummary);
    const stale = { ...currentParts(root), storage: "tree:old" };
    const cases = {
      tampered: { ...good, validatedAt: "2026-09-25T00:00:00Z" },
      foreign: signNode(
        { ...good, validatorId: "00".repeat(32) },
        "11".repeat(32),
      ),
      failed: signNode({ ...good, result: "fail" }, VECTOR.seedHex),
      incomplete: householdNode(root, story, {
        scenarios: summary.scenarios.slice(0, 3),
      }),
      stale: signNode(
        {
          ...good,
          artifactCid: [...sutArtifactCidBytes(stale)],
          checkName: checkNameFor(story.concern, sutOf(stale)),
          resultSummary: JSON.stringify({
            ...summary,
            sut: sutOf(stale),
            sutParts: stale,
          }),
        },
        VECTOR.seedHex,
      ),
    };
    for (const [label, node] of Object.entries(cases))
      putRef(
        root,
        story.concern,
        `${label}-${JSON.parse(node.resultSummary).sut}`,
        node,
      );
    const { ok, out } = check(root, reports, [
      "scripts/ci/fleet-write-readiness.sh",
    ]);
    assert.equal(ok, false, out);
    assert.match(
      out,
      /missing household receipt — produce it: just test mesh features\/dataplane\/app-delivery-refuses-fast\.feature/,
    );
  });
});

// ── Governance atoms are not source-under-test ──────────────────────────────────
// A habit atom recording evidence about the system must not invalidate the evidence: `.epr-meta`
// entries leave every component identity and are never a serving change.
function commitAll(root, message) {
  assert.equal(git(root, ["add", "-A"]).status, 0);
  const done = git(root, [
    "-c",
    "user.name=t",
    "-c",
    "user.email=t@t",
    "commit",
    "-q",
    "-m",
    message,
  ]);
  assert.equal(done.status, 0, done.stderr);
}

function writeIn(root, path, text) {
  mkdirSync(join(root, path, ".."), { recursive: true });
  writeFileSync(join(root, path), text);
}

const STORAGE_HABIT = "elohim/elohim-storage/.epr-meta/x.habit.md";
const DOORWAY_CHANGE = ["doorway/doorway-service/src/server/http.rs"];

test("a receipt minted on tree T survives a commit that changes only a component's .epr-meta habit", () => {
  withFixture((root, reports) => {
    writeIn(root, STORAGE_HABIT, "---\nstatus: red\n---\n");
    writeIn(root, "doorway/doorway-service/.epr-meta/.epr-meta", "rules: []\n");
    commitAll(root, "governance package");
    writeFileSync(
      join(reports, "sprint-report-household-t.json"),
      JSON.stringify(storyReport(root, RECEIPT_STORIES.deliverability)),
    );
    assert.equal(check(root, reports, DOORWAY_CHANGE).ok, true);

    writeIn(root, STORAGE_HABIT, "---\nstatus: red\n---\n- 2026-09-25 delta\n");
    commitAll(root, "habit delta");
    let { ok, out } = check(root, reports, DOORWAY_CHANGE);
    assert.equal(ok, true, out);
    assert.match(
      out,
      /sprint-report-household-t\.json; every epr-app-deliverability/,
    );

    // Uncommitted governance edits (modified and untracked) do not move the identity either.
    writeIn(root, STORAGE_HABIT, "---\nstatus: green\n---\n");
    writeIn(root, "elohim/elohim-storage/.epr-meta/new.habit.md", "fresh\n");
    ({ ok, out } = check(root, reports, DOORWAY_CHANGE));
    assert.equal(ok, true, out);
  });
});

test("a receipt minted before the rule (raw tree counting .epr-meta) is re-expressed, not staled", () => {
  withFixture((root, reports) => {
    writeIn(root, STORAGE_HABIT, "v1\n");
    commitAll(root, "governance package");
    const raw = git(root, [
      "rev-parse",
      "HEAD:elohim/elohim-storage",
    ]).stdout.trim();
    const parts = { ...currentParts(root), storage: `tree:${raw}` };
    assert.notEqual(
      parts.storage,
      currentParts(root).storage,
      "raw form differs",
    );
    assert.equal(
      normalizeSutIdentity(createSutProbe(root, {}), parts.storage),
      currentParts(root).storage,
    );
    writeFileSync(
      join(reports, "sprint-report-household-legacy.json"),
      JSON.stringify(storyReport(root, RECEIPT_STORIES.deliverability, parts)),
    );
    writeIn(root, STORAGE_HABIT, "v2\n");
    commitAll(root, "habit delta");
    const { ok, out } = check(root, reports, DOORWAY_CHANGE);
    assert.equal(ok, true, out);
  });
});

test("a source change under the same component still stales the receipt", () => {
  withFixture((root, reports) => {
    writeIn(root, STORAGE_HABIT, "v1\n");
    commitAll(root, "governance package");
    writeFileSync(
      join(reports, "sprint-report-household-t.json"),
      JSON.stringify(storyReport(root, RECEIPT_STORIES.deliverability)),
    );
    writeIn(root, "elohim/elohim-storage/src/x.rs", "fn x() {}\n");
    commitAll(root, "source change");
    const { ok, out } = check(root, reports, DOORWAY_CHANGE);
    assert.equal(ok, false, out);
    assert.match(
      out,
      /missing household receipt — produce it: just test mesh features\/dataplane\/epr-app-deliverability\.feature/,
    );
    // A file merely NAMED like governance is source.
    assert.equal(
      isGovernancePath("elohim/elohim-storage/src/x.epr-meta"),
      false,
    );
  });
});

test("a changed path under .epr-meta is not a serving path, and the bash leg agrees with isGovernancePath", () => {
  assert.deepEqual(
    requiredReceipts([
      "elohim/elohim-render/.epr-meta/x.habit.md",
      "doorway/doorway-service/.epr-meta",
      "elohim/elohim-storage/src/services/release_adoption/.epr-meta/y.habit.md",
    ]),
    [],
  );
  assert.deepEqual(
    requiredReceipts([
      "elohim/elohim-render/.epr-meta/x.habit.md",
      "elohim/elohim-render/src/lib.rs",
    ]),
    [["deliverability"]],
  );
  withFixture((root, dir) => {
    // Every sample sits under a serving prefix, so the script refuses exactly the non-governance ones.
    const samples = {
      ".epr-meta/x.habit.md": true,
      ".epr-meta": true,
      "src/.epr-meta/nested.habit.md": true,
      "src/.epr-meta": true,
      "src/lib.rs": false,
      "src/x.epr-meta": false,
      "src/.epr-metadata/z.rs": false,
      "src/not.epr-meta/z.rs": false,
    };
    for (const [suffix, governance] of Object.entries(samples)) {
      const path = `elohim/elohim-render/${suffix}`;
      assert.equal(isGovernancePath(path), governance, `sut.ts: ${path}`);
      const result = runT2(root, [path], dir);
      assert.equal(result.status, governance ? 0 : 1, `t2-receipt.sh: ${path}`);
      assert.equal(/NO-SERVING-RECEIPT/.test(result.stderr), !governance, path);
    }
    const mixed = runT2(
      root,
      [
        "elohim/elohim-render/.epr-meta/x.habit.md",
        "elohim/elohim-render/src/lib.rs",
      ],
      dir,
    );
    assert.equal(mixed.status, 1);
    assert.match(
      mixed.stderr,
      /\[pre-push\]\s+elohim\/elohim-render\/src\/lib\.rs/,
    );
    assert.doesNotMatch(mixed.stderr, /\.epr-meta/);
  });
});

// ── Lane S2: a peer-executed stage is admitted by its verified DHT chain ─────────
const PEER = {
  rung: "H",
  provider: "uhCAk-jessica-provider",
  requester: "uhCAk-matthew-requester",
  grantActionHash: "uhCkk-grant",
  grantCid: "bafy-grant",
  scope: "measure-stage",
  requestActionHash: "uhCkk-request",
  completionActionHash: "uhCkk-completion-action",
  receiptCid: "bafy-receipt",
  taskCid: "bafy-task",
  featureSha256: "f".repeat(64),
  reportSha256: "e".repeat(64),
};
const verifiedTask = () => ({
  taskCid: PEER.taskCid,
  requestActionHash: PEER.requestActionHash,
  refusal: null,
  state: "completed",
  requester: PEER.requester,
  provider: PEER.provider,
  grantActionHash: PEER.grantActionHash,
  envelope: { project: "a2o-stage:x", dna: { sha256: PEER.featureSha256 } },
  completion: {
    actionHash: PEER.completionActionHash,
    receiptCid: PEER.receiptCid,
  },
  observed: {
    verified: true,
    grantCid: PEER.grantCid,
    scope: "measure-stage",
    grantProvider: PEER.provider,
    grantRecipient: PEER.requester,
    fulfilledEventId: `compute-fulfilled:${PEER.requestActionHash}`,
  },
});

test("a current-source peer-stage attestation is admitted when the requester's task record verifies its chain", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    const node = householdNode(root, story, { peer: PEER });
    putRef(root, story.concern, JSON.parse(node.resultSummary).sut, node);
    assert.deepEqual(peerRequestsOf(root), [PEER.requestActionHash]);
    const { ok, out } = check(
      root,
      reports,
      ["scripts/ci/fleet-write-readiness.sh"],
      new Map([[PEER.requestActionHash, verifiedTask()]]),
    );
    assert.equal(ok, true, out);
    assert.match(
      out,
      /brit validation attestation push-delivers-within-budget\/sha256:[0-9a-f]{16} \(reach trusted; signed by workspace [0-9a-f]{12}…\); stage ran on peer uhCAk-jessic… \(rung H\), completion uhCkk-comple… verified/,
    );
  });
});

test("without verified task records a peer-stage attestation is refused and the reader falls back to report files", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    const node = householdNode(root, story, { peer: PEER });
    putRef(root, story.concern, JSON.parse(node.resultSummary).sut, node);
    const paths = ["scripts/ci/fleet-write-readiness.sh"];
    for (const statuses of [
      undefined,
      new Map(),
      new Map([
        [
          PEER.requestActionHash,
          { ...verifiedTask(), observed: { verified: false, refused: "x" } },
        ],
      ]),
    ])
      assert.equal(check(root, reports, paths, statuses).ok, false);
    writeFileSync(
      join(reports, "sprint-report-household-z.json"),
      JSON.stringify(storyReport(root, story)),
    );
    const { ok, out } = check(root, reports, paths, new Map());
    assert.equal(ok, true, out);
    assert.match(out, /T2 serving receipt: sprint-report-household-z\.json/);
    assert.doesNotMatch(out, /brit validation attestation/);
  });
});

test("the prefetch reads task records only through the loopback compute API, and says why when it cannot", async () => {
  const none = await prefetchPeerStatuses([], {});
  assert.equal(none.statuses.size, 0);
  assert.equal(none.skipped, null);
  const noEnv = await prefetchPeerStatuses([PEER.requestActionHash], {
    COMPUTE_API_URL: "http://127.0.0.1:8090",
  });
  assert.equal(noEnv.statuses.size, 0);
  assert.match(noEnv.skipped, /^peer attestation skipped: no loopback env/);
  const env = {
    COMPUTE_API_URL: "http://127.0.0.1:8090",
    COMPUTE_PERFORMER: "uhCAk-matthew-requester",
    ELOHIM_COMPUTE_LOCAL_TOKEN: "tok",
  };
  const seen = [];
  const fetcher = async (url, init) => {
    seen.push({ url: String(url), init });
    return String(url).endsWith("uhCkk-request")
      ? { ok: true, json: async () => verifiedTask() }
      : { ok: false, status: 403, json: async () => ({}) };
  };
  const got = await prefetchPeerStatuses(
    [PEER.requestActionHash, "uhCkk-missing"],
    env,
    { fetcher },
  );
  assert.deepEqual(got.statuses.get(PEER.requestActionHash), verifiedTask());
  assert.equal(got.statuses.has("uhCkk-missing"), false);
  assert.match(got.skipped, /1\/2 task record\(s\) unreadable .*403/);
  assert.equal(
    seen[0].url,
    "http://127.0.0.1:8090/api/v1/compute/tasks/uhCkk-request",
  );
  assert.equal(seen[0].init.method, "GET");
  assert.equal(seen[0].init.headers["x-elohim-compute-token"], "tok");
  assert.equal(
    seen[0].init.headers["x-elohim-verified-performer"],
    env.COMPUTE_PERFORMER,
  );
  const remote = await prefetchPeerStatuses([PEER.requestActionHash], {
    ...env,
    COMPUTE_API_URL: "http://doorway.example:8090",
  });
  assert.equal(remote.statuses.size, 0);
  assert.match(remote.skipped, /loopback storage/);
});

test("the CLI names why a peer-stage attestation was skipped and still reads report files", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    const node = householdNode(root, story, { peer: PEER });
    putRef(root, story.concern, JSON.parse(node.resultSummary).sut, node);
    const changed = join(reports, "changed.txt");
    writeFileSync(changed, "scripts/ci/fleet-write-readiness.sh\n");
    const env = Object.fromEntries(
      Object.entries(NO_GIT_ENV).filter(
        ([k]) =>
          ![
            "COMPUTE_API_URL",
            "COMPUTE_PERFORMER",
            "ELOHIM_COMPUTE_LOCAL_TOKEN",
            "SERVING_RECEIPT_ATTESTATIONS",
          ].includes(k),
      ),
    );
    const cli = (extra = {}) =>
      spawnSync(
        process.execPath,
        [
          join(root, "genesis/orchestrator/scripts/serving-receipt.mjs"),
          root,
          reports,
          changed,
        ],
        { cwd: root, encoding: "utf8", env: { ...env, ...extra } },
      );
    let result = cli();
    assert.equal(result.status, 1, result.stderr);
    assert.match(
      result.stderr,
      /\[pre-push\] peer attestation skipped: no loopback env/,
    );
    writeFileSync(
      join(reports, "sprint-report-household-z.json"),
      JSON.stringify(storyReport(root, story)),
    );
    result = cli();
    assert.equal(result.status, 0, result.stderr);
    assert.match(result.stdout, /sprint-report-household-z\.json/);
    result = cli({ SERVING_RECEIPT_ATTESTATIONS: "0" });
    assert.doesNotMatch(result.stderr, /peer attestation/);
  });
});

test("a task record binds the request it was fetched for and the task CID the summary claims", () => {
  const good = verifiedTask();
  assert.equal(
    boundStatus(
      new Map([[PEER.requestActionHash, good]]),
      PEER.requestActionHash,
    ),
    good,
  );
  const misfiled = { ...good, requestActionHash: "uhCkk-other-request" };
  assert.equal(
    boundStatus(
      new Map([[PEER.requestActionHash, misfiled]]),
      PEER.requestActionHash,
    ),
    undefined,
  );
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    const node = householdNode(root, story, {
      peer: { ...PEER, taskCid: "bafy-task-B" },
    });
    putRef(root, story.concern, JSON.parse(node.resultSummary).sut, node);
    const paths = ["scripts/ci/fleet-write-readiness.sh"];
    for (const status of [{ ...good, taskCid: "bafy-task-A" }, good, misfiled])
      assert.equal(
        check(root, reports, paths, new Map([[PEER.requestActionHash, status]]))
          .ok,
        false,
      );
  });
});

test("the prefetch refuses a task record that names another request", async () => {
  const got = await prefetchPeerStatuses(
    [PEER.requestActionHash],
    {
      COMPUTE_API_URL: "http://127.0.0.1:8090",
      COMPUTE_PERFORMER: PEER.requester,
      ELOHIM_COMPUTE_LOCAL_TOKEN: "tok",
    },
    {
      fetcher: async () => ({
        ok: true,
        json: async () => ({
          ...verifiedTask(),
          requestActionHash: "uhCkk-other-request",
        }),
      }),
    },
  );
  assert.equal(got.statuses.size, 0);
  assert.match(got.skipped, /returned task record for uhCkk-other-/);
});

// ── brit-build-ref lineages: `signer --json` identity and verified reads ────────
// Stubs print the EXACT streams the two lineages print (brit 7e4a83115e read from source; the
// pinned 76ba95d8a0 executed): `{"agentId":…}` / clap's exit-2 usage error for `signer`, and
// `validate get`'s miss + `unverifiable: <ref> <reason>` lines. They live under TMPDIR.
const PINNED_SIGNER_STDERR =
  "error: unrecognized subcommand 'signer'\n\nUsage: brit-build-ref [OPTIONS] <COMMAND>\n\nFor more information, try '--help'.\n";
const C2_PATHS = ["scripts/ci/fleet-write-readiness.sh"];

function stubBrit(kind, agentId) {
  const dir = mkdtempSync(join(tmpdir(), `brit-stub-${kind}-`));
  mkdirSync(join(dir, "get"));
  const signer =
    kind === "new"
      ? `printf '%s\\n' '${JSON.stringify({ agentId })}'; exit 0`
      : `printf '%s' "${PINNED_SIGNER_STDERR.replaceAll("\n", "\\n")}" >&2; exit 2`;
  const bin = join(dir, "brit-build-ref");
  writeFileSync(
    bin,
    `#!/bin/sh
DIR='${dir}'
printf '%s\\n' "$*" >> "$DIR/calls.log"
if [ "$1" = "--repo" ]; then shift 2; fi
case "$1 $2" in
  "signer --json"|"signer ") ${signer} ;;
  "validate get")
    step=''; check=''
    while [ $# -gt 0 ]; do
      [ "$1" = "--step" ] && step="$2"
      [ "$1" = "--check" ] && check="$2"
      shift
    done
    key=$(printf '%s' "$check" | tr '/:' '__')
    if [ -f "$DIR/get/$key.out" ]; then cat "$DIR/get/$key.out"; fi
    if [ -f "$DIR/get/$key.err" ]; then cat "$DIR/get/$key.err" >&2
    elif [ ! -f "$DIR/get/$key.out" ]; then printf 'no validate ref found for step=%s check=%s\\n' "$step" "$check" >&2; fi
    exit 0 ;;
  *) printf "error: unrecognized subcommand '%s'\\n" "$1" >&2; exit 2 ;;
esac
`,
    { mode: 0o755 },
  );
  return { bin, dir };
}

function stubGet(dir, check, r) {
  const key = check.replaceAll(/[/:]/g, "_");
  if (r.out !== undefined) writeFileSync(join(dir, "get", `${key}.out`), r.out);
  if (r.err !== undefined) writeFileSync(join(dir, "get", `${key}.err`), r.err);
}

function stubCalls(dir) {
  return readFileSync(join(dir, "calls.log"), "utf8").trim().split("\n");
}

function withStub(kind, agentId, fn) {
  const stub = stubBrit(kind, agentId);
  try {
    fn(stub);
  } finally {
    rmSync(stub.dir, { recursive: true, force: true });
  }
}

function putStory(root, story, overrides) {
  const node = householdNode(root, story, overrides);
  const sut = JSON.parse(node.resultSummary).sut;
  putRef(root, story.concern, sut, node);
  return {
    node,
    sut,
    check: checkNameFor(story.concern, sut),
    ref: `refs/notes/brit/validate/a2o-household/${story.concern}/${sut.replace(":", "%3A")}`,
  };
}

test("the new brit's `signer --json` names the workspace key with no key file on disk; the receipt says brit verified the read", () => {
  withFixture((root, reports) => {
    rmSync(join(root, ".git/brit/agent-key"));
    const story = RECEIPT_STORIES.refusesFast;
    const { node, check: checkName } = putStory(root, story);
    withStub("new", VECTOR.node.validatorId, (stub) => {
      stubGet(stub.dir, checkName, {
        out: JSON.stringify(node, null, 2) + "\n",
      });
      const { ok, out } = check(root, reports, C2_PATHS, undefined, {
        BRIT_BUILD_REF: stub.bin,
      });
      assert.equal(ok, true, out);
      assert.match(
        out,
        /signed by workspace [0-9a-f]{12}…\); every app-delivery-refuses-fast\.feature station passed on current source; read verified by brit-build-ref\.$/m,
      );
      assert.deepEqual(stubCalls(stub.dir), [
        `--repo ${root} signer --json`,
        `--repo ${root} validate get --step a2o-household --check ${checkName}`,
      ]);
    });
    // Without a brit and without the file there is no workspace key, so no attestation receipt.
    const bare = check(root, reports, C2_PATHS, undefined, {
      PATH: "/nonexistent",
    });
    assert.equal(bare.ok, false, bare.out);
    assert.doesNotMatch(bare.out, /brit validation attestation/);
  });
});

test("a record the new brit reports `unverifiable:` for is absent — warned by ref and reason — and the report file still admits", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    const { check: checkName, ref } = putStory(root, story);
    const id = VECTOR.node.validatorId;
    withStub("new", id, (stub) => {
      stubGet(stub.dir, checkName, {
        err: `no validate ref found for step=a2o-household check=${checkName}\nunverifiable: ${ref} signature does not verify against agentId "${id}"\n`,
      });
      const env = { BRIT_BUILD_REF: stub.bin };
      let r = check(root, reports, C2_PATHS, undefined, env);
      assert.equal(r.ok, false, r.out);
      assert.ok(
        r.out.includes(
          `[pre-push] household attestation unverifiable: ${ref} signature does not verify against agentId "${id}" (brit-build-ref verified read)`,
        ),
        r.out,
      );
      assert.match(
        r.out,
        /missing household receipt — produce it: just test mesh/,
      );
      assert.doesNotMatch(r.out, /brit validation attestation/);
      writeFileSync(
        join(reports, "sprint-report-household-z.json"),
        JSON.stringify(storyReport(root, story)),
      );
      r = check(root, reports, C2_PATHS, undefined, env);
      assert.equal(r.ok, true, r.out);
      assert.match(
        r.out,
        /sprint-report-household-z\.json; every .* station passed/,
      );
      assert.doesNotMatch(r.out, /brit validation attestation/);
    });
  });
});

test("a new brit that names no key, or refuses to load one, admits no attestation: the key file is not its fallback", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    const { node, check: checkName } = putStory(root, story, { peer: PEER });
    withStub("new", null, (stub) => {
      stubGet(stub.dir, checkName, {
        out: JSON.stringify(node, null, 2) + "\n",
      });
      const env = { BRIT_BUILD_REF: stub.bin };
      const r = check(
        root,
        reports,
        C2_PATHS,
        new Map([[PEER.requestActionHash, verifiedTask()]]),
        env,
      );
      assert.equal(r.ok, false, r.out);
      assert.doesNotMatch(r.out, /brit validation attestation/);
      assert.deepEqual(peerRequestsOf(root, undefined, { env }), []);
      assert.deepEqual(stubCalls(stub.dir), [
        `--repo ${root} signer --json`,
        `--repo ${root} signer --json`,
      ]);
    });
    withStub("new", null, (stub) => {
      writeFileSync(
        stub.bin,
        '#!/bin/sh\nprintf "Error: cannot load the repository signer key\\n\\nCaused by:\\n    seed is 31 bytes\\n" >&2\nexit 1\n',
        { mode: 0o755 },
      );
      const r = check(root, reports, C2_PATHS, undefined, {
        BRIT_BUILD_REF: stub.bin,
      });
      assert.equal(r.ok, false, r.out);
      assert.match(
        r.out,
        /^\[pre-push\] no workspace key via brit signer: Error: cannot load the repository signer key$/m,
      );
    });
  });
});

test("the pinned brit (clap exit 2 for `signer`) falls back to the key file; its unverified `validate get` is never asked, and the receipt says the read was verified locally", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    putStory(root, story);
    withStub("pinned", null, (stub) => {
      const { ok, out } = check(root, reports, C2_PATHS, undefined, {
        BRIT_BUILD_REF: stub.bin,
      });
      assert.equal(ok, true, out);
      assert.match(
        out,
        /station passed on current source; read verified locally \(the pinned brit-build-ref has no verified read\)\.$/m,
      );
      assert.deepEqual(stubCalls(stub.dir), [`--repo ${root} signer --json`]);
    });
    const none = check(root, reports, C2_PATHS, undefined, {
      PATH: "/nonexistent",
    });
    assert.equal(none.ok, true, none.out);
    assert.match(
      none.out,
      /read verified locally \(brit-build-ref is not on this host\)\.$/m,
    );
  });
});

test("under the pinned brit a tampered or misfiled record is warned by ref and reason, never silently dropped", () => {
  withFixture((root, reports) => {
    const story = RECEIPT_STORIES.refusesFast;
    const good = householdNode(root, story);
    const sut = JSON.parse(good.resultSummary).sut;
    putRef(root, story.concern, sut, {
      ...good,
      validatedAt: "2026-09-25T00:00:00Z",
    });
    putRef(root, story.concern, `misfiled-${sut}`, good);
    const { ok, out } = check(root, reports, C2_PATHS, undefined, {
      PATH: "/nonexistent",
    });
    assert.equal(ok, false, out);
    const prefix = `refs/notes/brit/validate/a2o-household/${story.concern}/`;
    const verifier = "(verified locally; brit-build-ref is not on this host)";
    assert.ok(
      out.includes(
        `[pre-push] household attestation unverifiable: ${prefix}${sut.replace(":", "%3A")} signature does not verify against agentId "${VECTOR.node.validatorId}" ${verifier}`,
      ),
      out,
    );
    assert.ok(
      out.includes(
        `[pre-push] household attestation unverifiable: ${prefix}misfiled-${sut.replace(":", "%3A")} checkName "${checkNameFor(story.concern, sut)}" does not match this ref's "${story.concern}/misfiled-${sut}" ${verifier}`,
      ),
      out,
    );
  });
});
