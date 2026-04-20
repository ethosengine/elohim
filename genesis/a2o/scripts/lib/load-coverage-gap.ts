import { readFileSync, existsSync } from 'node:fs';

export interface GapFinding {
  feature: string;
  missing: string;
  severity: 'low' | 'medium' | 'high';
}

interface GapFileShape {
  gaps?: Array<{ feature: string; missing: string; severity?: string }>;
}

export function loadCoverageGap(path: string): GapFinding[] {
  if (!existsSync(path)) return [];
  const raw = JSON.parse(readFileSync(path, 'utf8')) as GapFileShape;
  const out: GapFinding[] = [];
  for (const gap of raw.gaps ?? []) {
    const severity = (gap.severity as GapFinding['severity']) ?? 'medium';
    out.push({ feature: gap.feature, missing: gap.missing, severity });
  }
  return out;
}
