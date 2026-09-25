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
        { cwd: root, encoding: "utf8" },
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

function check(root, reports, paths) {
  const out = [];
  const ok = checkReports(root, reports, paths, {
    attestations: true,
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
