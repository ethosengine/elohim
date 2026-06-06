#!/usr/bin/env node
// Link-integrity static gate (spec §7.1 of
// genesis/docs/superpowers/specs/2026-06-06-epr-route-claims-link-conformance-design.md):
// links are MINTED (eprToRoute / eprToUniversalHref / claims), never literal.
// Generalizes the pillar-bundle-split runbook §4.4 router-literal canary.
// Keepers: a line containing `route-literal-ok: <reason>` is exempt; <base href>,
// SEO canonical generators, and doc comments should carry the pragma.
import { execSync } from 'node:child_process';

const TARGETS = process.argv.slice(2);
if (TARGETS.length === 0) {
  console.error('usage: lint-route-literals.mjs <srcDir> [...more]');
  process.exit(2);
}
// Forbidden literal route minting patterns (single-quoted, double-quoted, or
// backtick template literals — the dominant mint form).
const PATTERNS = [
  "'/lamad",
  '"/lamad',
  '`/lamad',
  "'/resource/",
  '"/resource/',
  '`/resource/',
  "'/epr/",
  '"/epr/',
  '`/epr/',
];
let failures = 0;
for (const dir of TARGETS) {
  for (const pat of PATTERNS) {
    let out = '';
    try {
      // Pass the pattern via env, not the command string: backtick template
      // patterns (`/epr/) would otherwise be parsed as shell command
      // substitution. -F = fixed string (these are literal substrings).
      out = execSync(
        `grep -rnF -e "$ROUTE_LINT_PAT" ${dir} --include='*.ts' --include='*.html'`,
        { encoding: 'utf8', env: { ...process.env, ROUTE_LINT_PAT: pat } },
      );
    } catch {
      continue; // grep exit 1 = no matches
    }
    for (const line of out.split('\n').filter(Boolean)) {
      if (line.includes('route-literal-ok:')) continue;
      if (line.includes('.spec.ts:')) continue; // tests assert minted output
      console.error(`route literal: ${line}`);
      failures += 1;
    }
  }
}
if (failures > 0) {
  console.error(`\n${failures} raw route literal(s). Mint via eprToRoute/eprToUniversalHref/claims,`);
  console.error(`or annotate a documented keeper with: // route-literal-ok: <reason>`);
  process.exit(1);
}
console.log('lint-route-literals: clean');
