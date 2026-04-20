import type { SprintReport, Finding } from './aggregate.js';

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

  const byPillar = new Map<string, Finding[]>();
  for (const f of report.findings) {
    const arr = byPillar.get(f.pillar) ?? [];
    arr.push(f);
    byPillar.set(f.pillar, arr);
  }

  for (const [pillar, findings] of [...byPillar.entries()].sort()) {
    lines.push(`## ${pillar}`);
    lines.push('');
    for (const f of findings) {
      lines.push(`### [${f.source}] \`${f.fingerprint}\` (occurrences: ${f.occurrences})`);
      lines.push('');
      lines.push(`> ${f.message}`);
      lines.push('');
      if (f.firstSeenUrl) lines.push(`- URL: ${f.firstSeenUrl}`);
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
    }
  }

  return lines.join('\n');
}
