#!/usr/bin/env node
// CI Preview: shows exactly what the orchestrator will decide for a changeset.
//
// Usage:
//   node preview.mjs [base-ref]
//
// Default base ref: origin/dev
// Computes: git diff --name-only <base>..HEAD plus uncommitted
// (staged + unstaged + untracked) then runs the same simulate() logic
// that the orchestrator uses in Jenkins.
//
// The algorithm is imported from ./orchestrator-strategy.mjs — the
// pure-function module shared with orchestrator-strategy.test.mjs. So
// what you see locally is what Jenkins will decide.

import { execFileSync } from 'node:child_process';

import { simulate, PIPELINES } from './orchestrator-strategy.mjs';

const baseRef = process.argv[2] || 'origin/dev';

function gitDiffNames(base) {
  try {
    const out = execFileSync('git', ['diff', '--name-only', `${base}..HEAD`], {
      encoding: 'utf8',
    });
    return out.split('\n').filter(Boolean);
  } catch (e) {
    console.error(`WARNING: git diff against ${base} failed: ${e.message}`);
    return [];
  }
}

function uncommittedNames() {
  try {
    const out = execFileSync('git', ['status', '--porcelain'], { encoding: 'utf8' });
    return parsePorcelain(out);
  } catch (e) {
    console.error(`WARNING: git status failed: ${e.message}`);
    return [];
  }
}

// Parse `git status --porcelain` v1 output. Handles renames/copies
// (`R  old -> new`) by emitting BOTH paths so old-path source globs still
// match. Strips porcelain v1 quoting from filenames with special chars.
//
// Limitation: paths containing C-escapes (newlines, tabs) are best-effort —
// the unquoting strips outer quotes but does not interpret backslash escapes.
// Acceptable for a dev preview tool; rerun with cleaner filenames if needed.
function parsePorcelain(output) {
  const files = [];
  for (const rawLine of output.split('\n')) {
    if (!rawLine) continue;
    // Porcelain v1: 'XY filename' (cols 1-2 = status, col 3 = space)
    const status = rawLine.slice(0, 2);
    const rest = rawLine.slice(3);

    // Rename: 'R  old -> new' (or 'C  old -> new' for copy)
    if (status[0] === 'R' || status[0] === 'C') {
      const arrow = rest.indexOf(' -> ');
      if (arrow >= 0) {
        const oldPath = unquote(rest.slice(0, arrow));
        const newPath = unquote(rest.slice(arrow + 4));
        files.push(oldPath, newPath);
        continue;
      }
    }
    files.push(unquote(rest));
  }
  return files;
}

function unquote(s) {
  if (s.length >= 2 && s.startsWith('"') && s.endsWith('"')) {
    return s.slice(1, -1);
  }
  return s;
}

const committed = gitDiffNames(baseRef);
const pending = uncommittedNames();
const all = [...new Set([...committed, ...pending])];

console.log(`\nCI Preview (base: ${baseRef})`);
console.log(
  `  ${committed.length} committed, ${pending.length} uncommitted, ${all.length} unique\n`,
);

const { pipelines, analysis, skipped } = simulate({ changedFiles: all });

if (skipped) {
  console.log('  [skip ci] detected — orchestrator would skip all pipelines.\n');
}

console.log('+--------------------------------------------------------------------------+');
console.log('|                     ORCHESTRATOR DECISION (preview)                      |');
console.log('+--------------------------------------------------------------------------+');

const allNames = Object.keys(PIPELINES).sort();
for (const name of allNames) {
  const info = analysis[name];
  const willRun = (info && info.shouldRun) || pipelines.includes(name);
  const manualOnly = info && info.manualOnly;
  const marker = willRun ? '[BUILD] ' : manualOnly ? '[MANUAL]' : '[SKIP]  ';
  const status = willRun ? 'BUILD ' : manualOnly ? 'MANUAL' : 'SKIP  ';
  const reasons =
    info && info.matchedPatterns && info.matchedPatterns.length
      ? info.matchedPatterns.slice(0, 2).join(', ')
      : 'no matching files';
  let line = `| ${marker} ${name.padEnd(20)} | ${status} | ${reasons}`;
  if (line.length > 74) {
    line = line.slice(0, 71) + '...';
  }
  console.log(line.padEnd(75) + '|');
}
console.log('+--------------------------------------------------------------------------+');

// Per-file breakdown (the "why was this file skipped?" view)
console.log('\nPer-file routing:');
const fileToPipelines = new Map();
for (const f of all) fileToPipelines.set(f, []);
for (const [pname, info] of Object.entries(analysis)) {
  for (const f of info.sampleFiles || []) {
    if (fileToPipelines.has(f)) fileToPipelines.get(f).push(pname);
  }
}
for (const f of all.slice(0, 20)) {
  const pipes = fileToPipelines.get(f);
  const tag = pipes && pipes.length ? `-> ${pipes.join(', ')}` : '-> (skipped: no pattern match)';
  console.log(`  ${f}  ${tag}`);
}
if (all.length > 20) console.log(`  ... and ${all.length - 20} more`);

// Note: sampleFiles is capped at 5 per pipeline, so pipelines with >5
// matched files will show "(skipped: no pattern match)" for files 6+
// in the per-file view. The BUILD/SKIP matrix above is still accurate.
if (all.length > 5) {
  console.log(
    '\n  note: per-file routing uses sampleFiles (capped at 5 per pipeline);',
  );
  console.log(
    '        the decision matrix above is the authoritative view.',
  );
}

console.log(
  `\nPipelines that will run (in order): ${pipelines.join(' -> ') || '(none)'}\n`,
);
