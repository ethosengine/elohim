import type { Observation, Presentation, Projection, Subject } from './view-contract';

export interface Reading {
  observation: Observation;
  projection: Projection;
  presentation: Presentation;
}
export interface BurnPoint {
  sequence: number;
  time: string;
  authoredRemaining: number;
  acceptanceMin: number;
  acceptanceMax: number;
  unknown: number;
  missing: number;
  additions: number;
  available: boolean;
  declaration: string;
}
export function field(subject: Subject, name: string): string {
  const value = name === '$identity' ? subject.identity : subject.fields[name];
  return typeof value === 'string' ? value : '';
}
export function presentation(value: Partial<Presentation> | undefined): Presentation {
  return {
    titleField: value?.titleField ?? '$identity',
    statusField: value?.statusField ?? '$identity',
    columns: value?.columns ?? [],
    terminalValues: value?.terminalValues ?? [],
    readings: value?.readings ?? [],
    reconciliationReading: value?.reconciliationReading ?? null,
  };
}
/** Only summarize the current native assertion set; never adjudicate its evidence. */
export function acceptance(
  subject: Subject,
  name: string | null
): 'accepted' | 'remaining' | 'unknown' {
  const reading = name ? subject.readings[name] : null;
  if (!reading || typeof reading !== 'object' || Array.isArray(reading)) return 'unknown';
  const r = reading as { state?: string; value?: { assertions?: { acceptance?: string }[] } };
  const assertions = r.value?.assertions;
  if (r.state !== 'available' || !Array.isArray(assertions) || assertions.length === 0)
    return 'unknown';
  if (assertions.some(a => typeof a.acceptance !== 'string')) return 'unknown';
  return assertions.every(a => a.acceptance === 'accepted') ? 'accepted' : 'remaining';
}
/** Fixed baseline cohort. Filters never enter this function. Each point uses its captured declaration. */
export function burndown(readings: Reading[], baseline: Reading): BurnPoint[] {
  const cohort = new Set(baseline.projection.subjects.map(s => s.identity));
  return readings
    .filter(r => r.observation.sequence >= baseline.observation.sequence)
    .map(r => {
      const current = new Map(r.projection.subjects.map(s => [s.identity, s]));
      let authored = 0,
        accepted = 0,
        unknown = 0,
        missing = 0,
        known = 0;
      for (const id of cohort) {
        const subject = current.get(id);
        if (!subject) {
          missing++;
          unknown++;
          continue;
        }
        if (r.presentation.terminalValues.includes(field(subject, r.presentation.statusField)))
          authored++;
        const verdict = acceptance(subject, r.presentation.reconciliationReading);
        if (verdict === 'unknown') unknown++;
        else {
          known++;
          if (verdict === 'accepted') accepted++;
        }
      }
      return {
        sequence: r.observation.sequence,
        time: r.observation.observedAt,
        authoredRemaining: cohort.size - authored,
        acceptanceMin: cohort.size - accepted - unknown,
        acceptanceMax: cohort.size - accepted,
        unknown,
        missing,
        additions: [...current.keys()].filter(id => !cohort.has(id)).length,
        available: known > 0,
        declaration: r.observation.declarationCid,
      };
    });
}
