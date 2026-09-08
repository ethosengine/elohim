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
import { fileURLToPath } from "node:url";
import {
  createSutProbe,
  resolveComponent,
  DEFAULT_SUT_COMPONENTS,
} from "../a2o/scripts/lib/sut.ts";
import { validateReceipt, checkReports } from "./scripts/serving-receipt.mjs";

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
