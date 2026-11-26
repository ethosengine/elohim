// Fence-scope is a reading beside the existing sibling-governance fence. It never supplies a
// candidate, changes a decision, or changes a closure key. Its classification and the judge's
// transitive JS import closure are read from the destination tip, so candidate bytes cannot bless
// themselves. The first landing therefore holds: its destination has no table yet.

import { posix } from 'node:path';
import { deviceEprResolution } from './device-epr.mjs';

export const FENCE_SCOPE_TABLE = 'genesis/orchestrator/gate-fence-scope.json';
export const FENCE_SCOPE_DESTINATION = 'origin/dev';

/** `GATE_FENCE_SCOPE=shadow|off`, default shadow. There is deliberately no `on`. */
export function fenceScopeMode(env = process.env) {
  return env?.GATE_FENCE_SCOPE === 'off' ? 'off' : 'shadow';
}

const strings = (value, name) => {
  if (!Array.isArray(value) || value.some(item => typeof item !== 'string' || !item)) {
    throw new Error(`fence-scope table ${name} must be non-empty strings`);
  }
  return [...new Set(value)];
};

export function parseFenceScopeTable(text) {
  let value;
  try { value = JSON.parse(text); } catch { throw new Error('fence-scope table is not JSON'); }
  if (value?.version !== 1) throw new Error('fence-scope table version is not 1');
  const table = {
    version: 1,
    judge: {
      importRoots: strings(value?.judge?.importRoots, 'judge.importRoots'),
      exact: strings(value?.judge?.exact, 'judge.exact'),
    },
    keyed: { exact: strings(value?.keyed?.exact, 'keyed.exact') },
    substrate: {
      prefixes: strings(value?.substrate?.prefixes, 'substrate.prefixes'),
      exact: strings(value?.substrate?.exact, 'substrate.exact'),
    },
  };
  if (!table.judge.exact.includes(FENCE_SCOPE_TABLE)) {
    throw new Error('fence-scope table does not classify itself as J');
  }
  return table;
}

function relativeImports(source) {
  const found = new Set();
  const staticImport = /\b(?:import|export)\s+(?:[^'";]*?\sfrom\s*)?['"](\.[^'"]+)['"]/g;
  const dynamicImport = /\bimport\s*\(\s*['"](\.[^'"]+)['"]\s*\)/g;
  for (const pattern of [staticImport, dynamicImport]) {
    for (const match of String(source).matchAll(pattern)) found.add(match[1]);
  }
  return [...found];
}

function importPath(from, specifier) {
  let path = posix.normalize(posix.join(posix.dirname(from), specifier));
  if (!posix.extname(path)) path += '.mjs';
  return path.startsWith('../') || path.startsWith('/') ? null : path;
}

function gitShow(root, ref, path, spawn) {
  const result = spawn('git', ['show', `${ref}:${path}`], { cwd: root, encoding: 'utf8' });
  if (result?.error?.code === 'EBUDGET') throw result.error;
  return result?.status === 0 ? String(result.stdout ?? '') : null;
}

const destinationCache = new Map();

/** The destination's table and import closure. Missing table is a distinct, fail-closed state. */
export function readDestinationFenceScope({
  root, ref = FENCE_SCOPE_DESTINATION, spawn,
}) {
  const cacheKey = `${root}\0${ref}`;
  if (destinationCache.has(cacheKey)) return destinationCache.get(cacheKey);
  const text = gitShow(root, ref, FENCE_SCOPE_TABLE, spawn);
  if (text === null) {
    const absent = { state: 'absent', ref, table: null, imports: new Set() };
    destinationCache.set(cacheKey, absent);
    return absent;
  }
  const table = parseFenceScopeTable(text);
  const imports = new Set();
  const pending = [...table.judge.importRoots];
  while (pending.length) {
    const path = pending.shift();
    if (imports.has(path)) continue;
    imports.add(path);
    const source = gitShow(root, ref, path, spawn);
    if (source === null) throw new Error(`destination import ${path} is unreadable`);
    for (const specifier of relativeImports(source)) {
      const imported = importPath(path, specifier);
      if (imported && !imports.has(imported)) pending.push(imported);
    }
  }
  const read = { state: 'read', ref, table, imports };
  destinationCache.set(cacheKey, read);
  return read;
}

/** Tests only: destination objects are immutable, while fixtures reuse process state. */
export function resetFenceScopeCache() {
  destinationCache.clear();
}

const unknownJudge = path =>
  /^genesis\/orchestrator\/gate-[^/]*\.mjs$/.test(path)
  || /^genesis\/orchestrator\/suite-[^/]*$/.test(path);

/** J wins over K and S: an R1 file remains judge material even inside the substrate tree. */
export function classifyFencePath(path, { table, imports = new Set() }) {
  if (imports.has(path) || table.judge.exact.includes(path) || unknownJudge(path)) return 'J';
  if (table.keyed.exact.includes(path)) return 'K';
  if (table.substrate.exact.includes(path)
      || table.substrate.prefixes.some(prefix => path.startsWith(prefix))) return 'S';
  return null;
}

const matchingGreen = (candidate, project, key) => candidate?.source === 'sibling'
  && candidate.success === true && candidate.project === project && candidate.closure === key;

/** Pure classification of one observed sibling read and one push range. */
export function evaluateFenceScope({
  project, key, paths = [], candidates = [], destination, readerAdmitted = false,
}) {
  if (candidates.length === 0) return null;
  if (!candidates.some(candidate => matchingGreen(candidate, project, key))) {
    return { answer: 'no-sibling-green', paths: [], reason: null };
  }
  if (destination.state === 'absent') {
    return { answer: 'would-refuse', paths: [FENCE_SCOPE_TABLE], reason: 'table-absent' };
  }
  const classified = paths.map(path => ({ path, cls: classifyFencePath(path, destination) }))
    .filter(row => row.cls);
  const judges = classified.filter(row => row.cls === 'J');
  if (judges.length) {
    return { answer: 'would-refuse', paths: judges.map(row => row.path), reason: 'judge' };
  }
  const substrate = classified.filter(row => row.cls === 'S');
  if (substrate.length && !readerAdmitted) {
    return {
      answer: 'would-refuse', paths: substrate.map(row => row.path),
      reason: 'substrate-reader-not-admitted',
    };
  }
  if (classified.length === 0) return { answer: 'not-applicable', paths: [], reason: null };
  return { answer: 'would-admit', paths: classified.map(row => row.path), reason: 'key-unchanged' };
}

export function fenceScopeLine(project, reading) {
  const prefix = `[gate] ${project} fence-scope (shadow): `;
  if (reading.reason === 'table-absent') return `${prefix}table absent at destination; the fence holds`;
  if (reading.reason === 'range-unavailable') return `${prefix}would refuse: governance range unavailable; the fence holds`;
  if (reading.answer === 'no-sibling-green') return `${prefix}no sibling green to judge`;
  if (reading.answer === 'not-applicable') return `${prefix}not applicable (no governance path changed)`;
  if (reading.answer === 'would-refuse') {
    const why = reading.reason === 'substrate-reader-not-admitted'
      ? ' (the verdict reader is not an admitted store release)'
      : '';
    return `${prefix}would refuse: J path ${reading.paths[0]}${why}`;
  }
  return `${prefix}would admit a sibling green (the push changes only S/K paths, first: ${reading.paths[0]}; key unchanged)`;
}

/** The existing native gate-reading ledger carries the answer and exact deciding paths. */
export function fenceScopeStep(project, reading) {
  const paths = reading.paths.length
    ? reading.paths.map(path => encodeURIComponent(path)).join(',')
    : 'none';
  return `${project}:fence-scope:${reading.answer}:${paths}`;
}

/**
 * Read and classify one shadow. Off and an unobserved sibling channel perform no read or spawn.
 * `range` is supplied by gate-runner's bounded, cached push-range reader.
 */
export function fenceScopeShadow({
  project, key, range, candidates, root, env = process.env, spawn,
  resolveReader = deviceEprResolution,
}) {
  if (fenceScopeMode(env) === 'off' || !Array.isArray(candidates) || candidates.length === 0) return null;
  const destination = readDestinationFenceScope({ root, spawn });
  if (destination.state === 'read' && !range?.base) {
    return {
      answer: 'would-refuse', paths: ['<governance-range-unavailable>'],
      reason: 'range-unavailable',
    };
  }
  let readerAdmitted = false;
  if (destination.state === 'read') {
    const hasSubstrate = (range?.files || []).some(path =>
      classifyFencePath(path, destination) === 'S');
    if (hasSubstrate) readerAdmitted = resolveReader({ env, warn: () => {} }).source === 'head';
  }
  return evaluateFenceScope({
    project, key, paths: range?.files || [], candidates, destination, readerAdmitted,
  });
}
