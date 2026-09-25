// Reuse the report producer's source identity; timestamps cannot attest tested bytes.
//
// A serving path is admitted to a push only by a household receipt from the story that
// witnesses it, on the current source. Two receipt forms, preferred in this order:
//   1. a brit ValidationAttestation under refs/notes/brit/validate/a2o-household/<concern>/<sut>,
//      signed by this workspace's key, whose sut re-derives from the source parts it carries
//      (reach trusted: verified on the household) — written by build-sprint-report.ts;
//   2. a sprint-report-household-*.json file with the same content.
// Both must name every station of the story, each passed, on source parts equal to the tree.
// Governance metadata is not source-under-test: a `.epr-meta` path is never a serving change, and
// a component's identity leaves its `.epr-meta` entries out (sut.ts owns that one definition).
//
// usage: serving-receipt.mjs <repo-root> <reports-dir> [<file: changed serving paths, one per line>]
import { readFileSync, readdirSync, existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import {
  createSutProbe,
  resolveComponent,
  normalizeSutIdentity,
  isGovernancePath,
  DEFAULT_SUT_COMPONENTS,
} from "../../a2o/scripts/lib/sut.ts";
import {
  admitAttestation,
  gitCommonDir,
  readHouseholdAttestations,
  workspaceAgentId,
} from "../../a2o/scripts/lib/household-attestation.ts";

/** The household stories whose passing run is a receipt, and the concern each rolls up under. */
export const RECEIPT_STORIES = {
  deliverability: {
    feature: "genesis/a2o/features/dataplane/epr-app-deliverability.feature",
    concern: "doorway-failover",
    minStations: 5,
  },
  refusesFast: {
    feature: "genesis/a2o/features/dataplane/app-delivery-refuses-fast.feature",
    concern: "push-delivers-within-budget",
    minStations: 4,
  },
};

// Serving paths only the not-ready-window story exercises (lane C2), and the deploy script both
// stories run, whose receipt either one may give. Every other serving path needs deliverability.
const REFUSES_FAST_ONLY =
  /^(scripts\/ci\/(fleet-write-readiness\.sh|lib\/)|elohim\/elohim-storage\/src\/services\/release_adoption\/)/;
const EITHER_STORY = /^scripts\/ci\/stage-spa-blob/;

/**
 * The receipts a set of changed serving paths requires: a list of alternatives, each a list of
 * story keys any one of which admits the paths behind it. No list means the historical contract
 * (deliverability for every serving path).
 */
export function requiredReceipts(paths) {
  if (!paths) return [["deliverability"]];
  const groups = new Map();
  for (const path of paths
    .map((p) => p.trim())
    .filter((p) => p && !isGovernancePath(p))) {
    const keys = REFUSES_FAST_ONLY.test(path)
      ? ["refusesFast"]
      : EITHER_STORY.test(path)
        ? ["deliverability", "refusesFast"]
        : ["deliverability"];
    groups.set(keys.join("|"), keys);
  }
  return [...groups.values()];
}

/**
 * `normalize` maps a report's component identity onto today's form (a receipt minted before
 * governance was excluded still counts `.epr-meta` in its tree term); identity by default.
 */
export function validateReceipt(
  report,
  expected,
  names,
  story = RECEIPT_STORIES.deliverability,
  normalize = (identity) => identity,
) {
  if (report.env?.lane !== "household" || report.env?.processControl !== true)
    return false;
  for (const [name, identity] of Object.entries(expected)) {
    const reported = report.env?.sutParts?.[name];
    if (!identity || typeof reported !== "string") return false;
    if (reported !== identity && normalize(reported) !== identity) return false;
  }
  const surface = story.feature.replace(/^genesis\/a2o\//, "");
  const scenarios = report.summary?.byConcern?.[story.concern]?.scenarios ?? [];
  const measured = scenarios.filter((s) =>
    s.surface?.replaceAll("\\", "/").endsWith(surface),
  );
  // Exactly the current feature's stations: missing/duplicate/skipped/pending is not proof.
  return (
    names.length >= story.minStations &&
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

function stationNames(root, story) {
  const feature = readFileSync(join(root, story.feature), "utf8");
  return [...feature.matchAll(/^\s*Scenario:\s*(.+)$/gm)].map((m) =>
    m[1].trim(),
  );
}

/** The newest attestation that admits `story` on the current source, or null. */
export function findAttestationReceipt(
  root,
  story,
  expected,
  names,
  normalize,
) {
  const common = gitCommonDir(root);
  const workspaceId = common ? workspaceAgentId(common) : null;
  if (!workspaceId) return null;
  const candidates = readHouseholdAttestations(root, story.concern).sort(
    (a, b) =>
      String(b.node.validatedAt).localeCompare(String(a.node.validatedAt)),
  );
  for (const { ref, node } of candidates) {
    const verdict = admitAttestation(node, story.concern, workspaceId);
    if (!verdict.ok) continue;
    const s = verdict.summary;
    const asReport = {
      env: {
        lane: s.lane,
        processControl: s.processControl,
        sutParts: s.sutParts,
      },
      summary: { byConcern: { [story.concern]: { scenarios: s.scenarios } } },
    };
    if (validateReceipt(asReport, expected, names, story, normalize))
      return { ref, node, summary: s };
  }
  return null;
}

/** The newest household report file that admits `story` on the current source, or null. */
export function findReportReceipt(reports, story, expected, names, normalize) {
  if (!existsSync(reports)) return null;
  for (const name of readdirSync(reports)
    .filter((n) => /^sprint-report-household-.*\.json$/.test(n))
    .sort()
    .reverse()) {
    try {
      const report = JSON.parse(readFileSync(join(reports, name), "utf8"));
      if (validateReceipt(report, expected, names, story, normalize))
        return name;
    } catch {
      /* An incomplete report cannot be a receipt; inspect the remaining runs. */
    }
  }
  return null;
}

export function checkReports(root, reports, paths, options = {}) {
  const log = options.log ?? console.log;
  const warn = options.warn ?? console.error;
  const attestations =
    options.attestations ?? process.env.SERVING_RECEIPT_ATTESTATIONS !== "0";
  const probe = createSutProbe(root, {}); // Environment overrides cannot claim matching source.
  const expected = Object.fromEntries(
    DEFAULT_SUT_COMPONENTS.filter((c) => c.path).map((c) => [
      c.name,
      resolveComponent(probe, c),
    ]),
  );
  const normalized = new Map();
  const normalize = (identity) => {
    if (!normalized.has(identity))
      normalized.set(identity, normalizeSutIdentity(probe, identity));
    return normalized.get(identity);
  };
  let all = true;
  const said = new Set();
  for (const alternatives of requiredReceipts(paths)) {
    let found = null;
    // Prefer a signed attestation from any admitting story, then a report file.
    for (const source of attestations
      ? ["attestation", "report"]
      : ["report"]) {
      for (const key of alternatives) {
        const story = RECEIPT_STORIES[key];
        const names = stationNames(root, story);
        const feature = story.feature.split("/").pop();
        if (source === "attestation") {
          const hit = findAttestationReceipt(
            root,
            story,
            expected,
            names,
            normalize,
          );
          if (hit) {
            found = `brit validation attestation ${hit.node.checkName} (reach ${hit.summary.reach}; signed by workspace ${hit.node.validatorId.slice(0, 12)}…); every ${feature} station passed on current source.`;
          }
        } else {
          const hit = findReportReceipt(
            reports,
            story,
            expected,
            names,
            normalize,
          );
          if (hit) {
            found = `${hit}; every ${feature} station passed on current source.`;
          }
        }
        if (found) break;
      }
      if (found) break;
    }
    if (found) {
      if (!said.has(found)) log(`[pre-push] T2 serving receipt: ${found}`);
      said.add(found);
    } else {
      all = false;
      const run = alternatives
        .map(
          (key) =>
            `just test mesh ${RECEIPT_STORIES[key].feature.replace(/^genesis\/a2o\//, "")}`,
        )
        .join("  OR  ");
      warn(`[pre-push] missing household receipt — produce it: ${run}`);
    }
  }
  return all;
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  try {
    const paths = process.argv[4]
      ? readFileSync(process.argv[4], "utf8").split("\n")
      : undefined;
    process.exitCode = checkReports(process.argv[2], process.argv[3], paths)
      ? 0
      : 1;
  } catch (error) {
    console.error(`[pre-push] serving receipt unavailable: ${error.message}`);
    process.exitCode = 1;
  }
}
