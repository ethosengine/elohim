#!/usr/bin/env node
// Static hints for repository reads outside a gate's declared input closure.
// This is deliberately not a Rust compiler: unrecognized transformations and
// repository walks need explicit gate-reads declarations or remain unresolved.

import { spawnSync } from 'child_process';
import { readFileSync } from 'fs';
import { dirname, relative, resolve } from 'path';
import { fileURLToPath } from 'url';
import { loadGateRegistry } from './pipeline-registry.mjs';

/** Deliberately simpler than full glob syntax: a terminal /** is a subtree; otherwise exact. */
export function globCovers(glob, path) {
  if (glob === '**') return true;
  if (!glob.endsWith('/**')) return glob === path;
  const prefix = glob.slice(0, -3);
  return path === prefix || path.startsWith(`${prefix}/`);
}

function repositoryFiles(root) {
  const result = spawnSync('git', ['ls-files', '-z', '--cached', '--others', '--exclude-standard', '--', '*.rs'], {
    cwd: root, encoding: 'utf8', maxBuffer: 16 * 1024 * 1024,
  });
  if (result.status !== 0) throw new Error(result.error?.message || result.stderr || 'git ls-files failed');
  return [...new Set(result.stdout.split('\0').filter(Boolean))];
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
    const declaration = raw.match(/^\/\/\s*gate-reads:\s*(\S.*?)\s*$/);
    if (declaration && /^\s*$/.test(text.slice(text.lastIndexOf('\n', offset - 1) + 1, offset))) {
      declarations.push({ path: declaration[1], offset, line });
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

function pathExpression(ts, start, crate, constants, bindings, helpers = new Map()) {
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
      if (args.length !== 1 || result.prefix) result.dynamic = true;
      else if (!str) result.prefix = true;
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

function auditFile(root, file, text, crate) {
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
    const path = pathExpression(ts, fn.body + 1, crate, constants, new Map());
    if (path && path.end === fn.end && !path.dynamic && !path.prefix) helpers.set(fn.name, path);
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
      const value = ts[eq]?.text === '=' && pathExpression(ts, eq + 1, crate, constants, bindings, helpers);
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
      const value = args.length && pathExpression(ts, args[0][0], crate, constants, bindings, helpers);
      if (value) { addPath(i, { ...value, end: call.end }, 'read'); i = call.end - 1; continue; }
    }
    const path = pathExpression(ts, i, crate, constants, new Map(), bindings.has(call.name) ? new Map() : helpers);
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
  return candidates.filter(row => !helperOrigins.has(row.origin)).filter(row => row.how === 'unresolved' || row.kind === 'read' ||
    candidates.some(other => other.kind === 'read' && other.path === row.path) ||
    !candidates.some(other => other.origin === row.origin && other.kind === 'read' && other.how !== 'unresolved' && other.path.startsWith(`${row.path}/`)),
  ).flatMap(({ kind, origin, ...row }) => {
    if (row.how !== 'unresolved') return [row];
    const fn = owner(ts[origin].offset);
    const declared = declarations.filter(declaration => {
      const declarationFn = owner(declaration.offset);
      return (fn && declarationFn === fn) ||
        (!declarationFn && declaration.line <= row.line && row.line - declaration.line <= 15);
    }).filter(({ path }) => {
      const rel = relative(root, resolve(root, path));
      return !path.startsWith('/') && !path.includes('\\') && rel !== '..' && !rel.startsWith('../');
    });
    return declared.length ? declared.map(({ path }) => ({ ...row, path, how: 'gate-reads' })) : [row];
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
function scan({ root, sources, readFile = readFileSync, listFiles = repositoryFiles }) {
  root = resolve(root);
  const found = new Map();
  for (const file of [...new Set(listFiles(root))].sort()) {
    if (!file.endsWith('.rs') || !sources.some(glob => globCovers(glob, file))) continue;
    const rows = auditFile(root, file, readFile(resolve(root, file), 'utf8'), nearestCrate(root, file, readFile));
    for (const row of rows) found.set(JSON.stringify(row), row);
  }
  return [...found.values()];
}

/** Every named repository read, including declared files and directory prefixes. */
export function readSet(options) {
  const rows = scan(options);
  return {
    paths: [...new Set(rows.filter(row => row.how !== 'unresolved').map(row => row.path))].sort(),
    unresolved: rows.filter(row => row.how === 'unresolved').map(({ file, line, path }) => ({ file, line, expr: path })),
  };
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
