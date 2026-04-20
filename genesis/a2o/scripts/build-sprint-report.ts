import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import * as AjvNs from 'ajv/dist/2020.js';
import * as addFormatsNs from 'ajv-formats';

// AJV v8 dual-CJS/ESM packaging means the default export is sometimes wrapped.
// Unwrap both at runtime using namespace imports to avoid TypeScript's
// "no construct signatures" error on default imports from CJS modules.
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const AjvCtor: new (opts: { strict: boolean; allErrors: boolean }) => AjvNs.default =
  (AjvNs as any).default ?? AjvNs;
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const addFormatsFn: (ajv: AjvNs.default) => void = (addFormatsNs as any).default ?? addFormatsNs;

import { loadCucumber } from './lib/load-cucumber.js';
import { loadConsoleArtifacts } from './lib/load-console.js';
import { loadCoverageGap } from './lib/load-coverage-gap.js';
import { aggregate } from './lib/aggregate.js';
import { renderMarkdown } from './lib/render-markdown.js';

interface Args {
  reportsDir: string;
  cucumberPath: string;
  consoleDir: string;
  coverageGapPath: string;
  outJson: string;
  outMd: string;
  runId: string;
  profile: string;
  doorway?: string;
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
    doorway: opts.get('--doorway') ?? process.env.E2E_DOORWAY_ALPHA,
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

  const report = aggregate({
    scenarios,
    consoleArtifacts,
    gaps,
    runId: args.runId,
    profile: args.profile,
    doorway: args.doorway,
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
}

main();
