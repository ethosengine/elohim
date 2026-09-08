// Reuse the report producer's source identity; timestamps cannot attest tested bytes.
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  createSutProbe,
  resolveComponent,
  DEFAULT_SUT_COMPONENTS,
} from "../../a2o/scripts/lib/sut.ts";

export function validateReceipt(report, expected, names) {
  if (report.env?.lane !== "household" || report.env?.processControl !== true)
    return false;
  for (const [name, identity] of Object.entries(expected)) {
    if (!identity || report.env?.sutParts?.[name] !== identity) return false;
  }
  const scenarios =
    report.summary?.byConcern?.["doorway-failover"]?.scenarios ?? [];
  const measured = scenarios.filter((s) =>
    s.surface
      ?.replaceAll("\\", "/")
      .endsWith("features/dataplane/epr-app-deliverability.feature"),
  );
  // Exactly the current feature's stations: missing/duplicate/skipped/pending is not proof.
  return (
    names.length >= 5 &&
    measured.length === names.length &&
    names.every((name) => {
      const matches = measured.filter((s) => s.name === name);
      return (
        matches.length === 1 &&
        matches[0].status === "passed" &&
        matches[0].durationMs > 0
      );
    })
  );
}

export function checkReports(root, reports) {
  const feature = readFileSync(
    join(root, "genesis/a2o/features/dataplane/epr-app-deliverability.feature"),
    "utf8",
  );
  const names = [...feature.matchAll(/^\s*Scenario:\s*(.+)$/gm)].map((m) =>
    m[1].trim(),
  );
  const probe = createSutProbe(root, {}); // Environment overrides cannot claim matching source.
  const expected = Object.fromEntries(
    DEFAULT_SUT_COMPONENTS.filter((c) => c.path).map((c) => [
      c.name,
      resolveComponent(probe, c),
    ]),
  );
  for (const name of readdirSync(reports)
    .filter((n) => /^sprint-report-household-.*\.json$/.test(n))
    .sort()
    .reverse()) {
    try {
      if (
        validateReceipt(
          JSON.parse(readFileSync(join(reports, name), "utf8")),
          expected,
          names,
        )
      ) {
        console.log(
          `[pre-push] T2 serving receipt: ${name}; all deliverability stations passed on current source.`,
        );
        return true;
      }
    } catch {
      /* An incomplete report cannot be a receipt; inspect the remaining runs. */
    }
  }
  return false;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    process.exitCode = checkReports(process.argv[2], process.argv[3]) ? 0 : 1;
  } catch (error) {
    console.error(`[pre-push] serving receipt unavailable: ${error.message}`);
    process.exitCode = 1;
  }
}
