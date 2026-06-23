#!/usr/bin/env node
// Shift oracle (frozen at kickoff — judge file, do not edit mid-shift).
//
// score = count of {elohim-orchestrator, elohim-genesis} whose LATEST dev build
// is UNSTABLE-or-better. Reads the deterministic ci-harvest ledger
// (.claude/data/ci-cursor.json `recent.<job>`, newest element last), which the
// agentic loop refreshes at Ground (step 1) and after Measure (step 5).
//
// Returns a single integer on stdout: 0 (both red), 1 (one clear), 2 (both
// UNSTABLE-or-better → target).
import fs from 'fs';

const CURSOR = '.claude/data/ci-cursor.json';
const OK = new Set(['UNSTABLE', 'SUCCESS']);

let cursor;
try {
  cursor = JSON.parse(fs.readFileSync(CURSOR, 'utf8'));
} catch (e) {
  // No ledger yet → score 0 (treat as unproven/red, never crash the measure).
  process.stdout.write('0');
  process.exit(0);
}

const latest = (job) => {
  const w = cursor.recent?.[job];
  return Array.isArray(w) && w.length ? w[w.length - 1] : null;
};
const score = (job) => (OK.has(latest(job)) ? 1 : 0);

process.stdout.write(String(score('elohim-orchestrator') + score('elohim-genesis')));
