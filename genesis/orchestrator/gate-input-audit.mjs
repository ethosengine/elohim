#!/usr/bin/env node
// Static hints for repository reads outside a gate's declared input closure.
// This is deliberately not a Rust compiler: unrecognized transformations and
// repository walks need explicit gate-reads declarations or remain unresolved.

import { spawnSync } from 'child_process';
import { readFileSync } from 'fs';
import { dirname, matchesGlob, posix, relative, resolve } from 'path';
import { fileURLToPath } from 'url';
import { loadGateRegistry } from './pipeline-registry.mjs';

/**
 * A repository-wide basename read: `**\/<name>` names every tracked file whose last path segment
 * is exactly `<name>` (for example every `.epr-meta` manifest). It is the only wildcard a
 * gate-reads declaration may carry; it lets a walk over one file kind leave tree scope.
 */
/**
 * Gate projects whose recipe runs tests this audit cannot read (it scans Rust only): their closure
 * keys on the whole commit tree, whatever Rust read set their cargo workspace has. E9 (review of
 * E7): orchestrator declares the eprfs workspace only to build its EPR_BIN, while its recipe runs a
 * node, python and shell suite whose reads span the repository (every build-manifest.json through the
 * registry tests, every .rs through the REAL read-set tests, .husky, scripts/ci, genesis/scripts/ci,
 * app/elohim-app/scripts/__tests__, ...); keyed on eprfs's Rust reads, a change to one of them could
 * reuse a green the suite never judged.
 * E11: the suite declares those reads itself. orchestrator runs only the files that name every
 * read, in a view holding only those bytes (suite-run.mjs), and keys on their union
 * (SUITE_READ_LISTS); the files whose reads cannot be named, the python suites and the Jenkins
 * lint run in orchestrator-tree, which keeps the whole-tree key.
 */
export const UNAUDITED_RECIPE_PROJECTS = Object.freeze(['orchestrator-tree']);

/** Gate projects whose suite declares its own reads, by the repo-relative path of the list. */
export const SUITE_READ_LISTS = Object.freeze({ orchestrator: 'genesis/orchestrator/suite-reads.json' });

/**
 * Reads every named-side run makes besides the ones the list declares: what shapes the
 * node_modules the view copies (the lock, the workspace, the root manifest's overrides, and
 * .npmrc's hoisting, which a bare import resolves through).
 */
export const SUITE_IMPLICIT_READS = Object.freeze(['.npmrc', 'package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml']);

/**
 * The read set of a suite list's named side, in readSet's shape: every declared read, every named
 * test file outside the list's own directory (inside it, the project tree keys it), and the
 * implicit reads. Null when the text is not a list, so the key falls back to the whole tree.
 * suite-run.mjs builds the named half's view from this same set, so the view is never wider.
 */
export function suiteReadSet(text, listPath) {
  let list;
  try { list = JSON.parse(text); } catch { return null; }
  if (!Array.isArray(list?.groups)) return null;
  const dir = posix.dirname(listPath);
  const paths = new Set(SUITE_IMPLICIT_READS);
  for (const group of list.groups) {
    for (const entry of group.files || []) {
      if (entry.tree) continue;
      if (!Array.isArray(entry.reads) || typeof entry.file !== 'string') return null;
      const file = posix.normalize(posix.join(dir, entry.file));
      if (!(file === dir || file.startsWith(`${dir}/`))) paths.add(file);
      for (const read of entry.reads) paths.add(read);
    }
  }
  return { paths: [...paths].sort(), unresolved: [] };
}

export function isReadPattern(path) {
  return /^\*\*\/[^/*\\]+$/.test(path);
}

/** Does a read (exact path, directory prefix, `.`, or `**\/<name>`) cover a repo-relative file? */
export function readCovers(read, file) {
  if (read === '.') return true;
  if (isReadPattern(read)) {
    const name = read.slice(3);
    return file === name || file.endsWith(`/${name}`);
  }
  return file === read || file.startsWith(`${read}/`);
}

/**
 * Deliberately simpler than full glob syntax: a terminal /** is a subtree; otherwise exact.
 * Manifest `sources` globs also drive CI dispatch, so a basename read is NOT a source glob here —
 * reads are matched by readCovers.
 */
export function globCovers(glob, path) {
  if (glob === '**') return true;
  if (!glob.endsWith('/**')) return glob === path;
  const prefix = glob.slice(0, -3);
  return path === prefix || path.startsWith(`${prefix}/`);
}

/** Reject exhaust in declared sources, recipe path references, or audited reads.
 * Include prospective names as well as today's files: a newly named ledger must
 * not evade the promise just because it hasn't been written yet.
 */
export function exhaustInputs({ exhaust, files = [], sources = [], reads = [], recipe = '' }) {
  const references = [...recipe.replace(/^\s*#.*$/gm, '').matchAll(/\.claude\/data(?:\/[\w.*?{}[\]/-]+)?/g)]
    .map(match => match[0]);
  const inputs = [...sources, ...reads, ...references];
  const candidates = [...files, ...exhaust, ...inputs].map(path => path.replace(/\*\*/g, 'probe').replace(/[?*]/g, 'probe'));
  // A basename read can name a future ledger without its directory prefix.
  candidates.push(...reads.filter(isReadPattern).map(path => `.claude/data/${path.slice(3)}`));
  const excluded = candidates.filter(path => exhaust.some(pattern => matchesGlob(path, pattern)));
  return [...new Set(inputs.filter(input => excluded.some(path =>
    globCovers(input, path) || matchesGlob(path, input) || readCovers(input.replace(/\/\*\*$/, ''), path))))];
}

export function repositoryFiles(root) {
  const result = spawnSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard', '--', '*.rs'], {
    cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
  });
  if (result.status !== 0) throw new Error(result.error?.message || result.stderr || 'git ls-files failed');
  return [...new Set(result.stdout.split('\0').filter(Boolean))];
}

/**
 * `// gate-reads: <path>` names a read for every gate whose sources cover the file.
 * `// gate-reads(<project>): <path>` names it for that gate project alone: the statement it sits
 * above runs only in that project's recipe (eprfs's live-corpus legs run in `eprfs-live-corpus`,
 * which the workspace recipe skips by name), so another project scanning the same file neither
 * keys on the read nor reports it unresolved. A scan that names no project counts every
 * declaration, scoped or not (reverse-dependency narrowing must see every read).
 */
const DECLARATION = /^\/\/\s*gate-reads(?:\(([A-Za-z0-9_.-]+)\))?:\s*(\S.*?)\s*$/;

/**
 * Every `gate-reads` declaration a Rust source carries, scoped or not, as the audit reads it.
 * The grammar is pinned for this parser and for epr-cli's `assert_declared_reads` by one fixture,
 * elohim/eprfs/epr-cli/tests/fixtures/gate-reads-grammar.tsv.
 */
export function declarationsIn(text) {
  return tokens(text).declarations.map(({ project, path, line }) => ({ project, path, line }));
}

/** Every `gate-reads(<project>)` scope a Rust source names, in order of appearance. */
export function declarationScopes(text) {
  return declarationsIn(text).filter(d => d.project);
}

// Keep strings atomic and discard comments, preserving locations. In particular,
// neither examples in doc comments nor Rust source inside fixture strings are reads.
function tokens(text) {
  const out = [];
  const declarations = [];
  const pattern = /\s+|\/\/[^\n]*|(?:br|r)(#*)"[\s\S]*?"\1|b?"(?:\\[\s\S]|[^"\\])*"|b?'(?:\\(?:u\{[\da-fA-F]+\}|x[\da-fA-F]{2}|.)|[^'\\\n])'|[A-Za-z_][A-Za-z_0-9]*|\d+|::|./gy;
  let offset = 0;
  let line = 1;
  const stack = [];
  while (offset < text.length) {
    let raw;
    if (text.startsWith('/*', offset)) {
      let end = offset + 2;
      let depth = 1;
      while (end < text.length && depth) {
        if (text.startsWith('/*', end)) { depth++; end += 2; }
        else if (text.startsWith('*/', end)) { depth--; end += 2; }
        else end++;
      }
      raw = text.slice(offset, end);
    } else {
      pattern.lastIndex = offset;
      raw = pattern.exec(text)[0];
    }
    const declaration = raw.match(DECLARATION);
    if (declaration && /^\s*$/.test(text.slice(text.lastIndexOf('\n', offset - 1) + 1, offset))) {
      declarations.push({ path: declaration[2], project: declaration[1] || null, offset, line });
    }
    if (!/^\s|^\/\//.test(raw) && !raw.startsWith('/*')) {
      const token = { text: raw, offset, line };
      const rawString = raw.match(/^(?:br|r)(#*)"([\s\S]*)"\1$/);
      if (rawString) token.value = rawString[2];
      else if (/^b?"/.test(raw)) {
        try { token.value = JSON.parse(raw.replace(/^b/, '')); } catch { /* unsupported Rust escape */ }
      }
      const index = out.length;
      out.push(token);
      if (['(', '[', '{'].includes(raw)) stack.push(index);
      else if ([')', ']', '}'].includes(raw)) {
        const open = stack.pop();
        if (open !== undefined) out[open].close = index;
      }
    }
    line += raw.split('\n').length - 1;
    offset += raw.length;
  }
  return { ts: out, declarations };
}

function functionsIn(ts) {
  const functions = [];
  for (let i = 0; i < ts.length; i++) {
    if (ts[i].text !== 'fn') continue;
    let body = i + 2;
    while (body < ts.length && !['{', ';'].includes(ts[body].text)) {
      body = (ts[body].close ?? body) + 1;
    }
    if (ts[body]?.text === '{' && ts[body].close !== undefined) {
      functions.push({ name: ts[i + 1].text, start: i, body, end: ts[body].close });
    }
  }
  return functions;
}

function callAt(ts, start) {
  let end = start;
  if (!/^[A-Za-z_]\w*$/.test(ts[end]?.text || '')) return null;
  while (ts[end + 1]?.text === '::' && /^[A-Za-z_]\w*$/.test(ts[end + 2]?.text || '')) end += 2;
  if (ts[end + 1]?.text === '!') end++;
  const open = end + 1;
  if (ts[open]?.text !== '(' || ts[open].close === undefined) return null;
  return { name: ts.slice(start, open).map(t => t.text).join(''), open, end: ts[open].close + 1 };
}

function argumentsOf(ts, call) {
  const args = [];
  let start = call.open + 1;
  for (let i = start; i < call.end - 1; i++) {
    if (ts[i].text === ',') { args.push([start, i]); start = i + 1; }
    else if (ts[i].close !== undefined) i = ts[i].close;
  }
  if (start < call.end - 1) args.push([start, call.end - 1]);
  return args;
}

function stringValue(ts, start, end, crate, constants) {
  if (end === start + 1) {
    const value = ts[start]?.value ?? constants.get(ts[start]?.text);
    if (value === undefined) return null;
    // Today's audit takes the file's last `const NAME = "..."`; the narrowing rule accepts a
    // constant only when the file declares that name once.
    const doubt = ts[start]?.value === undefined && constants.ambiguous?.has(ts[start].text)
      ? `the constant ${ts[start].text} is declared more than once in this file` : null;
    return { value, manifest: false, ...(doubt ? { doubt } : {}) };
  }
  const call = callAt(ts, start);
  if (!call || call.end !== end) return null;
  const args = argumentsOf(ts, call);
  if (call.name === 'env!' && args.length === 1 && ts[args[0][0]]?.value === 'CARGO_MANIFEST_DIR') {
    return crate ? { value: crate, manifest: true } : null;
  }
  if (call.name !== 'concat!') return null;
  const parts = args.map(([a, b]) => stringValue(ts, a, b, crate, constants));
  const dynamic = parts.findIndex(p => !p || p.prefix);
  if (dynamic !== -1) {
    const known = parts.slice(0, dynamic + (parts[dynamic]?.prefix ? 1 : 0));
    // A macro argument can name any descendant. Hash the directory containing
    // the literal prefix, never the unfinished filename or a later literal.
    if (!known.some(p => p.prefix || (!p.manifest && p.value.includes('/')))) return null;
    const value = known.map(p => p.value).join('');
    return { value: value.slice(0, value.lastIndexOf('/') + 1), manifest: known.some(p => p.manifest), prefix: true };
  }
  const doubt = parts.find(p => p.doubt)?.doubt;
  return { value: parts.map(p => p.value).join(''), manifest: parts.some(p => p.manifest), ...(doubt ? { doubt } : {}) };
}

/**
 * `format!("{}/literal", env!("CARGO_MANIFEST_DIR"))` as the string it names: the template is one
 * string literal whose only placeholders are bare `{}`, each filled in order by an argument that
 * stringValue reads as a literal (a string, a once-declared constant, `env!("CARGO_MANIFEST_DIR")`,
 * a literal `concat!`). A named, numbered or formatted placeholder (`{name}`, `{0}`, `{:?}`), an
 * inline capture, or any other argument returns null: that join is not made only of literals.
 */
function literalFormat(ts, call, crate, constants) {
  const args = argumentsOf(ts, call);
  if (args.length < 2 || args[0][1] !== args[0][0] + 1) return null;
  const template = ts[args[0][0]]?.value;
  if (typeof template !== 'string') return null;
  const values = args.slice(1).map(([a, b]) => stringValue(ts, a, b, crate, constants));
  if (values.some(v => !v || v.prefix)) return null;
  let out = '';
  let next = 0;
  for (let i = 0; i < template.length; i++) {
    const two = template.slice(i, i + 2);
    if (two === '{{' || two === '}}') { out += two[0]; i++; }
    else if (two === '{}') {
      if (next >= values.length) return null;
      out += values[next++].value;
      i++;
    } else if (template[i] === '{' || template[i] === '}') return null;
    else out += template[i];
  }
  if (next !== values.length || !values.some(v => v.manifest)) return null;
  const doubt = values.find(v => v.doubt)?.doubt;
  return { value: out, manifest: true, ...(doubt ? { doubt } : {}) };
}

/** The `env` token of the first `env!("CARGO_MANIFEST_DIR")` in a token span: a join site's identity. */
function manifestSite(ts, from, to) {
  for (let i = from; i < to; i++) {
    if (ts[i].text === 'env' && ts[i + 1]?.text === '!' && ts[i + 3]?.value === 'CARGO_MANIFEST_DIR') return i;
  }
  return null;
}

/** Why a join argument is not a literal, in the words the narrow report prints. */
function nonLiteral(ts, start, end) {
  let at = start;
  while (ts[at]?.text === '&') at++;
  if (callAt(ts, at)?.name === 'format!') return 'joined with a format! placeholder';
  if (end === at + 1 && /^[A-Za-z_]\w*$/.test(ts[at]?.text || '')) return `joined with the variable ${ts[at].text}`;
  return 'joined with a non-literal expression';
}

/**
 * THE NARROWING RULE (GATE_READ_NARROW). One `CARGO_MANIFEST_DIR` join is audited as the path it
 * names only when every part of it is a literal; anything else keeps exactly what today's audit
 * records for it, never a guess and never nothing.
 *
 * A join is LITERAL when it is built only from: `env!("CARGO_MANIFEST_DIR")`; string literals; a
 * `const` declared once in the file with a string literal; `concat!` of those; `format!` whose
 * placeholders are all bare `{}` filled by those; `Path::new` / `PathBuf::from` of those;
 * `.join(<literal>)`; `.parent()` and `.ancestors().nth(<integer>)`; the pass-through calls
 * `canonicalize`, `unwrap`, `expect`, `to_path_buf`, `as_path`, `clone` and `?`; an immutable
 * `let` alias of such a chain; and a zero-argument helper whose whole body is such a chain.
 * `chain.why` names the first thing outside that list (a variable, a `format!` placeholder or loop
 * value, a macro parameter, a `mut` binding, an ancestor search, any other method), and the join
 * is then KEPT WIDE.
 *
 * A literal join is still kept wide when the path it names cannot be keyed as itself:
 *   - `..` is applied segment by segment, as the filesystem applies it, and only across a tracked
 *     directory; a `..` over anything else (a symbolic link, an absent or generated directory) is
 *     kept wide, because lexical normalisation would name a different file than the one read;
 *   - an absolute join, or a `..` that climbs out of the repository, is kept wide;
 *   - the repository root is kept wide (it is the whole tree, which is what today records);
 *   - a path that is not in the audited commit is kept wide: an absent file must never become a
 *     successful "no bytes" key;
 *   - a symbolic link, or a path whose kind cannot be read, is kept wide.
 * What survives is NARROWED: a file is that file; a directory is its whole subtree, and anything
 * later joined onto it by a non-literal keeps the directory.
 *
 * `chain`: `{ why, raw, path }` where `raw` is the absolute path before normalisation (when the
 * whole join is one string) and `path` the absolute path as today's audit resolved it.
 * `facts`: `{ root, kindOf(rel) }` with kindOf returning 'file', 'dir', 'symlink', or null (absent).
 * Returns `{ narrowed: <repo-relative path>, kind }` or `{ wide: <reason> }`.
 */
export function narrowJoin(chain, { root, kindOf }) {
  if (chain.why) return { wide: chain.why };
  const from = chain.raw ?? chain.path;
  if (typeof from !== 'string') return { wide: 'the join names no path' };
  const text = from.split('\\').join('/');
  const base = root.split('\\').join('/').replace(/\/+$/, '');
  if (text !== base && !text.startsWith(`${base}/`)) return { wide: 'an absolute path outside the repository' };
  const stack = [];
  for (const segment of text.slice(base.length).split('/')) {
    if (segment === '' || segment === '.') continue;
    if (segment !== '..') { stack.push(segment); continue; }
    if (stack.length === 0) return { wide: '`..` leaves the repository' };
    const crossed = stack.join('/');
    if (kindOf(crossed) !== 'dir') return { wide: `\`..\` crosses ${crossed}, which is not a tracked directory` };
    stack.pop();
  }
  const rel = stack.join('/');
  if (rel === '') return { wide: 'names the repository root, the whole tree' };
  const kind = kindOf(rel);
  if (kind === null) return { wide: `${rel} is absent at the audited commit` };
  if (kind !== 'file' && kind !== 'dir') return { wide: `${rel} is a symbolic link or of unknown kind` };
  return { narrowed: rel, kind };
}

function pathExpression(ts, start, crate, constants, bindings, helpers = new Map()) {
  while (ts[start]?.text === '&') start++;
  const call = callAt(ts, start);
  let result;
  if (call && /(?:^|::)(?:Path::new|PathBuf::from)$/.test(call.name)) {
    const args = argumentsOf(ts, call);
    const str = args.length === 1 && stringValue(ts, ...args[0], crate, constants);
    if (!str?.manifest) {
      if (!ts.slice(call.open + 1, call.end - 1).some(t => t.value === 'CARGO_MANIFEST_DIR')) return null;
      result = { path: crate, end: call.end, dynamic: true, origin: start, why: 'the path argument is not a literal' };
    } else result = { path: resolve(str.value), end: call.end, dynamic: false, prefix: str.prefix, origin: start, raw: str.value,
      why: str.prefix ? 'a concat! argument is not a literal' : str.doubt || null };
    result.site = manifestSite(ts, start, call.end);
  } else if (call && ['concat!', 'env!'].includes(call.name)) {
    const str = stringValue(ts, start, call.end, crate, constants);
    if (!str?.manifest) {
      if (!ts.slice(call.open + 1, call.end - 1).some(t => t.value === 'CARGO_MANIFEST_DIR')) return null;
      result = { path: crate, end: call.end, dynamic: true, origin: start, why: 'a concat! argument is not a literal' };
    } else result = { path: resolve(str.value), end: call.end, dynamic: false, prefix: str.prefix, origin: start, raw: str.value,
      why: str.prefix ? 'a concat! argument is not a literal' : str.doubt || null };
    result.site = manifestSite(ts, start, call.end);
    // A bare manifest string is not yet a path: where it goes next (an array, a format!, a call)
    // is not followed, so its join is never certified as narrowed.
    if (call.name === 'env!') result.bare = true;
  } else if (call && helpers.has(call.name) && argumentsOf(ts, call).length === 0 && !bindings.has(call.name)) {
    result = { ...helpers.get(call.name), end: call.end, origin: start, helper: call.name };
  } else if (bindings.get(ts[start]?.text)) {
    result = { ...bindings.get(ts[start].text), end: start + 1 };
  } else return null;

  while (ts[result.end]?.text === '.' || ts[result.end]?.text === '?') {
    if (ts[result.end].text === '?') { result.end++; continue; }
    const method = callAt(ts, result.end + 1);
    if (!method) break;
    const args = argumentsOf(ts, method);
    result.bare = false;
    if (method.name === 'join') {
      const str = args.length === 1 && stringValue(ts, ...args[0], crate, constants);
      if (args.length !== 1 || result.prefix) {
        result.dynamic = true;
        result.why ||= 'extended after a non-literal segment';
      } else if (!str) {
        result.prefix = true;
        result.why ||= nonLiteral(ts, ...args[0]);
      } else if (!result.dynamic) {
        result.path = resolve(result.path, str.value);
        result.raw = typeof result.raw === 'string' && !str.value.startsWith('/') ? `${result.raw}/${str.value}` : null;
        result.prefix = str.prefix;
        result.why ||= str.prefix ? 'a concat! argument is not a literal' : str.doubt || null;
      }
    } else if (result.prefix && ['parent', 'ancestors'].includes(method.name)) {
      result.dynamic = true;
      result.why ||= 'extended after a non-literal segment';
    } else if (method.name === 'parent' && result.path) { result.path = dirname(result.path); result.raw = null; }
    else if (method.name === 'ancestors') {
      const nth = ts[method.end]?.text === '.' && callAt(ts, method.end + 1);
      const count = nth && argumentsOf(ts, nth);
      if (nth?.name !== 'nth' || count.length !== 1 || count[0][1] !== count[0][0] + 1 || !/^\d+$/.test(ts[count[0][0]].text)) {
        result.dynamic = true;
        result.why ||= 'an ancestor search';
      } else {
        for (let n = Number(ts[count[0][0]].text); n > 0 && result.path; n--) result.path = dirname(result.path);
        result.raw = null;
        result.end = nth.end;
        continue;
      }
    } else if (!['canonicalize', 'unwrap', 'expect', 'to_path_buf', 'as_path', 'clone'].includes(method.name)) {
      // A method we cannot interpret may transform the path. Do not certify its prefix.
      result.dynamic = true;
      result.why ||= `the method .${method.name}() is not one the rule reads`;
    }
    result.end = method.end;
  }
  return result;
}

function nearestCrate(root, file, readFile) {
  for (let dir = dirname(resolve(root, file)); ; dir = dirname(dir)) {
    try { readFile(resolve(dir, 'Cargo.toml'), 'utf8'); return dir; }
    catch (error) { if (error.code !== 'ENOENT' && error.code !== 'ENOTDIR') throw error; }
    if (dir === root || dirname(dir) === dir) return null;
  }
}

function auditFile(root, file, text, crate, projectName = null, narrow = null) {
  const { ts, declarations } = tokens(text);
  const functions = functionsIn(ts);
  const constants = new Map();
  for (let i = 0; i < ts.length; i++) {
    if (ts[i].text !== 'const') continue;
    let eq = i + 2;
    while (eq < ts.length && !['=', ';', '{'].includes(ts[eq].text)) eq++;
    if (ts[eq]?.text === '=' && ts[eq + 1]?.value !== undefined) {
      if (constants.has(ts[i + 1].text)) (constants.ambiguous ||= new Set()).add(ts[i + 1].text);
      constants.set(ts[i + 1].text, ts[eq + 1].value);
    }
  }
  // One helper level only: recognize zero-argument functions whose entire body
  // is a literal manifest chain, without consulting other helpers recursively.
  const helpers = new Map();
  for (const fn of functions) {
    if (functions.filter(other => other.name === fn.name).length !== 1 ||
        ts[fn.start + 2]?.text !== '(' || ts[fn.start + 2].close !== fn.start + 3) continue;
    const path = pathExpression(ts, fn.body + 1, crate, constants, new Map(), new Map()) ?? boundTail(fn);
    if (path && path.end === fn.end && !path.dynamic && !path.prefix) helpers.set(fn.name, path);
  }
  // Review of E7 (doorway's schema_dir): one `let NAME = <manifest chain>;` then a tail chain on
  // NAME is the same literal path as the chain written in one expression. Nothing else is.
  function boundTail(fn) {
    const name = ts[fn.body + 2]?.text;
    if (ts[fn.body + 1]?.text !== 'let' || !/^[a-z_]\w*$/.test(name || '') || name === 'mut' || ts[fn.body + 3]?.text !== '=') return null;
    const bound = pathExpression(ts, fn.body + 4, crate, constants, new Map(), new Map());
    if (!bound || bound.dynamic || bound.prefix || ts[bound.end]?.text !== ';') return null;
    return pathExpression(ts, bound.end + 1, crate, constants,
      new Map([[name, { path: bound.path, dynamic: false, origin: bound.origin, site: bound.site, raw: bound.raw, why: bound.why }]]), new Map());
  }
  const usedHelpers = new Set();
  const candidates = [];
  const scopes = [new Map()];
  const mutableSeeds = new Set();
  // `chain` is the join's own record for the narrowing rule ({ site, why, raw, abs, bare }); it
  // never reaches a row, so an audit that does not narrow returns exactly what it always did.
  const add = (start, path, how, kind, origin = start, chain = null) => {
    candidates.push({ file, line: ts[start].line, path, how, kind, origin, chain });
  };
  const addPath = (start, path, kind = 'construction') => {
    if (path.helper) usedHelpers.add(path.helper);
    const rel = path.path ? relative(root, path.path).split('\\').join('/') || '.' : '.';
    const unresolved = path.dynamic || rel === '.' || rel === '..' || rel.startsWith('../');
    add(start, unresolved ? text.slice(ts[start].offset, ts[path.end - 1].offset + ts[path.end - 1].text.length).replace(/\s+/g, ' ') : rel,
      unresolved ? 'unresolved' : 'CARGO_MANIFEST_DIR', kind, path.origin,
      { site: path.site ?? null, why: path.why || (path.dynamic ? 'not a literal chain' : null), raw: path.raw ?? null, abs: path.path, bare: path.bare === true });
  };
  // Literal format! joins the rule narrowed (site -> true) or judged and kept wide (site -> reason).
  const formats = new Map();
  const bound = [];
  for (let i = 0; i < ts.length; i++) {
    const bindings = scopes.at(-1);
    if (ts[i].text === '{') { scopes.push(new Map(bindings)); continue; }
    if (ts[i].text === '}') { if (scopes.length > 1) scopes.pop(); continue; }
    if (ts[i].text === 'let') {
      const name = i + (ts[i + 1]?.text === 'mut' ? 2 : 1);
      let eq = name + 1;
      while (eq < ts.length && !['=', ';', '{'].includes(ts[eq].text)) eq++;
      const value = ts[eq]?.text === '=' && pathExpression(ts, eq + 1, crate, constants, bindings, helpers);
      bindings.set(ts[name]?.text, null); // shadowed fixture roots/callables must not inherit a live path
      if (value) {
        if (ts[i + 1]?.text === 'mut') {
          value.dynamic = true;
          value.why ||= 'a mut binding';
          mutableSeeds.add(eq + 1);
        }
        bindings.set(ts[name].text, value);
        // A join bound to a name is judged even when no read the audit follows ever uses it.
        if (value.site != null) {
          bound.push({ site: value.site, why: value.why || (value.dynamic ? 'not a literal chain' : null),
            raw: value.raw ?? null, abs: value.path, bare: value.bare === true });
        }
      }
    }
    const call = callAt(ts, i);
    if (!call || ['fn', '.', '::'].includes(ts[i - 1]?.text)) continue;
    if (['include_str!', 'include_bytes!'].includes(call.name)) {
      const args = argumentsOf(ts, call);
      const str = args.length === 1 && stringValue(ts, ...args[0], crate, constants);
      // Rust's plain include paths are source-relative, unlike manifest-based paths.
      const path = str && (relative(root, resolve(str.manifest ? crate : dirname(resolve(root, file)), str.value)).split('\\').join('/') || '.');
      const unresolved = !str || path === '.' || path === '..' || path.startsWith('../');
      const site = manifestSite(ts, i, call.end);
      add(i, unresolved ? text.slice(ts[i].offset, ts[call.end - 1].offset + 1).replace(/\s+/g, ' ') : path,
        unresolved ? 'unresolved' : call.name.slice(0, -1), 'read', i, site === null ? null : {
          site, bare: false, abs: str?.manifest && !str.prefix ? resolve(str.value) : null,
          raw: str?.manifest && !str.prefix ? str.value : null,
          why: !str ? 'the include path is not a literal' : str.prefix ? 'a concat! argument is not a literal' : str.doubt || null,
        });
      i = call.end - 1;
      continue;
    }
    // The one literal form today's audit cannot read: a format! filled only by literals. Narrowed
    // only when the rule accepts the path it names; otherwise fall through, so the manifest
    // string inside is recorded exactly as it is today.
    if (narrow && call.name === 'format!') {
      const str = literalFormat(ts, call, crate, constants);
      const site = str && manifestSite(ts, i, call.end);
      if (str && site !== null) {
        const verdict = narrowJoin({ why: str.doubt || null, raw: str.value }, narrow);
        formats.set(site, verdict.narrowed ? true : verdict.wide);
        if (verdict.narrowed) {
          add(i, verdict.narrowed, 'CARGO_MANIFEST_DIR', 'read', i, { site, why: null, raw: str.value, abs: null, bare: false });
          i = call.end - 1;
          continue;
        }
      }
    }
    if (/(?:^|::)(?:fs::read\w*|File::open|read_to_string|read_dir)$/.test(call.name)) {
      const args = argumentsOf(ts, call);
      const value = args.length && pathExpression(ts, args[0][0], crate, constants, bindings, helpers);
      if (value) { addPath(i, { ...value, end: call.end }, 'read'); i = call.end - 1; continue; }
    }
    const path = pathExpression(ts, i, crate, constants, new Map(), bindings.has(call.name) ? new Map() : helpers);
    if (path) {
      if (mutableSeeds.has(i)) { path.dynamic = true; path.why ||= 'a mut binding'; }
      addPath(i, path);
      i = path.end - 1;
    } else if (call.name === 'env!' && ts[call.open + 1]?.value === 'CARGO_MANIFEST_DIR') {
      add(i, 'CARGO_MANIFEST_DIR (unresolved context)', 'unresolved', 'construction', i,
        { site: i, why: 'a context the rule does not read', raw: null, abs: null, bare: true });
      i = call.end - 1;
    }
  }
  // A named base subsequently read at a concrete descendant is only construction.
  // Keep root/dynamic candidates: a descendant does not prove the rest of a walk.
  const helperOrigins = new Set([...usedHelpers].map(name => helpers.get(name).origin));
  const owner = offset => functions.filter(fn => ts[fn.body].offset < offset && offset < ts[fn.end].offset).at(-1);
  // A declaration (or a stack of them) names the reads of the ONE statement it sits above:
  // the rows whose construction begins there, and the rows derived from that construction
  // through bindings (they share its origin). Another walk in the same function stays
  // unresolved, so a fixed-read declaration cannot mask a data-derived walk beside it.
  const statementAfter = offset => {
    const from = ts.findIndex(token => token.offset > offset);
    if (from < 0) return null;
    let to = from;
    for (let j = from; j < ts.length; j++) {
      to = j;
      if (ts[j].text === ';') break;
      if ([')', ']', '}'].includes(ts[j].text)) { to = j - 1; break; }
      if (ts[j].close === undefined) continue;
      j = ts[j].close;
      to = j;
      if (ts[j].text === '}' && !['.', '?', ';', 'else'].includes(ts[j + 1]?.text)) break;
    }
    return { from, to };
  };
  const spans = declarations.map(declaration => ({ ...declaration, span: statementAfter(declaration.offset) }));
  const kept = candidates.filter(row => !helperOrigins.has(row.origin)).filter(row => row.how === 'unresolved' || row.kind === 'read' ||
    candidates.some(other => other.kind === 'read' && other.path === row.path) ||
    !candidates.some(other => other.origin === row.origin && other.kind === 'read' && other.how !== 'unresolved' && other.path.startsWith(`${row.path}/`)),
  ).flatMap(({ kind, origin, chain, ...row }) => declaredRows(row, origin).map(final => ({ row: final, chain })));
  const rows = kept.map(({ row }) => row);
  return { rows, joins: narrow ? joinsOf() : [] };

  /**
   * One entry per `env!("CARGO_MANIFEST_DIR")` the scan reached: narrowed (to what) or kept wide
   * (why, and what today's audit records for it, which is what stays keyed).
   */
  function joinsOf() {
    const sites = new Map();
    const at = site => sites.get(site) || sites.set(site, { made: [], kept: [] }).get(site);
    for (const candidate of candidates) if (candidate.chain?.site != null) at(candidate.chain.site).made.push(candidate.chain);
    for (const chain of bound) at(chain.site).made.push(chain);
    for (const entry of kept) if (entry.chain?.site != null) at(entry.chain.site).kept.push(entry);
    return [...sites].sort(([a], [b]) => a - b).map(([site, { made, kept: stays }]) => {
      const keeps = [...new Set(stays.map(({ row }) => (row.how === 'unresolved' ? 'the whole tree (unresolved)' : row.path)))].sort();
      const wide = reason => ({ file, line: ts[site].line, wide: reason, keeps });
      const judged = formats.get(site);
      if (typeof judged === 'string') return wide(judged);
      const why = made.find(chain => chain.why)?.why;
      if (why) return wide(why);
      if (made.every(chain => chain.bare)) return wide('a bare manifest string whose uses are not followed');
      if (stays.length === 0) return wide('no read of it is keyed for this gate: declared for another gate project, or never passed to a read the audit follows');
      if (stays.some(({ row }) => row.how === 'gate-reads')) return wide('declared with gate-reads: the declaration is what is keyed');
      for (const { row, chain } of stays) {
        const verdict = narrowJoin({ why: null, raw: chain.raw, path: chain.abs }, narrow);
        if (verdict.wide) return wide(verdict.wide);
        if (row.how === 'unresolved' || verdict.narrowed !== row.path) return wide('the rule and the audit name different paths');
      }
      for (const chain of made) {
        if (typeof chain.abs !== 'string') continue;
        const verdict = narrowJoin({ why: null, raw: chain.raw, path: chain.abs }, narrow);
        if (verdict.wide) return wide(verdict.wide);
      }
      return { file, line: ts[site].line, narrowed: keeps };
    });
  }

  function declaredRows(row, origin) {
    if (row.how !== 'unresolved') return [row];
    const fn = owner(ts[origin].offset);
    const declared = spans.filter(declaration => {
      if (!declaration.span || origin < declaration.span.from || origin > declaration.span.to) return false;
      const declarationFn = owner(declaration.offset);
      return (fn && declarationFn === fn) ||
        (!declarationFn && declaration.line <= row.line && row.line - declaration.line <= 15);
    }).filter(({ path }) => {
      if (isReadPattern(path)) return true;
      const rel = relative(root, resolve(root, path));
      return !path.startsWith('/') && !path.includes('\\') && !path.includes('*') && rel !== '..' && !rel.startsWith('../');
    });
    const mine = declared.filter(({ project }) => !project || !projectName || project === projectName);
    if (mine.length) return mine.map(({ path }) => ({ ...row, path, how: 'gate-reads' }));
    // Every declaration on this statement names another gate project: that project's recipe runs
    // the read and keys on it; this project's recipe never executes the statement.
    return declared.length ? [] : [row];
  }
}

/**
 * Synchronous injected IO: listFiles(root) returns repo-relative file names;
 * readFile(absolutePath, 'utf8') returns text and throws ENOENT for absent files.
 * Scan source-matched Rust files, including tests in other declared crates.
 * Literal chains, local bindings and one-level local helpers are interpreted.
 * Mutation/ancestor searches and other data-derived paths require declarations or review of the printed
 * unresolved roots; this audit is not a proof of an arbitrary Rust read closure.
 */
function scanVariants({
  root, sources, project = null, readFile = readFileSync, listFiles = repositoryFiles,
  listSymlinks = listFiles === repositoryFiles ? repositorySymlinks : () => [],
}, variants, measureVariant = (_name, run) => run()) {
  root = resolve(root);
  const files = [...new Set(listFiles(root))].sort();
  const contents = new Map();
  const cachedRead = (path, encoding = 'utf8') => {
    if (!contents.has(path)) contents.set(path, readFile(path, encoding));
    return contents.get(path);
  };
  // What the narrowing rule may know of a path: built only when a variant narrows, and the
  // symbolic-link listing only when a join's target is first judged.
  let kinds = null;
  let links;
  const kindOf = rel => {
    if (!kinds) {
      kinds = new Map(files.map(file => [file, 'file']));
      for (const file of files) {
        for (let cut = file.lastIndexOf('/'); cut > 0; cut = file.lastIndexOf('/', cut - 1)) {
          const dir = file.slice(0, cut);
          if (kinds.has(dir)) break;
          kinds.set(dir, 'dir');
        }
      }
    }
    const kind = kinds.get(rel) ?? null;
    if (kind !== 'file') return kind;
    if (links === undefined) {
      try { links = new Set(listSymlinks(root)); } catch { links = null; }
    }
    return links === null ? 'unknown' : links.has(rel) ? 'symlink' : 'file';
  };
  const sourceFiles = files.filter(file => file.endsWith('.rs') && sources.some(glob => globCovers(glob, file)));
  const prepared = sourceFiles.map(file => ({
    file,
    crate: nearestCrate(root, file, cachedRead),
    text: cachedRead(resolve(root, file), 'utf8'),
  }));
  const found = new Map(variants.map(({ name }) => [name, { rows: new Map(), joins: [] }]));
  for (const { file, crate, text } of prepared) {
    for (const { name, variant } of variants) {
      measureVariant(name, () => {
        const audited = auditFile(root, file, text, crate, project?.name ?? null,
          variant.narrow === true ? { root, kindOf } : null);
        for (const row of audited.rows) found.get(name).rows.set(JSON.stringify(row), row);
        found.get(name).joins.push(...audited.joins);
      });
    }
  }
  return new Map([...found].map(([name, { rows, joins }]) => [name, { rows: [...rows.values()], joins }]));
}

/** Tracked symbolic links of a checkout, for the narrowing rule (a link is never narrowed). */
export function repositorySymlinks(root) {
  const result = spawnSync('git', ['ls-files', '-s', '-z'], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(result.error?.message || result.stderr || 'git ls-files -s failed');
  return result.stdout.split('\0').filter(row => row.startsWith('120000 ')).map(row => row.slice(row.indexOf('\t') + 1));
}

export function scan({ variant = {}, ...options }) {
  return scanVariants(options, [{ name: 'only', variant }]).get('only').rows;
}

/**
 * Every CARGO_MANIFEST_DIR join the scan reaches, as the narrowing rule (narrowJoin) judges it:
 * `{ file, line, narrowed: [paths] }` or `{ file, line, wide: reason, keeps: [what stays keyed] }`.
 * It needs listFiles to return every tracked file, not only Rust sources.
 */
export function manifestJoins(options) {
  return scanVariants(options, [{ name: 'only', variant: { narrow: true } }]).get('only').joins;
}

/** Every named repository read, including declared files and directory prefixes. */
export function readSet(options) {
  const rows = scan(options);
  return {
    paths: [...new Set(rows.filter(row => row.how !== 'unresolved').map(row => row.path))].sort(),
    unresolved: rows.filter(row => row.how === 'unresolved').map(({ file, line, path }) => ({ file, line, expr: path })),
  };
}

/** Derive several audit variants from one repository listing and one shared source-file cache. */
export function readSetVariants(options, variants) {
  const clock = options.clock || (() => process.hrtime.bigint());
  const elapsedMs = started => {
    const elapsed = clock() - started;
    return typeof elapsed === 'bigint' ? Number(elapsed) / 1_000_000 : Number(elapsed);
  };
  const variantMs = Object.fromEntries(variants.map(({ name }) => [name, 0]));
  const started = clock();
  const rows = scanVariants(options, variants, (name, run) => {
    const at = clock();
    const result = run();
    variantMs[name] += elapsedMs(at);
    return result;
  });
  const totalMs = elapsedMs(started);
  const sets = new Map([...rows].map(([name, { rows: found }]) => [name, {
    paths: [...new Set(found.filter(row => row.how !== 'unresolved').map(row => row.path))].sort(),
    unresolved: found.filter(row => row.how === 'unresolved')
      .map(({ file, line, path }) => ({ file, line, expr: path })),
  }]));
  // Per narrowing variant only: how many joins the rule narrowed and how many it kept wide.
  const joins = new Map([...rows].filter(([name]) => variants.find(v => v.name === name).variant.narrow === true)
    .map(([name, { joins: all }]) => [name, { narrowed: all.filter(j => j.narrowed).length, wide: all.filter(j => j.wide).length }]));
  return { sets, joins, timing: { sharedMs: Math.max(0, totalMs - Object.values(variantMs).reduce((a, b) => a + b, 0)), variantMs } };
}

export function auditProject(options) {
  const { project, sources } = options;
  return { undeclared: scan(options).filter(row => row.how === 'unresolved' ||
    !(globCovers(`${project.dir}/**`, row.path) || sources.some(glob => globCovers(glob, row.path)))) };
}

/** Every tracked file of a checkout: the narrowing rule judges targets that are not Rust. */
function trackedFiles(root) {
  const result = spawnSync('git', ['ls-files', '-z'], { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  if (result.status !== 0) throw new Error(result.error?.message || result.stderr || 'git ls-files failed');
  return result.stdout.split('\0').filter(Boolean);
}

/**
 * `--narrow-report [project ...]`: per cargo gate project, each CARGO_MANIFEST_DIR join its
 * sources reach, as GATE_READ_NARROW=on would audit it: narrowed (to what) or kept wide (why, and
 * what stays keyed). Read from the checkout's tracked files. It changes no key and no knob.
 */
function narrowReport(root, names, write = text => process.stdout.write(text)) {
  const registry = loadGateRegistry(root);
  for (const name of names) if (!registry.has(name)) throw new Error(`Unknown gate project: ${name}`);
  const files = trackedFiles(root);
  const once = new Map();
  for (const [name, project] of registry) {
    if (!project.run?.cargo || (names.length && !names.includes(name))) continue;
    const manifest = JSON.parse(readFileSync(resolve(root, project.manifestPath), 'utf8'));
    const sources = [...new Set((project.steps || []).flatMap(step => manifest.steps?.[step]?.inputs?.sources || []))];
    const joins = manifestJoins({ root, project, sources, listFiles: () => files, listSymlinks: repositorySymlinks });
    write(`## ${name}: ${joins.filter(j => j.narrowed).length} narrowed, ${joins.filter(j => j.wide).length} kept wide\n`);
    for (const join of joins) {
      once.set(`${join.file}:${join.line}`, join);
      write(join.narrowed
        ? `  narrowed  ${join.file}:${join.line} -> ${join.narrowed.join(', ')}\n`
        : `  wide      ${join.file}:${join.line} (${join.wide}) keeps ${join.keeps.join(', ') || 'nothing for this gate'}\n`);
    }
  }
  const all = [...once.values()];
  write(`## every join once: ${all.filter(j => j.narrowed).length} narrowed, ${all.filter(j => j.wide).length} kept wide\n`);
  return 0;
}

function main(name, ...rest) {
  if (name === '--narrow-report') {
    return narrowReport(resolve(dirname(fileURLToPath(import.meta.url)), '../..'), rest);
  }
  if (!name) throw new Error('Usage: node gate-input-audit.mjs <project-name> | --narrow-report [project ...]');
  const root = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
  const project = loadGateRegistry(root).get(name);
  if (!project) throw new Error(`Unknown gate project: ${name}`);
  const manifest = JSON.parse(readFileSync(resolve(root, project.manifestPath), 'utf8'));
  const sources = [...new Set(project.steps.flatMap(step => manifest.steps[step].inputs?.sources || []))];
  const { undeclared } = auditProject({ root, project, sources });
  for (const row of undeclared) console.log(`${row.file}:${row.line}: ${row.how === 'unresolved' ? 'unresolved' : 'undeclared'} ${row.path} (${row.how})`);
  return undeclared.some(row => row.how !== 'unresolved') ? 1 : 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try { process.exitCode = main(...process.argv.slice(2)); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
