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
import { execFileSync, spawnSync } from "node:child_process";
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
  a2o: "tree:tests",
};
function receipt() {
  return {
    env: { lane: "household", processControl: true, sutParts: expected },
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
test("storage service alone cannot bypass mandatory receipt; caller file survives", () => {
  const root = execFileSync("git", ["rev-parse", "--show-toplevel"], {
    encoding: "utf8",
  }).trim();
  const dir = mkdtempSync(join(tmpdir(), "serving-receipt-"));
  try {
    const changed = join(dir, "changed");
    for (const path of [
      "elohim/elohim-storage/src/services/content_service.rs",
      "doorway/doorway-service/src/server/http.rs",
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
  const root = execFileSync("git", ["rev-parse", "--show-toplevel"], {
    encoding: "utf8",
  }).trim();
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
  const root = execFileSync("git", ["rev-parse", "--show-toplevel"], {
    encoding: "utf8",
  }).trim();
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
