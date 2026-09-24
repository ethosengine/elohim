import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import * as AjvNs from 'ajv/dist/2020.js';
import * as addFormatsNs from 'ajv-formats';

// AJV v8 dual-CJS/ESM packaging means the default export is sometimes wrapped.
// Unwrap both at runtime using namespace imports to avoid TypeScript's
// "no construct signatures" error on default imports from CJS modules.
const AjvCtor: new (opts: { strict: boolean; allErrors: boolean }) => AjvNs.default =
  (
    AjvNs as unknown as {
      default: new (opts: { strict: boolean; allErrors: boolean }) => AjvNs.default;
    }
  ).default ??
  (AjvNs as unknown as new (opts: { strict: boolean; allErrors: boolean }) => AjvNs.default);
const addFormatsFn: (ajv: AjvNs.default) => void =
  (addFormatsNs as unknown as { default: (ajv: AjvNs.default) => void }).default ??
  (addFormatsNs as unknown as (ajv: AjvNs.default) => void);

import { aggregate } from './lib/aggregate.js';
import { readDeclaredConcerns } from './lib/declared-concerns.js';
import { publishHouseholdEvidence } from './lib/household-attestation.js';
import { loadConsoleArtifacts } from './lib/load-console.js';
import { loadCoverageGap } from './lib/load-coverage-gap.js';
import { loadCucumber } from './lib/load-cucumber.js';
import { renderMarkdown } from './lib/render-markdown.js';
import { computeRunEnv, createRunEnvProbe, resolveGitCommit } from './lib/run-env.js';

import type { SprintReport } from './lib/aggregate.js';

/** genesis/a2o/scripts/ -> repo root. */
const REPO_ROOT = fileURLToPath(new URL('../../../', import.meta.url));
const HABITS_PATH = join(REPO_ROOT, 'genesis/manifests/habits.yaml');

interface Args {
  reportsDir: string;
  cucumberPath: string;
  consoleDir: string;
  coverageGapPath: string;
  outJson: string;
  outMd: string;
  runId: string;
  profile: string;
  transport: string;
  doorway?: string;
  /**
   * Which lane produced this run — `household` (local mesh) or `alpha-fleet`.
   * OPTIONAL, and every existing caller omits it: without it the lane is
   * derived from the profile (see laneForProfile), so
   * scripts/ci/run-dataplane-validation.sh keeps working unchanged.
   */
  lane?: string;
  /**
   * The path/tag scope a caller requested for this run (`just test mesh
   * <scope>`), OPTIONAL. When present and this run measured zero scenarios,
   * that is an empty-selection receipt — the scope resolved to nothing
   * rather than a real (possibly red) run — and main() exits non-zero
   * instead of writing a quiet, exit-0 "0 scenarios" report.
   */
  scope?: string;
}

function parseArgs(argv: string[]): Args {
  const opts = new Map<string, string>();
  for (let i = 0; i < argv.length; i += 2) opts.set(argv[i], argv[i + 1]);

  const reportsDir = opts.get('--reports-dir') ?? 'reports';
  return {
    reportsDir,
    cucumberPath: opts.get('--cucumber') ?? join(reportsDir, 'cucumber-report.json'),
    consoleDir: opts.get('--console-dir') ?? join(reportsDir, 'console'),
    coverageGapPath: opts.get('--coverage-gap') ?? join(reportsDir, 'coverage-gap.json'),
    outJson: opts.get('--out-json') ?? join(reportsDir, 'sprint-report.json'),
    outMd: opts.get('--out-md') ?? join(reportsDir, 'sprint-report.md'),
    runId: opts.get('--run-id') ?? process.env.BUILD_TAG ?? new Date().toISOString(),
    profile: opts.get('--profile') ?? process.env.CUCUMBER_PROFILE ?? 'unknown',
    transport:
      opts.get('--transport') ??
      process.env.MESH_TRANSPORT_BACKEND ??
      process.env.ELOHIM_TRANSPORT_BACKEND ??
      'unknown',
    doorway: opts.get('--doorway') ?? process.env.E2E_DOORWAY_ALPHA,
    lane: opts.get('--lane') ?? process.env.A2O_LANE,
    // "" (the justfile's unscoped default) means absent, same as omitted.
    scope: (opts.get('--scope') ?? '').length > 0 ? opts.get('--scope') : undefined,
  };
}

function ensureDir(p: string) {
  const dir = dirname(p);
  if (!existsSync(dir)) mkdirSync(dir, { recursive: true });
}

function main() {
  const args = parseArgs(process.argv.slice(2));

  const cucumberJson = existsSync(args.cucumberPath)
    ? readFileSync(args.cucumberPath, 'utf8')
    : '[]';
  const scenarios = loadCucumber(cucumberJson);
  const consoleArtifacts = loadConsoleArtifacts(args.consoleDir);
  const gaps = loadCoverageGap(args.coverageGapPath);

  // The derivation key (Law II) and the denominator (Law I). Both are computed
  // outside aggregate(): the environment is observed here, and the declared
  // concern set is READ from the covenant file rather than from the run.
  const probe = createRunEnvProbe(REPO_ROOT);
  const env = computeRunEnv(probe, { profile: args.profile, lane: args.lane });
  const declaredConcerns = readDeclaredConcerns(HABITS_PATH);

  const report = aggregate({
    scenarios,
    consoleArtifacts,
    gaps,
    runId: args.runId,
    profile: args.profile,
    transport: args.transport,
    doorway: args.doorway,
    gitCommit: resolveGitCommit(probe),
    env,
    declaredConcerns,
  });

  // Schema-validate before writing
  const schemaPath = fileURLToPath(
    new URL('../schemas/sprint-report.schema.json', import.meta.url)
  );
  const schema = JSON.parse(readFileSync(schemaPath, 'utf8'));
  const ajv = new AjvCtor({ strict: true, allErrors: true });
  addFormatsFn(ajv);
  const validate = ajv.compile(schema);
  if (!validate(report)) {
    console.error('Sprint report failed schema validation:');
    console.error(JSON.stringify(validate.errors, null, 2));
    process.exit(2);
  }

  ensureDir(args.outJson);
  writeFileSync(args.outJson, JSON.stringify(report, null, 2));
  ensureDir(args.outMd);
  writeFileSync(args.outMd, renderMarkdown(report));

  console.log(`Sprint report written:`);
  console.log(`  ${args.outJson}`);
  console.log(`  ${args.outMd}`);
  console.log(
    `Findings: ${report.summary.findings.total} (scenarios: ${report.summary.scenarios.total})`
  );
  console.log(
    `Lane: ${env.lane} · transport ${args.transport} · peers ${env.peers} · ` +
      `processControl ${env.processControl} · ` +
      `stage ${env.networkStage} · sut ${env.sut}` +
      (env.unknown.length > 0 ? ` · unknown: ${env.unknown.join(', ')}` : '')
  );
  printDeclaredHeadline(report);

  // Empty-selection receipt: a scoped run that measured ZERO scenarios did
  // not exercise anything — most often the scope resolved to nothing
  // (silently, at exit 0, from cucumber's own end). A clean report with 0
  // scenarios and a scope on record is exactly that false-green shape, so
  // this is the one case where the BUILDER itself fails the run rather than
  // deferring entirely to cucumber's own exit code.
  if (args.scope && report.summary.scenarios.total === 0) {
    console.error(
      `EMPTY SELECTION: scope '${args.scope}' measured 0 scenarios — this is an ` +
        `empty-selection receipt, not a passing run. Check the scope resolves to ` +
        `something relative to genesis/a2o (a bad path silently selects nothing).`
    );
    process.exit(3);
  }

  // The report is the evidence; these are its projections, and neither changes the verdict.
  // A household run fulfils its valueflow commitments (`epr flow fulfill`, formerly by hand)
  // and leaves one signed ValidationAttestation per concern, keyed by env.sut, that the
  // pre-push T2 reader admits as the receipt (genesis/orchestrator/scripts/serving-receipt.mjs).
  // Each step runs only when its binary is on this host and says so when it is not.
  publishHouseholdEvidence({
    report,
    reportPath: resolve(args.outJson),
    repoRoot: REPO_ROOT,
    env: process.env,
  });
}

/**
 * Law I's five-number headline on the console: declared · permitted · refused ·
 * referred · not measured. `referred` is 0 by construction — `Decision::Refer`
 * has no producer on this path (guidestar §S5) — and is printed anyway so a
 * producer cannot arrive silently.
 */
function printDeclaredHeadline(report: SprintReport): void {
  const declared = report.declared;
  if (!declared) return;

  // A zero denominator must never render as clean (Law I): if habits.yaml could
  // not be read, every number below is 0 for the wrong reason. The loud line
  // goes to stderr so a CI log scanner cannot miss it.
  if (declared.unreadable) {
    console.error(`DENOMINATOR UNREADABLE — ${declared.unreadable}`);
    console.error(
      `  Every declared/not-measured count below is 0 because the covenant could not be read,` +
        ` NOT because coverage is complete. This run measured nothing it can account for.`
    );
  }

  const byConcern = report.summary.byConcern;
  let permitted = 0;
  let refused = 0;
  for (const name of declared.exercised) {
    if ((byConcern[name]?.failed ?? 0) > 0) refused += 1;
    else permitted += 1;
  }

  const missing = declared.notMeasured;
  console.log(
    `declared ${declared.concerns.length} · permitted ${permitted} · refused ${refused} · ` +
      `referred 0 · NOT MEASURED ${missing.length}` +
      (missing.length > 0 ? ` (${missing.join(', ')})` : '')
  );
  if (declared.notMeasuredRanAndSkipped.length > 0) {
    console.log(
      `  of those, RAN AND SKIPPED (apparatus, not scope): ` +
        declared.notMeasuredRanAndSkipped.join(', ')
    );
  }
}

main();
