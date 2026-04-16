import { execFile } from 'node:child_process';
import { promisify } from 'node:util';
import { matchesPalette } from './palette.mjs';

const execFileP = promisify(execFile);

/**
 * Run `git status --porcelain`; empty output means clean.
 */
export async function checkGitClean({ cwd }) {
  try {
    const { stdout } = await execFileP('git', ['status', '--porcelain'], {
      cwd,
      env: process.env,
    });
    if (stdout.trim().length > 0) {
      return {
        ok: false,
        reason: `git has untracked or uncommitted changes:\n${stdout.trim()}`,
      };
    }
    return { ok: true };
  } catch (err) {
    return { ok: false, reason: `git status failed: ${err.message}` };
  }
}

/**
 * Run the Objective's measure command; parse stdout as a number.
 * Returns the baseline for iteration 1's delta tracking.
 */
export async function checkMeasureRuns({ cmd }) {
  try {
    const { stdout } = await execFileP('sh', ['-c', cmd], { env: process.env });
    const value = Number(stdout.trim());
    if (Number.isNaN(value)) {
      return {
        ok: false,
        reason: `measure command did not return a numeric value (got: ${JSON.stringify(stdout.trim().slice(0, 120))})`,
      };
    }
    return { ok: true, baseline: value };
  } catch (err) {
    return { ok: false, reason: `measure command failed: ${err.message}` };
  }
}

/**
 * Compare planned commands against the palette. Returns the set that would
 * trigger permission prompts.
 */
export function checkPaletteGaps({ palette, planned }) {
  const missing = [];
  for (const cmd of planned) {
    if (!matchesPalette(cmd, palette)) missing.push(cmd);
  }
  return { ok: missing.length === 0, missing };
}

/**
 * CLI entry point. Reads an Objective YAML, runs all applicable checks,
 * emits a structured JSON readiness report to stdout. Exit 0 if ready,
 * 1 if any check failed.
 */
export async function runReadiness({ objectivePath }) {
  const { readFileSync } = await import('node:fs');
  // Minimal YAML-ish loader: expect the Objective file to be JSON or
  // properly-structured YAML. For simplicity in v1, accept JSON.
  const obj = JSON.parse(readFileSync(objectivePath, 'utf8'));
  const reports = {};

  reports.measure = await checkMeasureRuns({ cmd: obj.measure.run });
  reports.git = await checkGitClean({ cwd: process.cwd() });

  const ok = Object.values(reports).every((r) => r.ok);
  const out = {
    ready: ok,
    checks: reports,
    baseline: reports.measure.ok ? reports.measure.baseline : null,
  };
  process.stdout.write(JSON.stringify(out, null, 2) + '\n');
  process.exit(ok ? 0 : 1);
}

// Invoked directly?
if (import.meta.url === `file://${process.argv[1]}`) {
  const idx = process.argv.indexOf('--objective');
  const objectivePath = idx >= 0 ? process.argv[idx + 1] : null;
  if (!objectivePath) {
    console.error('Usage: node genesis/agentic/readiness.mjs --objective <path>');
    process.exit(2);
  }
  runReadiness({ objectivePath }).catch((e) => {
    console.error(e);
    process.exit(2);
  });
}
