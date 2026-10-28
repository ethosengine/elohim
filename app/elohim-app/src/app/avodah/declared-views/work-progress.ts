import { field, type Reading } from './view-reading';

import type { JsonValue, Subject } from './view-contract';

export function object(value: JsonValue | undefined): Record<string, JsonValue | undefined> {
  return value && typeof value === 'object' && !Array.isArray(value) ? value : {};
}
export function strings(value: JsonValue | undefined): string[] {
  return Array.isArray(value) ? value.filter((v): v is string => typeof v === 'string') : [];
}
export function claims(subject: Subject): { text: string; line: number }[] {
  const value = subject.fields['$claims'];
  return Array.isArray(value)
    ? value
        .map(v => object(v))
        .map(v => ({ text: String(v['text'] ?? ''), line: Number(v['line']) }))
    : [];
}
function assertions(subject: Subject | undefined): Map<string, string> {
  const out = new Map<string, string>();
  if (!subject) return out;
  for (const [name, reading] of Object.entries(subject.readings)) {
    const r = object(reading);
    if (r['state'] !== 'available') continue;
    const values = object(r['value'])['assertions'];
    if (!Array.isArray(values)) continue;
    for (const value of values) {
      const a = object(value);
      const commitments = a['commitments'];
      const commitmentKey = Array.isArray(commitments)
        ? commitments
            .map(c => object(c)['commitment'])
            .filter((v): v is string => typeof v === 'string')
            .sort((a, b) => a.localeCompare(b))
            .join(',')
        : '';
      const key = a['gap_id'] ?? a['intent'] ?? (commitmentKey || undefined);
      if (typeof key === 'string' && typeof a['acceptance'] === 'string')
        out.set(`${name}/${key}`, a['acceptance']);
    }
  }
  return out;
}
export function movement(subject: Subject, current: Reading, previous?: Reading): string {
  if (!previous) return 'Baseline established';
  if (
    current.observation.declarationCid !== previous.observation.declarationCid ||
    JSON.stringify(current.projection.reader_versions) !==
      JSON.stringify(previous.projection.reader_versions)
  )
    return 'Interpretation changed — review evidence';
  const before = previous.projection.subjects.find(s => s.identity === subject.identity);
  if (!before) return 'Added since comparison';
  const now = assertions(subject),
    old = assertions(before);
  if (
    [...old].some(
      ([key, value]) => value === 'accepted' && now.has(key) && now.get(key) !== 'accepted'
    )
  )
    return 'Acceptance reopened';
  if ([...now].some(([key, value]) => value === 'accepted' && old.get(key) !== 'accepted'))
    return 'New verified acceptance';
  if ([...old.keys()].some(key => !now.has(key))) return 'Evidence coverage changed';
  if (
    JSON.stringify(claims(before).map(c => c.text)) !==
    JSON.stringify(claims(subject).map(c => c.text))
  )
    return 'Authored claims changed';
  if (before.version !== subject.version) return 'Authored source changed';
  if (JSON.stringify(before.readings) !== JSON.stringify(subject.readings))
    return 'Evidence changed — inspect readings';
  return 'Unchanged reading';
}
function assertionValues(subject: Subject): Record<string, JsonValue | undefined>[] {
  return Object.values(subject.readings).flatMap(reading => {
    const r = object(reading);
    const values = object(r['value'])['assertions'];
    return r['state'] === 'available' && Array.isArray(values) ? values.map(object) : [];
  });
}
export function evidenceSummary(subject: Subject): string {
  const values = assertionValues(subject);
  if (!values.length) return 'Structured evidence unavailable';
  const hasEvidence = (a: Record<string, JsonValue | undefined>, name: string): boolean => {
    const commitments = a['commitments'];
    return Array.isArray(commitments) && commitments.some(c => strings(object(c)[name]).length > 0);
  };
  const produced = values.filter(a => hasEvidence(a, 'fulfillments')).length;
  const reviewed = values.filter(a => hasEvidence(a, 'technical_reviews')).length;
  const accepted = values.filter(a => a['acceptance'] === 'accepted').length;
  return `${produced} produced · ${reviewed} reviewed · ${accepted}/${values.length} accepted assertions`;
}
export function remainingActions(subject: Subject): string[] {
  const actions: string[] = [];
  for (const reading of Object.values(subject.readings)) {
    const r = object(reading);
    const values = object(r['value'])['assertions'];
    if (r['state'] !== 'available' || !Array.isArray(values)) continue;
    for (const value of values) {
      const a = object(value);
      if (a['acceptance'] !== 'accepted' && typeof a['remaining_action'] === 'string')
        actions.push(a['remaining_action']);
    }
  }
  return actions;
}
export function collection(reading: Reading, group: string): Reading {
  const config = reading.projection.composition?.groups.find(g => g.id === group);
  if (!reading.projection.composition) return reading;
  return {
    ...reading,
    projection: {
      ...reading.projection,
      subjects: reading.projection.subjects.filter(s => field(s, '$group') === group),
    },
    presentation: config
      ? { ...config.presentation, titleField: '$title', statusField: '$status' }
      : reading.presentation,
  };
}
