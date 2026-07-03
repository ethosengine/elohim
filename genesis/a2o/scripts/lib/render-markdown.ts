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
  if (rollup.pending > 0) return '◌';
  return '✅';
}

function renderByConcern(report: SprintReport): string[] {
  const byConcern = report.summary.byConcern ?? {};
  const concerns = Object.keys(byConcern).sort((a, b) => a.localeCompare(b));
  if (concerns.length === 0) return [];

  const lines: string[] = [];
  lines.push(`## Dataplane validation by concern`);
  lines.push('');
  lines.push(`| concern | status | passed | failed | pending |`);
  lines.push(`|---|---|---|---|---|`);
  for (const name of concerns) {
    const r = byConcern[name];
    lines.push(`| \`${name}\` | ${concernGlyph(r)} | ${r.passed} | ${r.failed} | ${r.pending} |`);
  }
  lines.push('');

  for (const name of concerns) {
    const r = byConcern[name];
    lines.push(`### ${concernGlyph(r)} \`${name}\``);
    lines.push('');
    for (const s of r.scenarios) {
      let glyph = '◌';
      if (s.status === 'passed') glyph = '✅';
      else if (s.status === 'failed') glyph = '❌';
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
