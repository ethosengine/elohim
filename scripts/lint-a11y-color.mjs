#!/usr/bin/env node
// A11y paired-color canary — enforces the theme-authority gospel rule
// ("every bound *-bg paired with a bound *-fg"; html[data-theme] is the
// authority) as a static, pre-push gate for Angular SHELL components.
//
// WHY THIS EXISTS (the epr-raw-node dark-on-dark bug, 2026-06-08):
//   A component set `color: var(--epr-raw-fg, #1f2430)` with NO paired
//   background. `--epr-raw-fg` was never defined → it resolved to the
//   hardcoded near-black literal in BOTH themes. On the dark constellation
//   canvas (painted by `color-scheme: dark`, not a readable background-color)
//   the text was invisible. axe-core scored it CLEAN — it resolves transparent
//   backgrounds to an assumed white and never models the canvas. Static analysis
//   (SonarQube S7924) and the inert stylelint-a11y plugin couldn't see it either.
//   The shell's real contrast gate lives only in elohim-core (Lit). This is the
//   shell twin's cheap static layer; the both-themes render gate is the dynamic
//   layer (see genesis backlog: a11y-contrast-gate).
//
// THE RULE: a `color:` / `background:` / `background-color:` declaration whose
// value contains a hardcoded color literal (hex / rgb() / hsl()) MUST also
// reference a theme-reactive token (var(--lamad-* | --elohim-* | --el-* |
// --primary | --accent | --text-*)). A literal that never chains to a theme
// token won't adapt across themes — flag it. Local override-seam vars
// (var(--epr-raw-fg, …)) are fine ONLY when their fallback chain bottoms out in
// a theme token. Keepers (sanctioned semantic status colors, on-image scrims,
// brand-locked surfaces) carry `a11y-color-ok: <reason>`.
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { execSync } from 'node:child_process';

// `--diff <baseRef>` = RATCHET mode: report only violations on lines ADDED vs
// baseRef. The shell carries ~1086 pre-existing literals (legacy debt tracked in
// genesis backlog: a11y-contrast-gate); a full sweep is a migration, not a gate.
// The ratchet grandfathers that debt and blocks NEW literals as they land — the
// boy-scout rule. Whole-dir mode (no --diff) is for the already-clean subtrees
// (the /epr viewer surface) and CI conformance once a subtree is migrated.
const ARGV = process.argv.slice(2);
let diffBase = null;
const TARGETS = [];
for (let i = 0; i < ARGV.length; i++) {
  if (ARGV[i] === '--diff') { diffBase = ARGV[++i]; continue; }
  TARGETS.push(ARGV[i]);
}
if (TARGETS.length === 0) {
  console.error('usage: lint-a11y-color.mjs [--diff <baseRef>] <srcDir> [...more]');
  process.exit(2);
}

/** Lines added (vs baseRef) per file, from `git diff -U0` hunk headers. */
function addedLines(base, file) {
  const set = new Set();
  let out = '';
  try {
    out = execSync(`git diff -U0 ${base} -- "${file}"`, { encoding: 'utf8' });
  } catch { return set; } // not tracked / no diff → nothing added
  for (const line of out.split('\n')) {
    const m = /^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@/.exec(line);
    if (!m) continue;
    const start = Number(m[1]);
    const count = m[2] === undefined ? 1 : Number(m[2]);
    for (let n = start; n < start + count; n++) set.add(n);
  }
  return set;
}

// Properties whose value participates in foreground/background contrast.
const PROP = /(?:^|[;{(\s])(color|background|background-color)\s*:\s*([^;}]+)/gi;
// A hardcoded color literal: #rgb / #rrggbb / #rrggbbaa, or rgb()/rgba()/hsl()/hsla().
const LITERAL = /#[0-9a-fA-F]{3,8}\b|\b(?:rgba?|hsla?)\s*\(/;
// A theme-reactive token reference — the value is theme-safe if it cites one.
// Tight on purpose: the canonical reactive namespaces are --lamad-* (shell +
// lamad content), --elohim-*/--el-* (elohim-core palette), and the shell's
// --primary/--accent/--text-light. Generic names like --text-primary or
// --surface-elevated are NOT reactive (undefined → they fall to their light
// literal), so they must NOT be whitelisted — that was the exploration-sidebar
// theme-blindness the loose `--text-` namespace originally hid.
const THEME_TOKEN = /var\(\s*--(?:lamad|elohim|el|primary\b|accent|text-light)/;
// Values that carry no real literal even if they look colorful.
const SAFE_KEYWORD = /^\s*(?:inherit|initial|unset|transparent|currentColor|none)\s*$/i;

function walk(dir, acc) {
  let entries;
  try { entries = readdirSync(dir); } catch { return acc; }
  for (const e of entries) {
    const p = join(dir, e);
    let st;
    try { st = statSync(p); } catch { continue; }
    if (st.isDirectory()) {
      if (e === 'node_modules' || e === 'dist' || e === '.angular') continue;
      walk(p, acc);
    } else if (/\.component\.(css|ts)$/.test(e) && !/\.spec\.ts$/.test(e)) {
      acc.push(p);
    }
  }
  return acc;
}

let failures = 0;
let grandfathered = 0;
const files = TARGETS.flatMap(t => walk(t, []));
for (const file of files) {
  const added = diffBase ? addedLines(diffBase, file) : null;
  if (added && added.size === 0) continue; // ratchet: file unchanged vs base
  const lines = readFileSync(file, 'utf8').split('\n');
  lines.forEach((line, i) => {
    if (line.includes('a11y-color-ok:')) return;
    const trimmed = line.trimStart();
    if (/^(\*|\/\/|\/\*)/.test(trimmed)) return; // comment line — describes, doesn't paint
    PROP.lastIndex = 0;
    let m;
    while ((m = PROP.exec(line)) !== null) {
      const value = m[2].trim();
      if (SAFE_KEYWORD.test(value)) continue;
      if (!LITERAL.test(value)) continue; // token-only or keyword — theme-safe
      if (THEME_TOKEN.test(value)) continue; // literal but chains to a theme token
      if (added && !added.has(i + 1)) { grandfathered += 1; continue; } // legacy line — ratcheted
      console.error(`a11y color: ${file}:${i + 1}:  ${m[1]}: ${value}`);
      failures += 1;
    }
  });
}

if (diffBase) {
  console.error(`(ratchet vs ${diffBase}: ${grandfathered} pre-existing literal(s) grandfathered)`);
}
if (failures > 0) {
  console.error(`\n${failures} NEW hardcoded color(s) not bound to a theme token.`);
  console.error(`Bind to var(--lamad-*/--elohim-*/--el-*/--primary/--accent/--text-*) (theme-reactive,`);
  console.error(`html[data-theme] authority), or annotate a sanctioned keeper: /* a11y-color-ok: <reason> */`);
  process.exit(1);
}
console.log('lint-a11y-color: clean');
