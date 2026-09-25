/**
 * habit-delta — S3c of the sprint plan
 * (/projects/.claude-config/plans/we-ran-into-a-cryptic-robin.md): resolve exactly one habit
 * atom by its declared `@concern:<tag>` and append one dated DELTA line to it, newest-first,
 * directly after the closing frontmatter `---`.
 *
 * A habit is DECLARED where its concern lives, `<dir>/.epr-meta/<id>.habit.md` (CLAUDE.md, "The
 * habit register"); this module is the one place that WRITES a delta line into that atom, so a
 * peer-stage collection (S3a) and any other caller share one grammar rather than each hand-
 * rolling string surgery against the covenant's own ledger.
 *
 * Node builtins only — this file is imported both by the CLI wrapper (genesis/a2o/scripts/
 * habit-delta.ts) and by the peer-stage collector (genesis/agentic/compute/stage/
 * collect-stage.mjs), a plain .mjs that imports a .ts file directly under Node's native
 * type-stripping (the same pattern genesis/orchestrator/scripts/serving-receipt.mjs already
 * uses for lib/sut.ts and lib/household-attestation.ts) — no relative module specifier beyond
 * node builtins keeps that import free of a build step or a second toolchain.
 */

import { readdirSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

/** Directory names never walked, wherever they occur in the tree. */
const SKIP_DIRS = new Set(['node_modules', 'target', '.worktrees', '.git']);

/** `habits-status.py:273`'s own grammar — the line a written delta must satisfy. */
export const DELTA_DATE_RE = /^\s*DELTA (20\d{2}-\d{2}-\d{2})/m;

function safeReaddir(dir: string): { name: string; isDirectory: () => boolean }[] {
  try {
    return readdirSync(dir, { withFileTypes: true });
  } catch {
    return [];
  }
}

/** Every `<dir>/.epr-meta/*.habit.md` file under `root`, skipping the heavy/irrelevant trees. */
export function walkHabitFiles(root: string): string[] {
  const out: string[] = [];
  const stack: string[] = [root];
  while (stack.length > 0) {
    const dir = stack.pop() as string;
    for (const entry of safeReaddir(dir)) {
      if (SKIP_DIRS.has(entry.name)) continue;
      const full = join(dir, entry.name);
      if (!entry.isDirectory()) continue;
      if (entry.name === '.epr-meta') {
        for (const child of safeReaddir(full)) {
          if (!child.isDirectory() && child.name.endsWith('.habit.md')) {
            out.push(join(full, child.name));
          }
        }
        continue;
      }
      stack.push(full);
    }
  }
  return out.sort();
}

/** The frontmatter block's raw text (between the opening and closing `---` fences), or null. */
export function frontmatterOf(text: string): string | null {
  if (!text.startsWith('---')) return null;
  const end = text.indexOf('\n---', 3);
  if (end < 0) return null;
  return text.slice(0, end);
}

/** Just the `checks:` list's own lines from a frontmatter block — never `refs:` or prose. */
export function checksTextOf(frontmatter: string): string {
  const lines = frontmatter.split('\n');
  const start = lines.findIndex(l => /^checks:\s*$/.test(l));
  if (start < 0) return '';
  const out: string[] = [];
  for (let i = start + 1; i < lines.length; i++) {
    const line = lines[i];
    if (/^\s+-\s/.test(line)) out.push(line);
    else if (line.trim() === '') continue;
    else break; // the next top-level frontmatter key
  }
  return out.join('\n');
}

export type FindResult =
  | { ok: true; path: string }
  | { ok: false; reason: string; matches: string[] };

/**
 * Exactly one `.habit.md` whose `checks:` names `@concern:<concern>`, else a refusal naming
 * zero or the full ambiguous set — never a guess.
 */
export function findHabitByConcern(repoRoot: string, concern: string): FindResult {
  const tag = `@concern:${concern}`;
  const matches: string[] = [];
  for (const file of walkHabitFiles(repoRoot)) {
    let text: string;
    try {
      text = readFileSync(file, 'utf8');
    } catch {
      continue;
    }
    const front = frontmatterOf(text);
    if (front && checksTextOf(front).includes(tag)) matches.push(file);
  }
  if (matches.length === 0) {
    return { ok: false, reason: `no habit atom's checks: names ${tag}`, matches };
  }
  if (matches.length > 1) {
    return {
      ok: false,
      reason: `ambiguous: ${matches.length} habit atoms name ${tag} (${matches.join(', ')})`,
      matches,
    };
  }
  return { ok: true, path: matches[0] };
}

export interface DeltaInput {
  /** YYYY-MM-DD. */
  date: string;
  /** The parenthetical after the date, e.g. "peer-stage rung H, provider ab12cd34". */
  label?: string;
  /** The delta's own sentence(s), after "DELTA <date> (<label>): ". */
  text: string;
  /**
   * An idempotency guard: a substring unique to this exact delta (a completion action hash is
   * the natural choice). When it already appears anywhere in the file, nothing is written and
   * `appended` is false — this call already happened, or something recorded the same fact.
   * The guard only works when the caller's own `label`/`text` actually CONTAINS `onceKey` (the
   * natural case: a completion hash named in the delta's own sentence) — appendDelta never
   * writes `onceKey` anywhere the caller did not already put it.
   */
  onceKey: string;
}

export interface AppendResult {
  ok: boolean;
  appended: boolean;
  reason?: string;
}

/**
 * Inserts one `DELTA <date> (<label>): <text>` line directly after the closing frontmatter
 * `---`, newest-first (ahead of whatever was already there). Refuses rather than writing a line
 * that would not satisfy `_DELTA_DATE_RE`'s grammar — a delta habits-status.py's `last_evidence`
 * cannot parse is silently invisible, which is worse than refusing.
 */
export function appendDelta(path: string, input: DeltaInput): AppendResult {
  let content: string;
  try {
    content = readFileSync(path, 'utf8');
  } catch (e) {
    return { ok: false, appended: false, reason: `cannot read ${path}: ${(e as Error).message}` };
  }
  if (input.onceKey && content.includes(input.onceKey)) {
    return { ok: true, appended: false, reason: 'already recorded (onceKey present)' };
  }
  if (!content.startsWith('---')) {
    return { ok: false, appended: false, reason: 'no frontmatter to anchor the delta after' };
  }
  const end = content.indexOf('\n---', 3);
  if (end < 0) {
    return { ok: false, appended: false, reason: 'frontmatter has no closing --- fence' };
  }
  const fenceEnd = end + 4; // past the closing "---"
  const before = content.slice(0, fenceEnd);
  const after = content.slice(fenceEnd).replace(/^\n+/, '');
  const line = `DELTA ${input.date}${input.label ? ` (${input.label})` : ''}: ${input.text}`;
  if (!DELTA_DATE_RE.test(line)) {
    return {
      ok: false,
      appended: false,
      reason: 'delta line does not match the habits-status.py _DELTA_DATE grammar',
    };
  }
  const next = `${before}\n${line}\n\n${after}`;
  try {
    writeFileSync(path, next);
  } catch (e) {
    return { ok: false, appended: false, reason: `cannot write ${path}: ${(e as Error).message}` };
  }
  return { ok: true, appended: true };
}
