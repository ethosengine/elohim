import { producedVerdict } from './aggregate.js';

import type { SprintReport, Finding, ConcernRollup } from './aggregate.js';

function renderFinding(f: Finding): string[] {
  const lines: string[] = [];
  lines.push(`### [${f.source}] \`${f.fingerprint}\` (occurrences: ${f.occurrences})`);
  lines.push('');
  lines.push(`> ${f.message}`);
  lines.push('');
  if (f.firstSeenUrl) lines.push(`- URL: ${f.firstSeenUrl}`);
  if (f.screenshotPath) lines.push(`- **Screenshot**: \`${f.screenshotPath}\``);
  lines.push(`- **Objective**: ${f.suggestedObjective}`);
  lines.push('');
  lines.push(`<details><summary>Scenarios (${f.scenarios.length})</summary>`);
  lines.push('');
  for (const s of f.scenarios) {
    const who = s.human ? ` — ${s.human}` : '';
    lines.push(`- \`${s.feature}\` · ${s.name}${who}`);
  }
  lines.push('');
  lines.push(`</details>`);
  lines.push('');
  return lines;
}

function concernGlyph(rollup: ConcernRollup): string {
  if (rollup.failed > 0) return '❌';
  // No verdict at all is NOT MEASURED, never a tick and never a soft "pending".
  if (!producedVerdict(rollup)) return '⊘';
  if (rollup.pending > 0) return '◌';
  return '✅';
}

/**
 * The two causes of an absence have different first moves: an apparatus that
 * ran and skipped is a fixture/gate problem; a concern with no scenario at all
 * is a scope, tag-filter or registry problem.
 */
function whyNotMeasured(
  name: string,
  ranAndSkipped: ReadonlySet<string>,
  byConcern: Record<string, ConcernRollup>
): string {
  if (!ranAndSkipped.has(name)) {
    return 'no scenario for it in this run at all — scope, tag filter, or registry';
  }
  const r = byConcern[name];
  const skipped = r?.skipped ?? 0;
  const noVerdict = r?.pending ?? 0;
  if (skipped === noVerdict) {
    return `all ${noVerdict} scenarios ran and SKIPPED — apparatus/fixture, not scope`;
  }
  return `${noVerdict} scenarios ran, none produced a verdict (${skipped} skipped) — apparatus/fixture`;
}

/**
 * Law I's headline, in the archived human-readable artifact — not only in the
 * JSON and not only on the CI console. Five numbers forever:
 * declared · permitted · refused · referred · not measured.
 */
function renderDeclared(report: SprintReport): string[] {
  const declared = report.declared;
  if (!declared) return [];
  const byConcern = report.summary.byConcern ?? {};

  let permitted = 0;
  let refused = 0;
  for (const name of declared.exercised) {
    const r = byConcern[name];
    if (!r) continue;
    if (r.failed > 0) refused += 1;
    else permitted += 1;
  }

  const lines: string[] = [];
  lines.push(`## Declared (Law I — the denominator is anchored outside the run)`);
  lines.push('');
  if (declared.unreadable) {
    lines.push(
      `> ⛔ **DENOMINATOR UNREADABLE** — ${declared.unreadable}. Every count below is 0 because ` +
        `the covenant could not be read, NOT because coverage is complete. Do not read this run as clean.`
    );
    lines.push('');
  }
  lines.push(
    `**declared ${declared.concerns.length} · permitted ${permitted} · refused ${refused} · ` +
      `referred 0 · NOT MEASURED ${declared.notMeasured.length}**`
  );
  lines.push('');
  lines.push(`- Source: \`${declared.source}\``);
  lines.push(
    `- \`referred\` is structurally 0: \`Decision::Refer\` has no producer on this path yet ` +
      `(guidestar §S5). It is rendered so a future producer cannot arrive silently.`
  );
  if (declared.notMeasured.length > 0) {
    const skippedSet = new Set(declared.notMeasuredRanAndSkipped);
    lines.push('');
    lines.push(`| not measured | why |`);
    lines.push(`|---|---|`);
    for (const name of declared.notMeasured) {
      lines.push(`| \`${name}\` | ${whyNotMeasured(name, skippedSet, byConcern)} |`);
    }
  }
  lines.push('');
  return lines;
}

/** Law II — the environment component of the key, including the system under test. */
function renderEnv(report: SprintReport): string[] {
  const env = report.env;
  if (!env) return [];
  const lines: string[] = [];
  lines.push(`## Environment (Law II — one measure, many environments)`);
  lines.push('');
  lines.push(`| lane | peers | processControl | networkStage | sut |`);
  lines.push(`|---|---|---|---|---|`);
  lines.push(
    `| ${env.lane} | ${env.peers} | ${env.processControl} | ${env.networkStage} | \`${env.sut}\` |`
  );
  lines.push('');
  if (report.gitCommit) lines.push(`- **Commit measured**: \`${report.gitCommit}\``);
  const parts = Object.entries(env.sutParts)
    .sort(([a], [b]) => a.localeCompare(b))
    .map(([k, v]) => `${k} \`${v}\``);
  if (parts.length > 0) lines.push(`- **sutParts**: ${parts.join(' · ')}`);
  lines.push(
    env.unknown.length > 0
      ? `- **unknown** (declared, never omitted): ${env.unknown.join(', ')}`
      : `- **unknown**: none — this run claims every key component was hashable`
  );
  lines.push('');
  return lines;
}

function renderByConcern(report: SprintReport): string[] {
  const byConcern = report.summary.byConcern ?? {};
  const concerns = Object.keys(byConcern).sort((a, b) => a.localeCompare(b));
  if (concerns.length === 0) return [];

  const lines: string[] = [];
  lines.push(`## Dataplane validation by concern`);
  lines.push('');
  lines.push(`| concern | status | passed | failed | skipped | no verdict |`);
  lines.push(`|---|---|---|---|---|---|`);
  for (const name of concerns) {
    const r = byConcern[name];
    lines.push(
      `| \`${name}\` | ${concernGlyph(r)} | ${r.passed} | ${r.failed} | ${r.skipped} | ${r.pending} |`
    );
  }
  lines.push('');
  lines.push(`\`⊘\` = NOT MEASURED: the concern produced no verdict at all (guidestar §S3).`);
  lines.push('');

  for (const name of concerns) {
    const r = byConcern[name];
    lines.push(`### ${concernGlyph(r)} \`${name}\``);
    lines.push('');
    for (const s of r.scenarios) {
      let glyph = '◌';
      if (s.status === 'passed') glyph = '✅';
      else if (s.status === 'failed') glyph = '❌';
      else if (s.status === 'skipped') glyph = '⊘';
      lines.push(`- ${glyph} ${s.name} — \`${s.surface}\``);
    }
    lines.push('');
  }

  return lines;
}

function renderVisualValidation(report: SprintReport): string[] {
  const v = report.summary.visualValidation;
  if (!v) return [];
  const lines: string[] = [];
  lines.push(`## Visual Validation`);
  lines.push('');
  lines.push(`|  | passed | failed |`);
  lines.push(`|---|---|---|`);
  lines.push(
    `| has \`@elohim-visually-validated\` | ${v.validatedPassing} | **${v.validatedRegressed}** |`
  );
  lines.push(`| no tag (pending review) | ${v.pendingPassing} | ${v.pendingFailing} |`);
  lines.push('');
  lines.push(`- **${v.validatedPassing}** validatedPassing — confirmed delivering as designed`);
  lines.push(
    `- **${v.validatedRegressed}** validatedRegressed — see \`visual-regression\` findings below`
  );
  lines.push(`- **${v.pendingPassing}** pendingPassing — candidates for review`);
  lines.push(`- **${v.pendingFailing}** pendingFailing — see \`scenario-failure\` findings below`);
  lines.push('');
  return lines;
}

export function renderMarkdown(report: SprintReport): string {
  const lines: string[] = [];
  lines.push(`# A2O Sprint Report`);
  lines.push('');
  lines.push(`- **Run**: \`${report.runId}\``);
  lines.push(`- **Profile**: \`${report.profile}\``);
  if (report.transport) lines.push(`- **Transport**: \`${report.transport}\``);
  if (report.doorway) lines.push(`- **Doorway**: ${report.doorway}`);
  lines.push(`- **Generated**: ${report.generatedAt}`);
  lines.push('');
  lines.push(`## Summary`);
  lines.push('');
  lines.push(`| scenarios | passed | failed | skipped | pending |`);
  lines.push(`|---|---|---|---|---|`);
  lines.push(
    `| ${report.summary.scenarios.total} | ${report.summary.scenarios.passed} | ${report.summary.scenarios.failed} | ${report.summary.scenarios.skipped} | ${report.summary.scenarios.pending} |`
  );
  lines.push('');
  lines.push(
    `Passed: **${report.summary.scenarios.passed}** | Failed: **${report.summary.scenarios.failed}** | Findings total: **${report.summary.findings.total}**`
  );
  lines.push('');

  lines.push(...renderDeclared(report));
  lines.push(...renderEnv(report));
  lines.push(...renderByConcern(report));
  lines.push(...renderVisualValidation(report));

  const byPillar = new Map<string, Finding[]>();
  for (const f of report.findings) {
    const arr = byPillar.get(f.pillar) ?? [];
    arr.push(f);
    byPillar.set(f.pillar, arr);
  }

  for (const [pillar, findings] of [...byPillar.entries()].sort(([a], [b]) => a.localeCompare(b))) {
    lines.push(`## ${pillar}`);
    lines.push('');
    for (const f of findings) {
      lines.push(...renderFinding(f));
    }
  }

  return lines.join('\n');
}
