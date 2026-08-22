/**
 * The denominator, declared OUTSIDE the run (Law I, guidestar 2026-08-22 §3).
 *
 * "A run's denominator is the concern set declared in genesis/manifests/
 * habits.yaml `checks:` and the @concern: tag registry — NEVER a set the
 * runner computes for itself." A scoped `just test mesh <tags>` that declared
 * its own denominator would report 100% permitted and zero missing while every
 * other concern went silently unexercised; the 76-of-80 case would have
 * vanished from the report instead of surfacing. So the set is read from the
 * covenant file, and `notMeasured` is derived against it regardless of what
 * the run chose to execute.
 *
 * HOW IT IS PARSED, honestly: this package has no YAML dependency (a2o's
 * package.json carries none), so rather than add one for a tag scan, the
 * `checks:` blocks are located with a small indentation-aware line scan and
 * the @concern: tags are extracted with the same regex habits-status.py's
 * _first_concern already uses. The scan is deliberately block-aware and NOT a
 * whole-file grep: `evidence:` prose quotes @concern: tags constantly (nine
 * places today), and a whole-file regex would silently inflate the denominator
 * with concerns nothing ever declared as a check.
 *
 * Verified at authoring time (2026-08-22) against a real `yaml.safe_load` pass
 * over the same file: both yield the identical 12-concern set. The scan is
 * pinned from both sides in scripts/__tests__/run-env.spec.ts — a synthetic
 * habits fixture whose `evidence:` prose names a concern the checks do not
 * (the exact way a grep would be wrong), and the live habits.yaml.
 */

import { readFileSync } from 'node:fs';

/** Same shape habits-status.py::_first_concern matches. */
const CONCERN_TAG = /@concern:([A-Za-z0-9][\w-]*)/g;

export interface DeclaredConcernSet {
  source: string;
  /**
   * Set ONLY when habits.yaml could not be read. `concerns: []` on its own is
   * silent: every count-based reader — including the builder's own console
   * line — renders `0 declared · 0 not measured`, which is byte-identical to
   * perfect coverage. The failure gets a field so it survives into the report.
   */
  unreadable?: string;
  concerns: string[];
}

function indentOf(line: string): number {
  return line.length - line.trimStart().length;
}

function collectTags(line: string, into: Set<string>): void {
  for (const match of line.matchAll(CONCERN_TAG)) into.add(match[1]);
}

/** Blank lines never end a block; a line at or left of `checks:` does. */
function stillInsideBlock(line: string, checksIndent: number): boolean {
  return line.trim() === '' || indentOf(line) > checksIndent;
}

/** The indent of a `checks:` block opened by this line, or null. */
function opensChecksBlock(line: string): number | null {
  const block = /^(\s*)checks:\s*$/.exec(line);
  return block ? block[1].length : null;
}

/**
 * Every @concern: tag named inside a `checks:` block, sorted and deduplicated.
 */
export function parseDeclaredConcerns(yamlText: string): string[] {
  const found = new Set<string>();
  let checksIndent: number | null = null;

  for (const line of yamlText.split('\n')) {
    if (checksIndent !== null && stillInsideBlock(line, checksIndent)) {
      collectTags(line, found);
      continue;
    }

    checksIndent = opensChecksBlock(line);
    // Inline form (`checks: [ ... ]`) opens no block and is scanned in place.
    if (checksIndent === null && /^\s*checks:\s*\[/.test(line)) collectTags(line, found);
  }

  return [...found].sort((a, b) => a.localeCompare(b));
}

/**
 * Read the declared concern set from habits.yaml.
 *
 * `laneScope`, when given, intersects the declared set with the concerns a
 * lane is declared to measure. NO lane declares a scope in habits.yaml today,
 * so the denominator is the full declared set and every concern a run does not
 * exercise renders as notMeasured — which is the intended default: a lane that
 * has not declared a narrower scope is answerable for all of it.
 *
 * An unreadable habits.yaml does NOT return a quietly-empty denominator (that
 * would be the exact honest-absence failure this law exists to abolish): the
 * failure is carried in `source` AND in `unreadable`, so a reader counting
 * numbers rather than reading prose still cannot mistake it for clean.
 */
export function readDeclaredConcerns(
  habitsPath: string,
  laneScope?: readonly string[],
  readText: (path: string) => string = path => readFileSync(path, 'utf8')
): DeclaredConcernSet {
  const source = 'habits.yaml:checks + @concern registry';
  let text: string;
  try {
    text = readText(habitsPath);
  } catch (error) {
    const why = `UNREADABLE ${habitsPath}: ${String(error)}`;
    return { source: `${source} (${why})`, unreadable: why, concerns: [] };
  }

  const concerns = parseDeclaredConcerns(text);
  if (!laneScope) return { source, concerns };

  const scope = new Set(laneScope);
  return {
    source: `${source} ∩ lane scope`,
    concerns: concerns.filter(concern => scope.has(concern)),
  };
}
