// package-quality.mjs — description-quality and trigger-overlap checks for
// capability packages, absorbed from the retired memory-kit audits
// (`.claude/scripts/memory-kit/{skill,agent}-audit.py`, deleted 2026-09-10).
//
// WHY THE CHECKS MOVED HERE. The audits scanned the PROJECTIONS
// (`.claude/skills/*/SKILL.md`, `.claude/agents/*.md`) and wrote a dated
// advisory report nobody gated on. Under package authority the description is
// package metadata, so the same quality bound belongs on the package and is
// evaluated by the same command that already proves projection fidelity.
//
// WHAT IS *NOT* PORTED: staleness-by-mtime (`surface-stale-mtime-days@1`,
// hard 90). Package verification is CONTENT-BASED: a package's identity is its
// bytes, and a projection's mtime is an artifact of the last `project` run, not
// evidence about the capability. An mtime bound evaluated here would fire on
// every fresh checkout and stay silent on a stale package that happened to be
// reprojected. If age is wanted it is a fold over authored history (git), which
// belongs to `epr flow report`, not to the projection verifier.
//
// The numbers are DECLARED, not invented: they are read from the middot
// registry `.claude/epr-meta/measures.yaml` — lens rows `skill-description-floor@1`
// (hard 60), `agent-description-floor@1` (hard 80) and `trigger-overlap-ceiling@1`
// (hard 3), each `consumes:` the measure it bounds (`package-description-chars@1`,
// `trigger-overlap@1`). The literals below are the fallback when the registry is
// unreadable; they are the same values those rows carry, and the row id is the
// authority.

import { readFile } from 'node:fs/promises';

/** Fallback bounds — identical to the registry rows named in each key's comment. */
export const FALLBACK_BOUNDS = {
  // lens `skill-description-floor@1` · hard 60 · consumes package-description-chars@1
  skillDescriptionFloor: 60,
  // lens `agent-description-floor@1` · hard 80 · consumes package-description-chars@1
  agentDescriptionFloor: 80,
  // lens `trigger-overlap-ceiling@1` · hard 3 · consumes trigger-overlap@1
  triggerOverlapThreshold: 3,
  // lens `dynamic-stopword-fraction-ceiling@1` · hard 0.25 · consumes dynamic-stopword-fraction@1
  dynamicStopwordFraction: 0.25,
};

const BOUND_ROWS = {
  skillDescriptionFloor: 'skill-description-floor',
  agentDescriptionFloor: 'agent-description-floor',
  triggerOverlapThreshold: 'trigger-overlap-ceiling',
  dynamicStopwordFraction: 'dynamic-stopword-fraction-ceiling',
};

/**
 * Read the four bounds out of the middot registry. Deliberately a narrow line
 * scanner rather than a YAML dependency: it reads exactly `- id: <row>` … `hard: <n>`
 * within one row block and nothing else, so it cannot silently pick up an
 * unrelated key. Any row it cannot find falls back to the declared literal above
 * and is named in `.source` so the caller can print which bounds were live.
 */
export async function loadQualityBounds(measuresPath) {
  const bounds = { ...FALLBACK_BOUNDS };
  const source = {};
  for (const key of Object.keys(BOUND_ROWS)) source[key] = 'fallback';
  let raw;
  try {
    raw = await readFile(measuresPath, 'utf8');
  } catch {
    return { ...bounds, source, registry: null };
  }
  const wanted = new Map(Object.entries(BOUND_ROWS).map(([key, row]) => [row, key]));
  const lines = raw.split('\n');
  let currentKey = null;
  for (const line of lines) {
    const idMatch = /^\s*-\s+id:\s*(\S+)\s*$/.exec(line);
    if (idMatch) {
      currentKey = wanted.get(idMatch[1]) ?? null;
      continue;
    }
    if (!currentKey) continue;
    const hardMatch = /^\s+hard:\s*([0-9]*\.?[0-9]+)\s*$/.exec(line);
    if (hardMatch) {
      bounds[currentKey] = Number(hardMatch[1]);
      source[currentKey] = `${BOUND_ROWS[currentKey]}@1`;
      currentKey = null;
      continue;
    }
    // A new top-level list item ends the row block without a `hard:` — keep the fallback.
    if (/^\s*-\s+\S/.test(line)) currentKey = null;
  }
  return { ...bounds, source, registry: measuresPath };
}

/** The description floor for a package kind, in characters. */
export function descriptionFloorFor(kind, bounds) {
  return kind === 'AgentPackage' ? bounds.agentDescriptionFloor : bounds.skillDescriptionFloor;
}

// ── A. Description quality (ported verbatim from the two audits) ────────────

const GENERIC_PATTERNS = [
  /\bfor working with\b/i,
  /\buse this (?:skill|agent)\b/i,
  /\bhelps? you\b/i,
  /\bgeneral(?:ly)? (?:purpose|useful)\b/i,
];
// agent-audit added `context` to the skill-audit signal set; the union is used
// here because one registry row now bounds both kinds.
const TRIGGER_SIGNALS = /\b(use|when|trigger|invoke|before|after|whenever|context)\b/i;
// agent-audit also accepted an <example> block as a concrete trigger.
const QUOTED_TRIGGER = /"[^"]{4,}"|<example>/;

/**
 * Classify one description. Returns the audit's `kinds` list; empty means clean.
 * `description-missing` and `description-too-short` are the HARD kinds (the
 * declared floor); `description-no-when` / `description-no-triggers` are advisory.
 */
export function diagnoseDescription(kind, description, bounds) {
  const desc = (description ?? '').trim();
  const kinds = [];
  if (!desc) {
    kinds.push('description-missing');
    return kinds;
  }
  if (desc.length < descriptionFloorFor(kind, bounds)) kinds.push('description-too-short');
  if (!TRIGGER_SIGNALS.test(desc)) kinds.push('description-no-when');
  if (!QUOTED_TRIGGER.test(desc) && GENERIC_PATTERNS.some((p) => p.test(desc))) {
    kinds.push('description-no-triggers');
  }
  return kinds;
}

export const HARD_DESCRIPTION_KINDS = new Set(['description-missing', 'description-too-short']);

/** The hard subset — what `verify` refuses on. */
export function hardDescriptionKinds(kinds) {
  return kinds.filter((k) => HARD_DESCRIPTION_KINDS.has(k));
}

// ── B. Trigger overlap (ported verbatim from the two audits) ────────────────

const STOPWORDS = new Set(
  `the a is of to and in for with that this it as on at by from or are be was were
   not can if then but which you your our their they we use when it's skill`
    .split(/\s+/)
    .filter(Boolean),
);
const WORD_RE = /[a-zA-Z][a-zA-Z\-_/]*/g;

/** Candidate distinctive tokens, pre corpus-stopword filter. */
export function rawKeywords(description) {
  const found = (description ?? '').match(WORD_RE) ?? [];
  return new Set(
    found.map((t) => t.toLowerCase()).filter((t) => t.length >= 3 && !STOPWORDS.has(t)),
  );
}

/** Words in >= `fraction` of the corpus are scaffolding, not distinctive triggers. */
export function buildDynamicStopwords(entries, fraction) {
  if (entries.length === 0) return new Set();
  const counts = new Map();
  for (const entry of entries) {
    for (const token of rawKeywords(entry.description)) {
      counts.set(token, (counts.get(token) ?? 0) + 1);
    }
  }
  const threshold = Math.max(2, Math.floor(entries.length * fraction));
  return new Set([...counts].filter(([, c]) => c >= threshold).map(([t]) => t));
}

/**
 * Pairs sharing MORE than `triggerOverlapThreshold` distinctive words, most
 * overlapping first. Compared within one kind, as both audits did.
 */
export function findOverlapPairs(entries, bounds) {
  const stops = buildDynamicStopwords(entries, bounds.dynamicStopwordFraction);
  const keywords = entries.map((e) => ({
    id: e.id,
    kw: new Set([...rawKeywords(e.description)].filter((t) => !stops.has(t))),
  }));
  const pairs = [];
  for (let i = 0; i < keywords.length; i += 1) {
    if (keywords[i].kw.size === 0) continue;
    for (let j = i + 1; j < keywords.length; j += 1) {
      if (keywords[j].kw.size === 0) continue;
      const shared = [...keywords[i].kw].filter((t) => keywords[j].kw.has(t)).sort();
      if (shared.length > bounds.triggerOverlapThreshold) {
        pairs.push({ a: keywords[i].id, b: keywords[j].id, shared });
      }
    }
  }
  pairs.sort((x, y) => y.shared.length - x.shared.length);
  return { pairs, dynamicStopwords: stops };
}
