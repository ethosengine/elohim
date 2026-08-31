/**
 * Shift measure (frozen oracle for 2026-08-31 fleet-carried-election-convergence):
 * prints ONE number.
 *
 *   1  — both alpha doorways serve HTTP 200 on /db/content/<probeId>/head with
 *        IDENTICAL headActionHash, equal to the expected elected head recorded
 *        by the authoring step.
 *   0  — anything else (probe file missing, either doorway failing, divergent
 *        heads, or heads not yet moved to the elected target).
 *
 * The probe state is written by the AUTHORING step (an act, not the judge) at
 * genesis/a2o/reports/carried-election-fleet-probe.json:
 *   { "probeId": "<epr id>", "expectedHead": "<uhCkk…>" }
 */
import { readFileSync } from 'fs';

const DOORWAYS = ['https://doorway-alpha.elohim.host', 'https://elohim.host'];
const PROBE_FILE = new URL('../reports/carried-election-fleet-probe.json', import.meta.url);

async function head(base: string, id: string): Promise<string | null> {
  try {
    const r = await fetch(`${base}/db/content/${id}/head`, {
      signal: AbortSignal.timeout(15000),
    });
    if (!r.ok) return null;
    const j: any = await r.json();
    return j?.headActionHash ?? null;
  } catch {
    return null;
  }
}

async function main() {
  let probe: { probeId: string; expectedHead: string };
  try {
    probe = JSON.parse(readFileSync(PROBE_FILE, 'utf8'));
  } catch {
    console.log(0);
    return;
  }
  const heads = await Promise.all(DOORWAYS.map((d) => head(d, probe.probeId)));
  const ok =
    heads.every((h) => h !== null && h === probe.expectedHead) &&
    new Set(heads).size === 1;
  console.log(ok ? 1 : 0);
}
main();
