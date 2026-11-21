// doc-cut: a seconds check that refuses a push to dev or main which leaves a long document much
// shorter than the destination holds it, unless a pushed commit that touches that file carries
// `[doc:cut]`. On 2026-11-21 five consecutive landings each replaced two long documents with only
// the section the author meant to append (a script emptied the file before reading it):
// genesis/data/timeline/backlog/steward-device-arrival-open-items.md went from 1,531 lines to 39,
// the sprint journal from 553 to 38. No commit message said so, every gate was green, and another
// device found it hours later (backlog row "Row 68"). Habit `careless-reds-cost-seconds`.
//
// THE RULE. For each pushed ref, the destination's current tip (the base) is compared with the
// pushed tip: what the destination would hold afterwards, not each commit (a cut that a later
// commit in the same push restores is clear). A file present at both ends trips it when its line
// count at the pushed tip is under half its count at the base AND at least 100 lines fewer. Scope:
// `.md`, `.feature`, `.yaml`, `.yml`, except generated files (genesis/manifests/habits.yaml and
// any path the pushed tip's root .gitattributes marks linguist-generated). A deleted file does not
// trip it. A renamed file is compared with its old path where git pairs them at 20% similarity;
// a rename git cannot pair reads as a delete plus an add, which is not seen.
//
// CLEARED BY. A pushed commit that touches that file (old or new path) carries `[doc:cut]`. A
// reason beside it is asked for and not checked. A clear check adds no row; a declared cut adds one
// `ok` row naming the commit, as fleet-roll does for `[edge:roll]`.
//
// VERDICT. A push naming refs/heads/dev or refs/heads/main: REFUSED. Any other push, and a run
// with no push (compares origin/dev with HEAD when both resolve, else says it did not run): WARN.
// A check that cannot run is REFUSED on a dev or main push and WARN otherwise, saying what failed.
//
// COST. One `git diff --numstat` of the two ends, then line counts only for files with more than
// 100 removed lines (removed >= before - after, so no other file can trip).

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

export const CHECK = 'doc-cut';
export const CUT_TAG = '[doc:cut]';
export const MIN_REMOVED = 100;
export const DEPLOYING_BRANCHES = Object.freeze(['dev', 'main']);
export const GENERATED = Object.freeze(['genesis/manifests/habits.yaml']);
const SCOPE = /\.(md|feature|ya?ml)$/;
const BUFFER = 64 * 1024 * 1024;
const short = sha => String(sha ?? '').slice(0, 12);
const isZero = sha => /^0+$/.test(String(sha ?? ''));
const branchOf = ref => (String(ref).startsWith('refs/heads/') ? String(ref).slice('refs/heads/'.length) : null);
const isDeploying = branch => DEPLOYING_BRANCHES.includes(branch);
const words = text => String(text ?? '').split(/\s+/).filter(Boolean);
const row = (check, level, why, cure = '') => ({ check, level, why, cure });
const fmt = n => n.toLocaleString('en-US');

function git(spawn, root, args) {
  const r = spawn('git', ['-C', root, ...args], { encoding: 'utf8', maxBuffer: BUFFER });
  if (r.error || r.status !== 0) {
    const what = r.error?.code === 'ENOBUFS' ? 'output larger than 64 MiB' : String(r.stderr || r.error?.message || '').trim().split('\n')[0];
    throw new Error(`git ${args.slice(0, 2).join(' ')} exited ${r.status ?? r.error?.code}: ${what}`);
  }
  return String(r.stdout);
}
const resolves = (spawn, root, rev) => spawn('git', ['-C', root, 'rev-parse', '--verify', '--quiet', `${rev}^{commit}`], { encoding: 'utf8' }).status === 0;

/** Lines in a blob at `rev:path`: newline count, plus one for an unterminated last line. */
export function countLines(spawn, root, rev, path) {
  const text = git(spawn, root, ['show', `${rev}:${path}`]);
  if (!text) return 0;
  let n = 0;
  for (let i = text.indexOf('\n'); i >= 0; i = text.indexOf('\n', i + 1)) n++;
  return text.endsWith('\n') ? n : n + 1;
}

// ── Generated files ─────────────────────────────────────────────────────────

function globToRegExp(pattern) {
  let out = '';
  for (let i = 0; i < pattern.length; i++) {
    const c = pattern[i];
    if (c === '*') {
      if (pattern[i + 1] === '*') { out += pattern[i + 2] === '/' ? '(?:.*/)?' : '.*'; i += pattern[i + 2] === '/' ? 2 : 1; } else out += '[^/]*';
    } else if (c === '?') out += '[^/]';
    else out += c.replace(/[.+^${}()|[\]\\]/g, '\\$&');
  }
  return new RegExp(`^${out}$`);
}

/** Matcher for paths a `.gitattributes` text marks linguist-generated (last matching line wins). */
export function generatedMatcher(attributes) {
  const rules = [];
  for (const raw of String(attributes ?? '').split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const [pattern, ...attrs] = line.split(/\s+/);
    let value = null;
    for (const a of attrs) {
      if (a === 'linguist-generated' || a === 'linguist-generated=true') value = true;
      else if (a === '-linguist-generated' || a === '!linguist-generated' || a === 'linguist-generated=false') value = false;
    }
    if (value === null) continue;
    const anchored = pattern.includes('/');
    const re = globToRegExp(pattern.replace(/^\//, '').replace(/\/$/, '/**'));
    rules.push({ re, anchored, value });
  }
  return path => {
    let verdict = false;
    for (const { re, anchored, value } of rules) {
      if (re.test(anchored ? path : path.split('/').pop()) || (!anchored && re.test(path))) verdict = value;
    }
    return verdict;
  };
}

// ── Pure judgement ──────────────────────────────────────────────────────────

/** Paths a numstat -z listing names: [{ removed, path, oldPath }], binary and unscoped files dropped. */
export function parseNumstat(text) {
  const parts = String(text).split('\0');
  const out = [];
  for (let i = 0; i < parts.length; i++) {
    const m = /^(\d+|-)\t(\d+|-)\t(.*)$/.exec(parts[i]);
    if (!m) continue;
    let path = m[3];
    let oldPath = m[3];
    if (path === '') { oldPath = parts[i + 1]; path = parts[i + 2]; i += 2; }
    if (m[2] === '-') continue;
    out.push({ removed: Number(m[2]), path, oldPath });
  }
  return out;
}

/** Whether `before` -> `after` lines is a cut: under half, and at least MIN_REMOVED fewer. */
export const isCut = (before, after) => after * 2 < before && before - after >= MIN_REMOVED;

export function cutRow({ target, path, oldPath, before, after, base, declaredBy, deploying }) {
  const was = oldPath !== path ? ` (was ${oldPath})` : '';
  const what = `${path}${was} falls from ${fmt(before)} to ${fmt(after)} lines`;
  const where = `push to ${target}`;
  if (declaredBy) return row(CHECK, 'ok', `${where}: ${what} — declared by ${CUT_TAG} in ${short(declaredBy)}`);
  return row(CHECK, deploying ? 'REFUSED' : 'WARN',
    `${where}: ${what}, and no pushed commit that touches it carries ${CUT_TAG}; the destination would hold ${Math.round((1 - after / before) * 100)}% less of it`,
    `if the cut is meant, put ${CUT_TAG} and why in the message of the commit that makes it; if not, the file's last whole version is git show ${short(base)}:${oldPath}`);
}

/** A failure to decide: REFUSED on a dev or main push, WARN otherwise; never silent. */
export function failureRows({ error, deploying, target = null }) {
  const first = String(error?.message || error).split('\n')[0];
  return [row(CHECK, deploying ? 'REFUSED' : 'WARN',
    `${target ? `push to ${target}: ` : ''}the doc-cut check could not decide: ${first}`,
    'fix what the error names (a missing object usually wants git fetch) and push again')];
}

// ── Git objects ─────────────────────────────────────────────────────────────

/** Rows for one base..tip comparison. `deploying` decides REFUSED or WARN. */
export function judgeRange({ root, base, tip, target, deploying, spawn = spawnSync }) {
  try {
    const stat = parseNumstat(git(spawn, root, ['diff', '--numstat', '-z', '-M20%', '--diff-filter=d', '--ignore-submodules=none', '--no-relative', '--no-ext-diff', '--no-color', base, tip]));
    const candidates = stat.filter(f => f.removed > MIN_REMOVED - 1 && SCOPE.test(f.path) && !GENERATED.includes(f.path));
    if (!candidates.length) return [];
    let attrs = '';
    if (spawn('git', ['-C', root, 'cat-file', '-e', `${tip}:.gitattributes`], { encoding: 'utf8' }).status === 0) attrs = git(spawn, root, ['show', `${tip}:.gitattributes`]);
    const generated = generatedMatcher(attrs);
    const rows = [];
    for (const f of candidates) {
      if (generated(f.path)) continue;
      const before = countLines(spawn, root, base, f.oldPath);
      const after = countLines(spawn, root, tip, f.path);
      if (!isCut(before, after)) continue;
      const paths = [...new Set([f.path, f.oldPath])];
      const declaredBy = git(spawn, root, ['log', '--no-show-signature', '--full-history', '-i', '--fixed-strings', `--grep=${CUT_TAG}`, '-1', '--format=%H', `${base}..${tip}`, '--', ...paths]).trim() || null;
      rows.push(cutRow({ target, path: f.path, oldPath: f.oldPath, before, after, base, declaredBy, deploying }));
    }
    return rows;
  } catch (error) {
    return failureRows({ error, deploying, target });
  }
}

/** `GATE_PUSH_REFS`: one `<remote ref> <pushed sha> <destination sha>` per line (pre-push.bash). */
export function parseRefs(text) {
  return String(text ?? '').split('\n').map(line => line.trim()).filter(Boolean).map(line => {
    const [ref, local, remote] = line.split(/\s+/);
    return { ref, local, remote, branch: branchOf(ref) };
  });
}

function judgeRef({ ref, root, spawn, remoteName }) {
  const deploying = isDeploying(ref.branch);
  const target = ref.branch ?? ref.ref;
  try {
    let base = ref.remote;
    if (isZero(ref.remote) || !resolves(spawn, root, ref.remote)) {
      if (deploying) throw new Error(isZero(ref.remote) ? `this push creates ${target}, so there is no destination tip to compare with` : `the destination tip ${short(ref.remote)} is not in this clone (git fetch ${remoteName} ${target})`);
      base = git(spawn, root, ['merge-base', `${remoteName}/dev`, ref.local]).trim();
    }
    return judgeRange({ root, base, tip: ref.local, target, deploying, spawn });
  } catch (error) {
    return failureRows({ error, deploying, target });
  }
}

/** The early hook block's mark, as fleet-roll's: the run writes the ref list it judged to this file. */
export function docCutMarkHonoured(env = process.env) {
  if (!env.GATE_DOC_CUT_MARK || !env.GATE_PUSH_REFS) return false;
  try { return readFileSync(env.GATE_DOC_CUT_MARK, 'utf8') === env.GATE_PUSH_REFS; } catch { return false; }
}

/**
 * The preflight entry. Under pre-push (GATE_PUSH_REFS): every pushed ref against its destination
 * tip. Outside a push: origin/dev against HEAD, WARN only; a WARN saying so when either is absent.
 */
export function checkDocCut({ env = process.env, root = process.cwd(), spawn = spawnSync } = {}) {
  const remoteName = env.GATE_PUSH_REMOTE || 'origin';
  if (env.GATE_PUSH_REFS === undefined && !words(env.GATE_PUSH_TARGETS).length) {
    if (!resolves(spawn, root, `${remoteName}/dev`) || !resolves(spawn, root, 'HEAD')) {
      return [row(CHECK, 'WARN', `the doc-cut check did not run: ${remoteName}/dev and HEAD do not both resolve in this clone`, `git fetch ${remoteName} dev, then run it again`)];
    }
    return judgeRange({ root, base: `${remoteName}/dev`, tip: 'HEAD', target: 'dev (not a push)', deploying: false, spawn });
  }
  if (env.GATE_PUSH_REFS === undefined) {
    // An older hook hands over no ref list: HEAD against the remote's tip of the named branch,
    // right only when HEAD is the pushed commit; not being able to read it is a WARN, not a red.
    const names = words(env.GATE_PUSH_TARGETS).map(branchOf);
    const branch = names.find(isDeploying) ?? names[0] ?? 'dev';
    if (!resolves(spawn, root, `${remoteName}/${branch}`) || !resolves(spawn, root, 'HEAD')) {
      return [row(CHECK, 'WARN', `the doc-cut check did not run: the running pre-push hook hands over no ref list (GATE_PUSH_REFS), and ${remoteName}/${branch} and HEAD do not both resolve`, `git fetch ${remoteName} ${branch}, or bring the checkout's .husky/pre-push.bash up to date`)];
    }
    return judgeRange({ root, base: `${remoteName}/${branch}`, tip: 'HEAD', target: branch, deploying: isDeploying(branch), spawn });
  }
  return parseRefs(env.GATE_PUSH_REFS).flatMap(ref => judgeRef({ ref, root, spawn, remoteName }));
}

// By hand: GATE_PUSH_REFS='refs/heads/dev <tip> <base>' node doc-cut-guard.mjs [root]
if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  const rows = checkDocCut({ root: process.argv[2] || process.cwd() });
  for (const r of rows) process.stdout.write(`${r.level} ${r.check}: ${r.why}${r.cure ? `\n  cure: ${r.cure}` : ''}\n`);
  if (!rows.length) process.stdout.write('clear: no document cut\n');
}
