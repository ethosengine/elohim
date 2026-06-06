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
// Forbidden literal route minting patterns (single- or double-quoted).
const PATTERNS = ["'/lamad", '"/lamad', "'/resource/", '"/resource/', "'/epr/", '"/epr/'];
let failures = 0;
for (const dir of TARGETS) {
  for (const pat of PATTERNS) {
    let out = '';
    try {
      out = execSync(
        `grep -rn ${JSON.stringify(pat)} ${dir} --include='*.ts' --include='*.html'`,
        { encoding: 'utf8' },
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
