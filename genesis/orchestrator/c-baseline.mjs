#!/usr/bin/env node
// A gate on a declared workspace builds its C at the declared level and nothing above it.
//
// Backlog steward-device-arrival-open-items row 29.2: RHEL 10 gcc defaults to -march=x86-64-v3,
// cc 1.2.65 (the eprfs lock) passes no -march, so ring's C carried AVX2 and a v2 sibling died
// with SIGILL running a drawn epr. The record (genesis/agentic/pool-policy.json declared_targets)
// names, per target triple, the ISA, the C flags that build it and the cargo workspaces bound to
// it; the gate exports those flags as CFLAGS_<triple> / CXXFLAGS_<triple>.
//
// Three checks per declared target that covers the selection, on its own host triple:
//   probe        `<CC or cc> <cflags> -dM -E -x c /dev/null` defines no macro above the baseline;
//   env          no other C-family flags variable is set (cc reads CFLAGS, TARGET_CFLAGS,
//                HOST_CFLAGS and the dashed triple spelling ahead of or beside the declared one),
//                and the declared underscore variable, when set, equals the record's cflags;
//   cargo-config no `.cargo/config[.toml]` [env] from the workspace up to the repository root
//                sets a C-family key.
// CC and CXX choose the compiler and are allowed: the probe runs them.
//
// A finding never refuses a gate. gate-preflight turns findings into a WARN that withholds this
// device's offer of what it built; the CLI prints one line per finding and exits 1 on any.

import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { arch, platform } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = process.env.GATE_ROOT
  ? resolve(process.env.GATE_ROOT)
  : resolve(dirname(fileURLToPath(import.meta.url)), '../..');

/**
 * Preprocessor macros above the x86-64 baseline: v2 (SSE3 through SSE4.2, POPCNT, CX16, LAHF),
 * v3 (AVX, AVX2, BMI, F16C, FMA, LZCNT, MOVBE, XSAVE) and the common extensions beyond. Plain
 * -march=x86-64 on gcc 14 defines none of them (c-baseline.test.mjs REAL).
 */
export const ABOVE_BASELINE = [
  '__SSE3__', '__SSSE3__', '__SSE4_1__', '__SSE4_2__', '__SSE4A__', '__POPCNT__', '__CRC32__', '__LAHF_SAHF__',
  '__GCC_HAVE_SYNC_COMPARE_AND_SWAP_16',
  '__AVX__', '__AVX2__', '__BMI__', '__BMI2__', '__F16C__', '__FMA__', '__FMA4__', '__LZCNT__', '__ABM__', '__MOVBE__',
  '__XSAVE__', '__XSAVEC__', '__XSAVEOPT__', '__XSAVES__',
  '__AES__', '__PCLMUL__', '__VAES__', '__VPCLMULQDQ__', '__GFNI__', '__SHA__', '__ADX__', '__RDRND__', '__RDSEED__',
];
const ABOVE_PATTERN = /^__AVX[A-Z0-9_]*__$/; // every __AVX512*__, __AVXVNNI__ and kin

/** The macro names in `-dM -E` output that lie above the baseline, sorted. */
export function aboveBaseline(macroText) {
  const names = new Set();
  for (const line of String(macroText || '').split('\n')) {
    const m = /^#define\s+(\S+)/.exec(line.trim());
    if (m && (ABOVE_BASELINE.includes(m[1]) || ABOVE_PATTERN.test(m[1]))) names.add(m[1]);
  }
  return [...names].sort();
}

/** A variable the cc crate reads as C, C++ or assembler flags, in any spelling; never CC or CXX. */
export function cFamilyKey(key) {
  return /^(?:(?:TARGET|HOST)_)?(?:C|CXX|CPP|ASM)FLAGS(?:_.+)?$/.test(key) || /^CRATE_CC_NO_DEFAULTS(?:_.+)?$/.test(key);
}

const underscored = triple => triple.replace(/-/g, '_');
/** The one spelling the record allows per language: CFLAGS_x86_64_unknown_linux_gnu, CXXFLAGS_… */
export const declaredVariables = triple => [`CFLAGS_${underscored(triple)}`, `CXXFLAGS_${underscored(triple)}`];

/** pool-policy.json declared_targets, or null when the policy has none or cannot be read. */
export function readDeclaredTargets(root = ROOT, readFile = path => readFileSync(path, 'utf8')) {
  try {
    const record = JSON.parse(readFile(join(root, 'genesis/agentic/pool-policy.json'))).declared_targets;
    return record && typeof record === 'object' ? record : null;
  } catch { return null; }
}

/** The triple rustc builds for on this host, as far as the record can name it. */
export function hostTripleOf({ a = arch(), p = platform() } = {}) {
  const cpu = { x64: 'x86_64', arm64: 'aarch64' }[a];
  return cpu && p === 'linux' ? `${cpu}-unknown-linux-gnu` : null;
}

/**
 * Declared targets to check: those on this host's triple (a native gate builds for its host),
 * optionally one named triple, and, given `workspaces`, only those whose workspaces cover one.
 */
export function applicableTargets(record, { workspaces = null, triple = null, hostTriple = hostTripleOf() } = {}) {
  if (!record || typeof record !== 'object') return [];
  return Object.entries(record)
    .filter(([key, value]) => !key.startsWith('_') && value && typeof value === 'object')
    .filter(([key]) => key === hostTriple && (!triple || key === triple))
    .map(([key, value]) => ({
      triple: key, isa: value.isa, cflags: String(value.cflags ?? ''),
      workspaces: Array.isArray(value.workspaces) ? value.workspaces : [],
    }))
    .filter(target => !workspaces || target.workspaces.some(ws => workspaces.includes(ws)));
}

function probe({ target, env, spawn }) {
  const [cmd, ...wrapped] = String(env.CC || 'cc').trim().split(/\s+/);
  const flags = target.cflags.split(/\s+/).filter(Boolean);
  const shown = [cmd, ...wrapped, ...flags].join(' ');
  let r;
  // gcc invoked by bare name finds its own cc1 through PATH, so the probe inherits this process's
  // environment beneath the one being checked.
  try { r = spawn(cmd, [...wrapped, ...flags, '-dM', '-E', '-x', 'c', '/dev/null'], { encoding: 'utf8', env: { ...process.env, ...env } }); }
  catch (error) { r = { status: null, error }; }
  if (r?.status !== 0) {
    const why = String(r?.error?.message || r?.stderr || `exit ${r?.status}`).trim().split('\n')[0];
    return [{ triple: target.triple, check: 'probe',
      why: `could not probe the C compiler (${shown} -dM -E): ${why} — the C level is unverified`,
      cure: 'install the C compiler the gate builds with, or set CC to it' }];
  }
  const above = aboveBaseline(r.stdout);
  if (!above.length) return [];
  return [{ triple: target.triple, check: 'probe', macros: above,
    why: `${shown} still defines ${above.slice(0, 6).join(' ')}${above.length > 6 ? ` +${above.length - 6}` : ''} — C built at the declared flags targets above ${target.isa}`,
    cure: `use a C compiler whose ${target.cflags || '(no flag)'} is the ${target.isa} baseline (set CC to it), or correct cflags in pool-policy.json declared_targets` }];
}

function envFindings({ target, env }) {
  const declared = declaredVariables(target.triple);
  const findings = [];
  for (const variable of Object.keys(env).filter(cFamilyKey).sort()) {
    const value = String(env[variable]);
    if (declared.includes(variable)) {
      if (value.trim() === target.cflags.trim()) continue;
      findings.push({ triple: target.triple, check: 'env', variable,
        why: `${variable}=${JSON.stringify(value)} differs from the declared ${JSON.stringify(target.cflags)}`,
        cure: `unset ${variable} (the gate exports it) or set it to '${target.cflags}'` });
      continue;
    }
    findings.push({ triple: target.triple, check: 'env', variable,
      why: `${variable}=${JSON.stringify(value)} is set — cc adds it to the C the gate builds, beside the declared ${declared[0]}`,
      cure: `unset ${variable}; the C level is declared in genesis/agentic/pool-policy.json declared_targets` });
  }
  return findings;
}

/** Keys an `[env]` table sets in one cargo config text: `[env]` rows, `[env.KEY]` tables, `env.KEY =`. */
export function cargoEnvKeys(text) {
  const keys = [];
  let table = null;
  const key = raw => raw.trim().replace(/^["']|["']$/g, '');
  for (const raw of String(text).split('\n')) {
    const line = raw.replace(/\s+#.*$/, '').trim();
    if (!line || line.startsWith('#')) continue;
    if (line.startsWith('[[')) { table = '[array]'; continue; }
    const header = /^\[\s*([^\]]+?)\s*\]$/.exec(line);
    if (header) {
      table = header[1];
      const sub = /^env\.(.+)$/.exec(table);
      if (sub) keys.push(key(sub[1]));
      continue;
    }
    const assign = /^((?:"[^"]*"|'[^']*'|[^=])+?)\s*=/.exec(line);
    if (!assign) continue;
    const name = assign[1].trim();
    if (table === 'env') keys.push(key(name));
    else if (table === null) {
      const dotted = /^env\.(.+)$/.exec(name);
      if (dotted) keys.push(key(dotted[1]));
      else if (name === 'env') {
        const inline = /=\s*\{(.*)\}/.exec(line);
        for (const part of (inline?.[1] || '').split(',')) {
          const m = /^\s*((?:"[^"]*"|'[^']*'|[A-Za-z0-9_-]+))\s*=/.exec(part);
          if (m) keys.push(key(m[1]));
        }
      }
    }
  }
  return keys;
}

function cargoConfigFindings({ root, target, readFile, workspaces }) {
  const findings = [];
  const dirs = new Set();
  for (const ws of target.workspaces.filter(ws => !workspaces || workspaces.includes(ws))) {
    for (let d = ws; ; d = dirname(d)) {
      dirs.add(d === '.' ? '' : d);
      if (d === '.' || d === '/' || d === '') break;
    }
  }
  for (const dir of [...dirs].sort()) {
    for (const name of ['config.toml', 'config']) {
      const path = join(root, dir, '.cargo', name);
      let text;
      try { text = readFile(path); } catch { continue; }
      for (const variable of cargoEnvKeys(text).filter(cFamilyKey)) {
        const file = relative(root, path);
        findings.push({ triple: target.triple, check: 'cargo-config', file, variable,
          why: `${file} [env] sets ${variable} — every cargo build under it carries C flags the record does not declare`,
          cure: `remove ${variable} from [env] in ${file}; the C level is declared in genesis/agentic/pool-policy.json declared_targets` });
      }
    }
  }
  return findings;
}

/**
 * Findings for every declared target that covers the gate: `{ triple, check, why, cure, ... }`.
 * `record` is pool-policy.json declared_targets (read from `root` when omitted); `workspaces`
 * narrows to the targets covering a selection's cargo workspaces.
 */
export function checkCBaseline({
  root = ROOT, env = process.env, record, spawn = spawnSync, readFile = path => readFileSync(path, 'utf8'),
  workspaces = null, triple = null, hostTriple = hostTripleOf(),
} = {}) {
  const declared = record === undefined ? readDeclaredTargets(root, readFile) : record;
  return applicableTargets(declared, { workspaces, triple, hostTriple }).flatMap(target => [
    ...probe({ target, env, spawn }),
    ...envFindings({ target, env }),
    ...cargoConfigFindings({ root, target, readFile, workspaces }),
  ]);
}

export function formatFinding(f) {
  return `[c-baseline] ${f.triple} ${f.check}: ${f.why}${f.cure ? ` — cure: ${f.cure}` : ''}`;
}

function main(argv) {
  const i = argv.indexOf('--triple');
  const triple = i >= 0 ? argv[i + 1] || null : null;
  const findings = checkCBaseline({ root: ROOT, triple });
  for (const f of findings) process.stdout.write(`${formatFinding(f)}\n`);
  return findings.length ? 1 : 0;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = main(process.argv.slice(2));
}
