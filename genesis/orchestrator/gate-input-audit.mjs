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
    return value === undefined ? null : { value, manifest: false };
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
  return { value: parts.map(p => p.value).join(''), manifest: parts.some(p => p.manifest) };
}

function formatJoinDirectory(ts, start, end) {
  const call = callAt(ts, start);
  if (!call || call.name !== 'format!' || call.end !== end) return null;
  const args = argumentsOf(ts, call);
  if (args.length === 0 || args[0][1] !== args[0][0] + 1) return null;
  const template = ts[args[0][0]]?.value;
  if (typeof template !== 'string') return null;
  const dynamic = template.indexOf('{');
  if (dynamic < 0) return null;
  // The dynamic value may choose a filename in the literal directory, but it must not choose a
  // descendant (or climb back out of one). Such a suffix cannot be represented by one directory
  // key, so leave the read at the crate fallback instead.
  const dynamicSuffix = template.slice(dynamic);
  if (dynamicSuffix.includes('/') || dynamicSuffix.includes('\\') || dynamicSuffix.includes('..')) return null;
  const prefix = template.slice(0, dynamic);
  if (!prefix) return null;
  if (prefix.endsWith('/')) return prefix.replace(/\/+$/, '');
  const slash = prefix.lastIndexOf('/');
  return slash < 0 ? null : prefix.slice(0, slash);
}

function pathExpression(ts, start, crate, constants, bindings, helpers = new Map(), variant = {}) {
  while (ts[start]?.text === '&') start++;
  const call = callAt(ts, start);
  let result;
  if (call && /(?:^|::)(?:Path::new|PathBuf::from)$/.test(call.name)) {
    const args = argumentsOf(ts, call);
    const str = args.length === 1 && stringValue(ts, ...args[0], crate, constants);
    if (!str?.manifest) {
      if (!ts.slice(call.open + 1, call.end - 1).some(t => t.value === 'CARGO_MANIFEST_DIR')) return null;
      result = { path: crate, end: call.end, dynamic: true, origin: start };
    } else result = { path: resolve(str.value), end: call.end, dynamic: false, prefix: str.prefix, origin: start };
  } else if (call && ['concat!', 'env!'].includes(call.name)) {
    const str = stringValue(ts, start, call.end, crate, constants);
    if (!str?.manifest) {
      if (!ts.slice(call.open + 1, call.end - 1).some(t => t.value === 'CARGO_MANIFEST_DIR')) return null;
      result = { path: crate, end: call.end, dynamic: true, origin: start };
    } else result = { path: resolve(str.value), end: call.end, dynamic: false, prefix: str.prefix, origin: start };
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
    if (method.name === 'join') {
      const str = args.length === 1 && stringValue(ts, ...args[0], crate, constants);
      const formatDir = !str && variant.resolveFormatJoin && args.length === 1
        ? formatJoinDirectory(ts, ...args[0]) : null;
      if (args.length !== 1 || result.prefix) result.dynamic = true;
      else if (formatDir) {
        const candidate = !result.dynamic && resolve(result.path, formatDir);
        const rel = candidate && variant.root
          ? relative(variant.root, candidate).split('\\').join('/') : null;
        if (candidate && (!rel || (rel !== '..' && !rel.startsWith('../')))
            && (!variant.pathExists || variant.pathExists(candidate))) result.path = candidate;
        result.prefix = true;
      } else if (!str) result.prefix = true;
      else if (!result.dynamic) {
        result.path = resolve(result.path, str.value);
        result.prefix = str.prefix;
      }
    } else if (result.prefix && ['parent', 'ancestors'].includes(method.name)) result.dynamic = true;
    else if (method.name === 'parent' && result.path) result.path = dirname(result.path);
    else if (method.name === 'ancestors') {
      const nth = ts[method.end]?.text === '.' && callAt(ts, method.end + 1);
      const count = nth && argumentsOf(ts, nth);
      if (nth?.name !== 'nth' || count.length !== 1 || count[0][1] !== count[0][0] + 1 || !/^\d+$/.test(ts[count[0][0]].text)) {
        result.dynamic = true;
      } else {
        for (let n = Number(ts[count[0][0]].text); n > 0 && result.path; n--) result.path = dirname(result.path);
        result.end = nth.end;
        continue;
      }
    } else if (!['canonicalize', 'unwrap', 'expect', 'to_path_buf', 'as_path', 'clone'].includes(method.name)) {
      // A method we cannot interpret may transform the path. Do not certify its prefix.
      result.dynamic = true;
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

/** Literal modules a Rust source includes with `#[path = "..."]`. */
function pathIncludedModules(text) {
  const { ts } = tokens(text);
  const paths = [];
  for (let i = 0; i < ts.length - 5; i++) {
    if (ts[i].text !== '#' || ts[i + 1].text !== '[' || ts[i + 1].close !== i + 5) continue;
    if (ts[i + 2].text === 'path' && ts[i + 3].text === '=' && typeof ts[i + 4].value === 'string') {
      paths.push(ts[i + 4].value);
    }
  }
  return paths;
}

function auditFile(root, file, text, crate, projectName = null, variant = {}) {
  const { ts, declarations } = tokens(text);
  const functions = functionsIn(ts);
  const constants = new Map();
  for (let i = 0; i < ts.length; i++) {
    if (ts[i].text !== 'const') continue;
    let eq = i + 2;
    while (eq < ts.length && !['=', ';', '{'].includes(ts[eq].text)) eq++;
    if (ts[eq]?.text === '=' && ts[eq + 1]?.value !== undefined) constants.set(ts[i + 1].text, ts[eq + 1].value);
  }
  // One helper level only: recognize zero-argument functions whose entire body
  // is a literal manifest chain, without consulting other helpers recursively.
  const helpers = new Map();
  for (const fn of functions) {
    if (functions.filter(other => other.name === fn.name).length !== 1 ||
        ts[fn.start + 2]?.text !== '(' || ts[fn.start + 2].close !== fn.start + 3) continue;
    const path = pathExpression(ts, fn.body + 1, crate, constants, new Map(), new Map(), variant) ?? boundTail(fn);
    if (path && path.end === fn.end && !path.dynamic && !path.prefix) helpers.set(fn.name, path);
  }
  // Review of E7 (doorway's schema_dir): one `let NAME = <manifest chain>;` then a tail chain on
  // NAME is the same literal path as the chain written in one expression. Nothing else is.
  function boundTail(fn) {
    const name = ts[fn.body + 2]?.text;
    if (ts[fn.body + 1]?.text !== 'let' || !/^[a-z_]\w*$/.test(name || '') || name === 'mut' || ts[fn.body + 3]?.text !== '=') return null;
    const bound = pathExpression(ts, fn.body + 4, crate, constants, new Map(), new Map(), variant);
    if (!bound || bound.dynamic || bound.prefix || ts[bound.end]?.text !== ';') return null;
    return pathExpression(ts, bound.end + 1, crate, constants,
      new Map([[name, { path: bound.path, dynamic: false, origin: bound.origin }]]), new Map(), variant);
  }
  const usedHelpers = new Set();
  const candidates = [];
  const scopes = [new Map()];
  const mutableSeeds = new Set();
  const add = (start, path, how, kind, origin = start) => {
    candidates.push({ file, line: ts[start].line, path, how, kind, origin });
  };
  const addPath = (start, path, kind = 'construction') => {
    if (path.helper) usedHelpers.add(path.helper);
    const rel = path.path ? relative(root, path.path).split('\\').join('/') || '.' : '.';
    const unresolved = path.dynamic || rel === '.' || rel === '..' || rel.startsWith('../');
    add(start, unresolved ? text.slice(ts[start].offset, ts[path.end - 1].offset + ts[path.end - 1].text.length).replace(/\s+/g, ' ') : rel,
      unresolved ? 'unresolved' : 'CARGO_MANIFEST_DIR', kind, path.origin);
  };
  for (let i = 0; i < ts.length; i++) {
    const bindings = scopes.at(-1);
    if (ts[i].text === '{') { scopes.push(new Map(bindings)); continue; }
    if (ts[i].text === '}') { if (scopes.length > 1) scopes.pop(); continue; }
    if (ts[i].text === 'let') {
      const name = i + (ts[i + 1]?.text === 'mut' ? 2 : 1);
      let eq = name + 1;
      while (eq < ts.length && !['=', ';', '{'].includes(ts[eq].text)) eq++;
      const value = ts[eq]?.text === '=' && pathExpression(ts, eq + 1, crate, constants, bindings, helpers, variant);
      bindings.set(ts[name]?.text, null); // shadowed fixture roots/callables must not inherit a live path
      if (value) {
        if (ts[i + 1]?.text === 'mut') {
          value.dynamic = true;
          mutableSeeds.add(eq + 1);
        }
        bindings.set(ts[name].text, value);
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
      add(i, unresolved ? text.slice(ts[i].offset, ts[call.end - 1].offset + 1).replace(/\s+/g, ' ') : path,
        unresolved ? 'unresolved' : call.name.slice(0, -1), 'read');
      i = call.end - 1;
      continue;
    }
    if (/(?:^|::)(?:fs::read\w*|File::open|read_to_string|read_dir)$/.test(call.name)) {
      const args = argumentsOf(ts, call);
      const value = args.length && pathExpression(ts, args[0][0], crate, constants, bindings, helpers, variant);
      if (value) { addPath(i, { ...value, end: call.end }, 'read'); i = call.end - 1; continue; }
    }
    const path = pathExpression(ts, i, crate, constants, new Map(), bindings.has(call.name) ? new Map() : helpers, variant);
    if (path) {
      if (mutableSeeds.has(i)) path.dynamic = true;
      addPath(i, path);
      i = path.end - 1;
    } else if (call.name === 'env!' && ts[call.open + 1]?.value === 'CARGO_MANIFEST_DIR') {
      add(i, 'CARGO_MANIFEST_DIR (unresolved context)', 'unresolved', 'construction');
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
  return candidates.filter(row => !helperOrigins.has(row.origin)).filter(row => row.how === 'unresolved' || row.kind === 'read' ||
    candidates.some(other => other.kind === 'read' && other.path === row.path) ||
    !candidates.some(other => other.origin === row.origin && other.kind === 'read' && other.how !== 'unresolved' && other.path.startsWith(`${row.path}/`)),
  ).flatMap(({ kind, origin, ...row }) => {
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
  });
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
}, variants, measureVariant = (_name, run) => run()) {
  root = resolve(root);
  const files = [...new Set(listFiles(root))].sort();
  const contents = new Map();
  const cachedRead = (path, encoding = 'utf8') => {
    if (!contents.has(path)) contents.set(path, readFile(path, encoding));
    return contents.get(path);
  };
  const pathExists = target => {
    const rel = relative(root, target).split('\\').join('/');
    return rel !== '..' && !rel.startsWith('../')
      && files.some(file => file === rel || file.startsWith(`${rel}/`));
  };
  const sourceFiles = files.filter(file => file.endsWith('.rs') && sources.some(glob => globCovers(glob, file)));
  const prepared = sourceFiles.map(file => ({
    file,
    crate: nearestCrate(root, file, cachedRead),
    text: cachedRead(resolve(root, file), 'utf8'),
  }));
  const includedDependencyTests = new Set();
  if (variants.some(({ variant }) => variant.excludeDependencyTests) && project) {
    for (const { file, crate, text } of prepared) {
      if (!crate) continue;
      const inCrate = relative(crate, resolve(root, file)).split('\\').join('/');
      if (inCrate.startsWith('tests/')) continue;
      for (const path of pathIncludedModules(text)) {
        const included = relative(root, resolve(dirname(resolve(root, file)), path)).split('\\').join('/');
        if (included !== '..' && !included.startsWith('../')) includedDependencyTests.add(included);
      }
    }
  }
  const found = new Map(variants.map(({ name }) => [name, new Map()]));
  for (const { file, crate, text } of prepared) {
    for (const { name, variant } of variants) {
      measureVariant(name, () => {
        if (variant.excludeDependencyTests && project && crate) {
          const projectDir = resolve(root, project.dir);
          const fromProject = relative(projectDir, crate).split('\\').join('/');
          const crateInProject = fromProject === '' || (fromProject !== '..' && !fromProject.startsWith('../'));
          const inCrate = relative(crate, resolve(root, file)).split('\\').join('/');
          if (!crateInProject && inCrate.startsWith('tests/') && !includedDependencyTests.has(file)) return;
        }
        const rows = auditFile(root, file, text, crate, project?.name ?? null,
          { ...variant, root, pathExists });
        for (const row of rows) found.get(name).set(JSON.stringify(row), row);
      });
    }
  }
  return new Map([...found].map(([name, rows]) => [name, [...rows.values()]]));
}

export function scan({ root, sources, project = null, readFile = readFileSync, listFiles = repositoryFiles, variant = {} }) {
  return scanVariants({ root, sources, project, readFile, listFiles }, [{ name: 'only', variant }]).get('only');
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
  const sets = new Map([...rows].map(([name, found]) => [name, {
    paths: [...new Set(found.filter(row => row.how !== 'unresolved').map(row => row.path))].sort(),
    unresolved: found.filter(row => row.how === 'unresolved')
      .map(({ file, line, path }) => ({ file, line, expr: path })),
  }]));
  return { sets, timing: { sharedMs: Math.max(0, totalMs - Object.values(variantMs).reduce((a, b) => a + b, 0)), variantMs } };
}

export function auditProject(options) {
  const { project, sources } = options;
  return { undeclared: scan(options).filter(row => row.how === 'unresolved' ||
    !(globCovers(`${project.dir}/**`, row.path) || sources.some(glob => globCovers(glob, row.path)))) };
}

function main(name) {
  if (!name) throw new Error('Usage: node gate-input-audit.mjs <project-name>');
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
  try { process.exitCode = main(process.argv[2]); }
  catch (error) { console.error(error.message); process.exitCode = 1; }
}
