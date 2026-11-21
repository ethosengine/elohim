// Recipe-closure digest for root-just gates, in SHADOW mode: it measures and reports, and changes
// no key, no reuse decision and no signed record. gate-runner.mjs takes the shadow's own span out of
// the gate's timed span, so recorded durations do not move either; every subprocess here has a
// SIGKILL deadline and the step has one per-process budget (DEFAULT_BUDGET), after which it stops.
//
// Today a root-just project's closure material carries `recipeFile`, the blob of the WHOLE root
// justfile at the commit (gate-evidence.mjs recipeAt). Fifty-odd recipes share that file, so an
// edit to one recipe re-keys every gate that runs from it. This module computes the part of the
// justfile a given gate's recipe can depend on, from the COMMIT's bytes (`git cat-file`, then
// `just --dump --dump-format json` on a temporary copy; never the working tree), and says per gate
// whether a justfile change since the integration base would have re-keyed it. Projects whose
// `run.kind` is `just` run their own small justfile: they keep the whole-file key, and get no line.
//
// The closure, as built (closureOfDump):
//   G, always: the dump's `settings`, `assignments`, `unexports`, `aliases` and every other
//     top-level dump field except the ones named in NOT_BEHAVIOUR, plus `just --version`.
//   M, the members: the project's recipe (`run.recipe`; `--recipe` runs record no evidence, so they
//     never key), then to a fixed point every recipe a member, or G, can run:
//     (a) its `dependencies`, prior and subsequent alike;
//     (b) every token, in the argument list of a `just` INVOCATION in its shell text, that is a
//         recipe or alias name of this file. An invocation is a simple command holding a `just`
//         word (`just`, `@just`, a path ending in `/just`, `{{just_executable()}}`), anywhere in it,
//         so `exec`, `env A=1`, `time`, `timeout 9`, `nice`, `sudo` and function wrappers all count;
//         its arguments run to the end of the simple command (`;`, `&&`, `||`, `|`, `&`, an
//         unescaped newline, `(`, `)`); `$(…)` and backtick substitutions are parsed as commands of
//         their own, inside double quotes too, and so is the literal script of `sh -c`/`bash -c`.
//         just's flags and their values and `K=V` overrides are skipped; every other token that is
//         a recipe name counts, argument or not, whatever `--justfile` or a `cd` before it names
//         (over-inclusion is the safe direction). No arity is read: every word built at run time
//         in the argument list refuses (below), except a flag or override value that cannot split. Shell text is a linewise body (line by line), a
//         shebang or `[script]` body whose interpreter is a POSIX shell, a backtick and a
//         `shell()` call (run by `set shell`, `sh` by default).
//     (c) a `just` with no recipe token (bare, or only flags) runs `first` (the `[default]` recipe
//         or the first in the file): that recipe joins M and `first` joins G.
//   A word that merely mentions a recipe (`cargo test`, prose, a comment) is not a member.
//   Each member is its dump record minus `doc`; the digest is sha256 over canonical JSON (sorted
//   keys) of G and M keyed by name, so recipe order, comments and blank lines outside bodies, and
//   doc comments change nothing, while any byte of a member's body, parameters, attributes,
//   dependencies, quiet or private marker does.
//
// No closure, whole-file key stands (each with its reason): a top-level `import`/`mod` line (read
// from the text, because `import?` and `mod?` of an absent file dump as if they were not there) or
// a non-empty `modules`; no `just` on PATH, a failed dump, unparseable output; the recipe absent;
// in a member: any word built at run time (`$x`, `"$x"`, `${x}`, `$(…)`, backticks, `{{…}}`, a
// glob) among a just invocation's arguments, since `$args` can split into a second recipe and a
// quoted word's place is only as good as an arity argument (a flag or override value that cannot
// split stays: `--justfile "$f"`); `source`/`.` of any file; a flag that prints or lists the justfile (--list, --dump, --show,
// --summary, --evaluate, --variables, --groups, --choose, --usage, --explain, --json, --fmt,
// --edit, --check); just after `xargs` (its arguments arrive on stdin); `eval`; `sh -c`/`bash -c`
// with a script built at run time; an assignment whose value holds the word `just` (`J=just`,
// `J=$(command -v just)`, `cmd=(just x)`), since `$J x` would then run it; a shebang, `[script]`
// or `set shell` interpreter that is not a POSIX shell (python3, node) whose text holds the word
// `just` at all.
//
// Soundness, in short:
//   1. One `just <recipe>` run reads only: settings and assignments (all assignments are evaluated
//      at startup unless `set lazy`, so even an unused backtick runs; both are in G), exported and
//      unexported names (G), and the recipes it executes: the recipe, its dependencies, and what
//      its shell text runs.
//   2. A recipe's body is shell text (or a script). The only way that text runs another recipe of
//      this file is by executing the just program, and only (i) in its own text, which rule (b)
//      reads, refusing every shape that builds the command word or the recipe name at run time, or
//      (ii) through another program it starts, which is point 5.
//   3. Text outside M and G that can still change behaviour: `first`/`[default]` (only for a bare
//      run: rule c); recipe order (only through `first` and listings, both covered); doc comments
//      and `[group]`/`[doc]` (only through listings, `--explain` and `--usage`, which refuse; the
//      attributes stay in M anyway); duplicate recipes or variables (the dump holds the winner, and
//      the allow-duplicate settings are in G). Settings that change every recipe (`export`,
//      `positional-arguments`, `shell`, `dotenv-*`, `working-directory`, `fallback`, `lazy`,
//      `quiet`, `ignore-comments`, `tempdir`, `unstable`) are all in G.
//   4. No arity is trusted: a static word naming a recipe counts in any position, and a word built
//      at run time refuses wherever it stands, except as the value of a value-taking flag or an
//      override, where it names a file, a directory or a variable. A quoted `{{…}}` there is taken
//      as one word; an interpolated value carrying a double quote would break that (residual).
//   5. Programs a recipe starts can read or run this justfile too (berth_test.py runs `just test
//      mesh`; orchestrator tests read it). Those are reads of the justfile by the gate's code, and
//      the rest of the closure already owns them: the whole-tree read scope, a read set that names
//      `justfile`, a project rooted at `.`, or step inputs that list it. The shadow line therefore
//      says "would not re-key" only when none of those also holds the justfile (alsoKeyed), and
//      names which one does otherwise. Under `on` that remains the condition, and it is an
//      assumption the read audit does not enforce for non-Rust programs (a Python or shell script
//      a recipe starts that runs `just x` is not a named read today).
//   6. Residual, the reason `on` waits for a ruling: a command word taken from a variable this text
//      did not assign (`"$JUSTBIN" x` with JUSTBIN from the caller; gateChildEnv strips only
//      JUST_*), a `${X:-just}` default, and a program that runs just (point 5). `source`/`.` refuses. None is in the root justfile
//      today; the parse cannot exclude them.
//   7. `.env` contents (dotenv-load) and the just binary's behaviour beyond its version are outside
//      this digest, as they are outside the whole-file key; `just --version` is in G.

import { spawnSync } from 'child_process';
import { createHash } from 'crypto';
import { mkdtempSync, rmSync, writeFileSync } from 'fs';
import { tmpdir } from 'os';
import { join, resolve } from 'path';
import picomatch from 'picomatch';
import { resolveBin } from './gate-evidence.mjs';
import { globCovers, readCovers } from './gate-input-audit.mjs';

/** `GATE_RECIPE_KEY=shadow|off`, default shadow. There is deliberately no `on` yet. */
export function recipeKeyMode(env = process.env) {
  return (env.GATE_RECIPE_KEY || '').trim() === 'off' ? 'off' : 'shadow';
}

// Top-level dump fields that are not behaviour of a `just <recipe>` run: the source path, parse
// warnings, the module doc, the group list (derived from attributes), the module path, `first`
// (G gains it only through rule c), and the recipes themselves (M).
const NOT_BEHAVIOUR = new Set(['recipes', 'source', 'warnings', 'doc', 'groups', 'module_path', 'first']);
const SENTINEL = '\u0001';
const LISTS_JUSTFILE = new Set(['--list', '--dump', '--show', '--summary', '--evaluate', '--variables', '--groups',
  '--choose', '--usage', '--explain', '--json', '--fmt', '--edit', '--check']);
const TAKES_VALUE = new Set(['--alias-style', '--ceiling', '--chooser', '--color', '--command-color', '--cygpath',
  '--dotenv-command', '--dotenv-filename', '--dotenv-path', '--dump-format', '--evaluate-format', '--group',
  '--indentation', '--jobs', '--justfile', '--justfile-name', '--list-heading', '--list-prefix', '--shell',
  '--shell-arg', '--tempdir', '--timestamp-format', '--working-directory', '--completions']);
const NO_RECIPE = new Set(['--command', '--clean', '--init', '--man', '--changelog', '--help', '--version', '--completions']);
const POSIX_SHELLS = new Set(['sh', 'bash', 'dash', 'zsh', 'ksh', 'mksh', 'ash', 'busybox']);
const RESERVED = new Set(['if', 'then', 'else', 'elif', 'fi', 'do', 'done', 'while', 'until', 'for', 'in', 'case', 'esac', '!', '{', '}', '[[', ']]']);
// The word `just`, not part of `justfile`, `just_executable`, `just-x` or `adjust`.
const JUST_WORD = /(?<![A-Za-z0-9_])just(?![A-Za-z0-9_.-])/;
const isJustCommand = token => /(^|\/)just$/.test(token.value);

const sha = text => createHash('sha256').update(text).digest('hex');
const basename = path => String(path).split('/').pop();

function canonical(value) {
  if (Array.isArray(value)) return value.map(canonical);
  if (value && typeof value === 'object') {
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, canonical(value[key])]));
  }
  return value;
}
const json = value => JSON.stringify(canonical(value));

function strings(value, out = []) {
  if (typeof value === 'string') out.push(value);
  else if (Array.isArray(value)) for (const item of value) strings(item, out);
  else if (value && typeof value === 'object') for (const item of Object.values(value)) strings(item, out);
  return out;
}

/** The command strings an expression runs: backticks (`evaluate`) and every string in a `shell()` call. */
function expressionCommands(value, out = []) {
  if (Array.isArray(value)) {
    if (value[0] === 'evaluate' && typeof value[1] === 'string') out.push(value[1]);
    else if (value[0] === 'call' && value[1] === 'shell') out.push(...strings(value.slice(2)));
    else for (const item of value) expressionCommands(item, out);
  } else if (value && typeof value === 'object') for (const item of Object.values(value)) expressionCommands(item, out);
  return out;
}

/** A body's lines as the shell sees them: interpolations become a dynamic marker, except just_executable(). */
function renderLines(body) {
  const interpolations = [];
  const lines = (body || []).map(line => line.map(fragment => {
    if (typeof fragment === 'string') return fragment;
    interpolations.push(fragment);
    const [expr] = Array.isArray(fragment) ? fragment : [fragment];
    return Array.isArray(expr) && expr.length === 2 && expr[0] === 'call' && expr[1] === 'just_executable' ? 'just' : SENTINEL;
  }).join(''));
  return { lines, interpolations };
}

/** `$(…)` and backtick substitutions wherever they sit, each as its own command text; the outer text keeps a marker. */
function substitutions(text) {
  const inner = [];
  let out = '';
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (c === '\\') { out += text.slice(i, i + 2); i++; continue; }
    if (c === '$' && text[i + 1] === '(') {
      let depth = 1;
      let j = i + 2;
      while (j < text.length && depth) { if (text[j] === '(') depth++; else if (text[j] === ')') depth--; j++; }
      inner.push(text.slice(i + 2, depth ? j : j - 1));
      out += SENTINEL;
      i = j - 1;
      continue;
    }
    if (c === '`') {
      const j = text.indexOf('`', i + 1);
      const end = j < 0 ? text.length : j;
      inner.push(text.slice(i + 1, end));
      out += SENTINEL;
      i = end;
      continue;
    }
    out += c;
  }
  return { text: out, inner };
}

/** Simple commands of shell text, each a list of `{ value, dynamic }` words, quotes removed. */
function simpleCommands(text) {
  const commands = [];
  let command = [];
  let token = null;
  let quote = null;
  const word = () => (token ??= { value: '', dynamic: false, splits: false });
  const end = () => { if (token) { command.push(token); token = null; } };
  const close = () => { end(); if (command.length) commands.push(command); command = []; };
  for (let i = 0; i < text.length; i++) {
    const c = text[i];
    if (quote === "'") {
      if (c === "'") quote = null;
      else { token.value += c; if (c === SENTINEL) token.dynamic = true; }
      continue;
    }
    if (quote === '"') {
      // "$@" and "${a[@]}" expand to many words even inside double quotes.
      if ((c === '$' && text[i + 1] === '@') || text.startsWith('[@]', i)) token.splits = true;
      if (c === '"') quote = null;
      else if (c === '\\' && i + 1 < text.length) token.value += text[++i];
      else { token.value += c; if (c === '$' || c === SENTINEL) token.dynamic = true; }
      continue;
    }
    if (c === '\\') {
      if (text[i + 1] === '\n') { i++; end(); continue; }
      word().value += text[++i] ?? '';
      continue;
    }
    if (c === "'" || c === '"') { word(); quote = c; continue; }
    if (c === '#' && !token) { while (i + 1 < text.length && text[i + 1] !== '\n') i++; continue; }
    if (/[;&|\n()]/.test(c)) { close(); continue; }
    if (/\s/.test(c)) { end(); continue; }
    word().value += c;
    // Outside quotes an expansion word-splits and a glob expands: neither has a bounded word count.
    if (/[$*?[\]{}~\u0001]/.test(c)) { token.dynamic = true; token.splits = true; }
  }
  close();
  return commands;
}

/** The value extent of each `NAME=` / `NAME+=` at a word start, for the assignment check. */
function assignedValues(text) {
  const values = [];
  const at = /(?<![^\s;&|(])[A-Za-z_][A-Za-z0-9_]*\+?=/g;
  for (const match of text.matchAll(at)) {
    let i = match.index + match[0].length;
    let value = '';
    while (i < text.length && !/[\s;&|]/.test(text[i])) {
      const c = text[i];
      const close = c === '"' ? '"' : c === "'" ? "'" : c === '(' ? ')' : null;
      if (close) {
        let depth = 1;
        let j = i + 1;
        while (j < text.length && depth) {
          if (close === ')' && text[j] === '(') depth++;
          else if (text[j] === close) depth--;
          j++;
        }
        value += text.slice(i, j);
        i = j;
      } else if (c === '$' && text[i + 1] === '(') {
        let depth = 1;
        let j = i + 2;
        while (j < text.length && depth) { if (text[j] === '(') depth++; else if (text[j] === ')') depth--; j++; }
        value += text.slice(i, j);
        i = j;
      } else { value += c; i++; }
    }
    values.push(value);
  }
  return values;
}

/**
 * just's argument grammar over one invocation's words: names, bare, or a refusal. Every word
 * built at run time refuses (a `$x` splits into any number of words, `"$@"` into many, and a
 * `{{…}}` value may carry quotes), except the value of a value-taking flag or a `K=V` override that
 * cannot split: those name a file, a directory or a variable, never a recipe position. No arity is
 * read, so a static word that names a recipe counts wherever it stands.
 */
function justArguments(words, ctx, snippet) {
  let sawRecipe = false;
  const value = word => (word.splits ? null : word);
  for (let i = 0; i < words.length; i++) {
    const word = words[i];
    const text = word.value;
    if (!word.dynamic && /^\d*[<>]/.test(text)) { if (/^\d*[<>]+&?$/.test(text)) i++; continue; }
    if (!word.dynamic && text === '--') continue;
    if (/^-./.test(text) && !/[$\u0001]/.test(text.split('=')[0]) && !word.splits) {
      const [flag] = text.split('=');
      const short = /^-[A-Za-z]+$/.test(text) ? text.slice(1) : '';
      if (LISTS_JUSTFILE.has(flag) || /[lse]/.test(short)) return `a member prints the justfile (${snippet})`;
      if (NO_RECIPE.has(flag) || short.includes('c')) return null;
      const takes = flag === '--set' ? (text.includes('=') ? 1 : 2)
        : (!text.includes('=') && (TAKES_VALUE.has(flag) || /[fdEF]$/.test(short)) ? 1 : 0);
      for (let k = 1; k <= takes; k++) {
        if (words[i + k] && !value(words[i + k])) return `an unquoted expansion where just reads a flag value (${snippet})`;
      }
      i += takes;
      continue;
    }
    if (!sawRecipe && /^[A-Za-z_][A-Za-z0-9_-]*=/.test(text)) {
      if (!value(word)) return `an unquoted expansion in a just override (${snippet})`;
      continue;
    }
    if (word.dynamic) return `a word built at run time in a just invocation, which may be or split into a recipe name (${snippet})`;
    const name = ctx.recipes[text] ? text : ctx.aliases[text]?.target;
    if (name && ctx.recipes[name]) ctx.names.add(name);
    sawRecipe = true;
  }
  if (!sawRecipe) ctx.bare = true;
  return null;
}

/** Shell text: every just invocation in it, its substitutions and its literal `sh -c` scripts. */
function shellText(text, ctx, depth = 0) {
  if (depth > 8) return 'shell text nested deeper than this parse follows';
  for (const value of assignedValues(text)) {
    // `out="$(just x)"` stores just's output, and its invocation is parsed below; a lookup
    // (`command -v just`, `which just`) or a literal stores the command itself.
    const stored = value.replace(/\$\(([^()]*)\)|`([^`]*)`/g, (_m, a, b) => {
      const inner = a ?? b;
      return /^\s*(command|which|type|whereis|echo|printf|realpath|readlink)\b/.test(inner) ? inner : '';
    });
    if (JUST_WORD.test(stored)) return `a variable holds the just command (${value.slice(0, 50)})`;
  }
  const { text: flat, inner } = substitutions(text);
  for (const piece of inner) {
    const refused = shellText(piece, ctx, depth + 1);
    if (refused) return refused;
  }
  for (const words of simpleCommands(flat)) {
    let first = 0;
    while (first < words.length && !words[first].dynamic && (RESERVED.has(words[first].value) || /^[A-Za-z_][A-Za-z0-9_]*\+?=/.test(words[first].value))) first++;
    const head = words[first];
    if (!head) continue;
    if (!head.dynamic && head.value === 'eval') return `a member runs eval (${words.map(w => w.value).join(' ').slice(0, 50)})`;
    // A sourced file's functions run in this shell and can start just under any name.
    if (head.value === 'source' || head.value === '.') return `a member sources a file (${words.map(w => w.value).join(' ').slice(0, 50)})`;
    if (!head.dynamic && POSIX_SHELLS.has(basename(head.value))) {
      let k = first + 1;
      while (k < words.length && /^[-+]/.test(words[k].value) && !/^-[A-Za-z]*c[A-Za-z]*$/.test(words[k].value)) k++;
      if (k < words.length && /^-[A-Za-z]*c[A-Za-z]*$/.test(words[k].value)) {
        const script = words[k + 1];
        if (!script) continue;
        if (script.dynamic) return `a shell runs a command string built at run time (${head.value} -c ${script.value.replace(/\u0001/g, '{{…}}').slice(0, 40)})`;
        const refused = shellText(script.value, ctx, depth + 1);
        if (refused) return refused;
        continue;
      }
    }
    const at = words.findIndex((word, index) => index >= first && isJustCommand(word));
    if (at < 0) continue;
    const snippet = words.slice(at).map(w => w.value).join(' ').slice(0, 60);
    if (words.slice(first, at).some(word => word.value === 'xargs')) return `xargs supplies just's arguments at run time (${snippet})`;
    const refused = justArguments(words.slice(at + 1), ctx, snippet);
    if (refused) return refused;
  }
  return null;
}

/** The interpreter a recipe body runs under, by basename. */
function interpreterOf(recipe, lines, settings) {
  const script = (recipe.attributes || []).find(attr => attr && typeof attr === 'object' && 'script' in attr);
  if (script) return basename(script.script?.command || settings?.script_interpreter?.command || 'sh');
  if (recipe.shebang) {
    const words = String(lines[0] || '').replace(/^#!/, '').trim().split(/\s+/);
    let i = 0;
    if (basename(words[0]) === 'env') {
      i = 1;
      while (i < words.length && (/^-/.test(words[i]) || /^[A-Za-z_]\w*=/.test(words[i]))) i++;
    }
    return basename(words[i] || 'sh');
  }
  return basename(settings?.shell?.command || 'sh');
}

/**
 * Every recipe one member's text can run: `{ names, bare }` or a refusal string. A body under a
 * non-POSIX interpreter is not parsed: it refuses when it holds the word `just` at all.
 */
function memberInvocations(recipe, ctx, settings) {
  const { lines, interpolations } = renderLines(recipe.body);
  const commands = [...expressionCommands({ ...recipe, body: undefined }), ...expressionCommands(interpolations)];
  const expressionShell = basename(settings?.shell?.command || 'sh');
  for (const command of commands) {
    if (!POSIX_SHELLS.has(expressionShell)) {
      if (JUST_WORD.test(command)) return `a ${expressionShell} expression holds the word just`;
      continue;
    }
    const refused = shellText(command, ctx);
    if (refused) return refused;
  }
  const interpreter = interpreterOf(recipe, lines, settings);
  const linewise = !recipe.shebang;
  const text = lines.join('\n');
  if (!POSIX_SHELLS.has(interpreter)) {
    return JUST_WORD.test(text) ? `a ${interpreter} recipe holds the word just (${recipe.name})` : null;
  }
  if (!linewise) return shellText(text, ctx);
  // A linewise recipe runs each logical line (backslash continuations joined) in its own shell,
  // after just strips the `@` and `-` prefixes.
  for (const line of text.replace(/\\\n/g, ' ').split('\n')) {
    const refused = shellText(line.replace(/^[@-]+/, ''), ctx);
    if (refused) return refused;
  }
  return null;
}

/** Whole-file key only: a top-level import or module line, whatever the dump reports. */
const IMPORT_OR_MOD = /^(?:import|mod)\??[ \t]/m;

function mentionPattern(names) {
  const sorted = [...names].sort((a, b) => b.length - a.length).map(name => name.replace(/[.*+?^${}()|[\]\\-]/g, '\\$&'));
  return sorted.length ? new RegExp(`(?<![A-Za-z0-9_-])(?:${sorted.join('|')})(?![A-Za-z0-9_-])`, 'g') : null;
}

/**
 * The closure of `recipe` over a parsed dump: `{ digest, members, global }` (each a sha256 per
 * part, for naming what changed) or `{ why }`. `mentions: true` is for analysis only (the shadow
 * line never passes it): it adds the earlier, broader rule, every whole-word mention of a recipe
 * name in any string a member holds.
 */
export function closureOfDump(dump, text, recipe, { justVersion = null, mentions = false } = {}) {
  if (IMPORT_OR_MOD.test(text) || Object.keys(dump.modules || {}).length > 0) return { why: 'the justfile has an import or module' };
  const recipes = dump.recipes || {};
  const aliases = dump.aliases || {};
  if (!recipes[recipe]) return { why: `recipe ${recipe} not found` };
  const settings = dump.settings || {};
  const global = Object.fromEntries(Object.entries(dump).filter(([key]) => !NOT_BEHAVIOUR.has(key)));
  global.just = justVersion;
  const pattern = mentions ? mentionPattern([...Object.keys(recipes), ...Object.keys(aliases)]) : null;
  const reached = new Set();
  const todo = [];
  const add = name => {
    if (name && recipes[name] && !reached.has(name)) { reached.add(name); todo.push(name); }
  };
  add(recipe);
  const ctx = { recipes, aliases, names: new Set(), bare: false };
  // Assignments run at startup for every recipe: their backticks and shell() calls seed M.
  const expressionShell = basename(settings.shell?.command || 'sh');
  for (const command of expressionCommands({ assignments: global.assignments })) {
    const refused = POSIX_SHELLS.has(expressionShell)
      ? shellText(command, ctx)
      : (JUST_WORD.test(command) ? `a ${expressionShell} assignment holds the word just` : null);
    if (refused) return { why: refused };
  }
  while (true) {
    for (const name of ctx.names) add(name);
    if (ctx.bare) add(dump.first);
    const next = todo.pop();
    if (next === undefined) break;
    const member = recipes[next];
    const refused = memberInvocations(member, ctx, settings);
    if (refused) return { why: refused };
    for (const dep of member.dependencies || []) add(dep.recipe);
    if (pattern) {
      const { doc: _doc, ...rest } = member;
      for (const piece of strings(rest)) for (const m of piece.matchAll(pattern)) add(recipes[m[0]] ? m[0] : aliases[m[0]]?.target);
    }
  }
  if (ctx.bare) global.first = dump.first ?? null;
  const members = Object.fromEntries([...reached].sort().map(name => {
    const { doc: _doc, ...record } = recipes[name];
    return [name, sha(json(record))];
  }));
  const parts = Object.fromEntries(Object.keys(global).sort().map(key => [key, sha(json(global[key]))]));
  return { digest: sha(json({ v: 1, global: parts, members })), members, global: parts };
}

const sanitizedEnv = env => Object.fromEntries(Object.entries(env).filter(([key]) => !key.startsWith('JUST_')));

// Every subprocess this module starts has a deadline it cannot ignore (SIGKILL at the timeout), and
// the shadow step as a whole has one budget per process: once spent, it says so once and stops.
const DEFAULT_BUDGET = Object.freeze({ totalMs: 5000, callMs: 2000 });
let budget = { ...DEFAULT_BUDGET, spentMs: 0, exhausted: false, announced: false };
const versions = new Map();
const models = new Map();
const reads = new Map();
const closures = new Map();
let noBaseAnnounced = false;

/** A spawn with an enforced deadline: `timeoutMs`, killed with SIGKILL, never longer. */
function bounded(spawn, timeoutMs) {
  return (bin, args, opts = {}) => spawn(bin, args, { ...opts, timeout: Math.max(1, Math.floor(timeoutMs())), killSignal: 'SIGKILL' });
}

/** The shadow's spawn: each call is bounded by what is left of the process budget, and its time is counted. */
function budgeted(spawn) {
  return (bin, args, opts = {}) => {
    const left = budget.totalMs - budget.spentMs;
    if (left <= 0) {
      budget.exhausted = true;
      return { status: null, stdout: '', stderr: '', error: { code: 'EBUDGET' } };
    }
    const started = Date.now();
    try {
      return bounded(spawn, () => Math.min(budget.callMs, left))(bin, args, opts);
    } finally {
      budget.spentMs += Date.now() - started;
      if (budget.spentMs >= budget.totalMs) budget.exhausted = true;
    }
  };
}

function justVersionOf(bin, env, run) {
  if (!versions.has(bin)) {
    const r = run(bin, ['--version'], { encoding: 'utf8', env: sanitizedEnv(env) });
    versions.set(bin, r?.status === 0 ? String(r.stdout).trim() : null);
  }
  return versions.get(bin);
}

/** The parsed dump of `text`, once per just binary and text per process: `{ dump, version }` or `{ why }`. */
function modelOf(text, { env, run, justBin }) {
  const bin = justBin || resolveBin('just', [], env);
  if (!bin) return { why: 'just not on PATH' };
  const key = `${bin}\0${sha(text)}`;
  if (models.has(key)) return models.get(key);
  let model;
  let dir = null;
  try {
    dir = mkdtempSync(join(tmpdir(), 'gate-recipe-dump-'));
    const path = join(dir, 'justfile');
    writeFileSync(path, text);
    const r = run(bin, ['--justfile', path, '--working-directory', dir, '--dump', '--dump-format', 'json'], {
      cwd: dir, encoding: 'utf8', env: sanitizedEnv(env), maxBuffer: 64 * 1024 * 1024,
    });
    if (r?.error) model = { why: r.error.code === 'ETIMEDOUT' ? 'just --dump timed out and was killed' : `just did not run (${r.error.code || r.error.message})` };
    else if (r?.status !== 0) model = { why: `dump failed: ${String(r?.stderr || '').trim().split('\n')[0] || `exit ${r?.status}${r?.signal ? ` ${r.signal}` : ''}`}` };
    else {
      try { model = { dump: JSON.parse(r.stdout), version: justVersionOf(bin, env, run) }; }
      catch { model = { why: 'dump is not JSON' }; }
    }
  } catch (error) {
    model = { why: String(error?.message || error).split('\n')[0] };
  } finally {
    if (dir) rmSync(dir, { recursive: true, force: true });
  }
  // A budget refusal is not a fact about these bytes, so it is not remembered.
  if (model.why !== 'just did not run (EBUDGET)') models.set(key, model);
  return model;
}

/** The closure digest of `recipe` in justfile `text`. Never throws; its subprocesses are bounded. */
export function recipeClosureFromText(text, recipe, { env = process.env, spawn = spawnSync, mentions = false, justBin = null, run = null } = {}) {
  try {
    if (IMPORT_OR_MOD.test(text)) return { why: 'the justfile has an import or module' };
    const model = modelOf(text, { env, justBin, run: run || bounded(spawn, () => DEFAULT_BUDGET.callMs) });
    if (model.why) return { why: model.why };
    return closureOfDump(model.dump, text, recipe, { justVersion: model.version, mentions });
  } catch (error) {
    return { why: String(error?.message || error).split('\n')[0] };
  }
}

function gitRead(root, args, run) {
  try {
    const r = run('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
    return r?.status === 0 ? r.stdout : null;
  } catch {
    return null;
  }
}

/** One git read per repository and arguments per process. */
function memoRead(root, args, run) {
  const key = `${resolve(root)}\0${args.join('\0')}`;
  if (reads.has(key)) return reads.get(key);
  const value = gitRead(root, args, run);
  if (!budget.exhausted) reads.set(key, value);
  return value;
}

const blobAt = (root, commit, run) => memoRead(root, ['rev-parse', '--verify', '--quiet', `${commit}:justfile`], run)?.trim() || null;

/** The closure digest of `recipe` in the root justfile at `commit`, from the commit's bytes. */
export function recipeClosureAt(root, commit, recipe, { env = process.env, spawn = spawnSync, justBin = null, run = null } = {}) {
  const runner = run || bounded(spawn, () => DEFAULT_BUDGET.callMs);
  const blob = blobAt(root, commit, runner);
  if (!blob) return { why: `no justfile at ${String(commit).slice(0, 9)}` };
  const key = `${resolve(root)}\0${blob}\0${recipe}`;
  if (closures.has(key)) return closures.get(key);
  const text = memoRead(root, ['cat-file', 'blob', blob], runner);
  if (text === null) return { why: `justfile ${blob.slice(0, 9)} unreadable` };
  const result = recipeClosureFromText(text, recipe, { env, justBin, run: runner });
  if (!budget.exhausted) closures.set(key, result);
  return result;
}

/** Reset the per-process caches and the shadow budget (tests; a gate process runs once). */
export function clearRecipeKeyCache({ totalMs = DEFAULT_BUDGET.totalMs, callMs = DEFAULT_BUDGET.callMs } = {}) {
  models.clear();
  versions.clear();
  reads.clear();
  closures.clear();
  noBaseAnnounced = false;
  budget = { totalMs, callMs, spentMs: 0, exhausted: false, announced: false };
}

/**
 * Which other piece of the closure material holds the justfile: `{ holder }`, `{ unknown }` when
 * that cannot be determined, or `{}` when none does. The manifest is read at the commit: brit
 * fingerprints the WORKING manifest, but a dirty manifest makes evidence neither reusable nor
 * recorded (dirtyPaths), and the caller makes no claim unless the tree is clean.
 */
function otherHolder(project, material, { root, commit, run }) {
  if (project.dir === '.') return { holder: 'the root directory tree' };
  if (!material?.reads?.scope) return { unknown: 'the material names no read scope' };
  if (material.reads.scope === 'tree') return { holder: 'the whole-tree read scope' };
  const read = (material.reads.oids || []).find(row => readCovers(row.path, 'justfile'));
  if (read) return { holder: `the read set (${read.path})` };
  if (!project.steps?.length) return {};
  const text = memoRead(root, ['show', `${commit}:${project.manifestPath}`], run);
  if (text === null) return { unknown: `manifest ${project.manifestPath} unreadable at ${String(commit).slice(0, 9)}` };
  let manifest;
  try { manifest = JSON.parse(text); } catch { return { unknown: `manifest ${project.manifestPath} is not JSON` }; }
  for (const step of project.steps) {
    const inputs = manifest?.steps?.[step]?.inputs;
    if (!inputs || !Array.isArray(inputs.sources || [])) return { unknown: `step ${step} has no readable inputs` };
    const hit = [...(inputs.sources || []), ...(inputs.buildProcess || []).map(ref => String(ref).split('@')[0])]
      .find(glob => globCovers(glob, 'justfile') || picomatch(glob, { dot: true })('justfile'));
    if (hit) return { holder: `step ${step}'s inputs (${hit})` };
  }
  return {};
}

function changedMembers(before, after) {
  const names = [];
  for (const part of new Set([...Object.keys(before.global), ...Object.keys(after.global)])) {
    if (before.global[part] !== after.global[part]) names.push(part);
  }
  for (const name of new Set([...Object.keys(before.members), ...Object.keys(after.members)])) {
    if (!(name in before.members)) names.push(`+${name}`);
    else if (!(name in after.members)) names.push(`-${name}`);
    else if (before.members[name] !== after.members[name]) names.push(name);
  }
  return names.length > 3 ? `${names.slice(0, 3).join(', ')} and ${names.length - 3} more` : names.join(', ');
}

function integrationBase(root, commit, run) {
  for (const ref of ['origin/dev', 'dev']) {
    const base = memoRead(root, ['merge-base', commit, ref], run)?.trim();
    if (base) return base;
  }
  return null;
}

/**
 * The shadow line for one gate, or null when there is nothing to say: off, not a root-just
 * project, no key computed, or the justfile unchanged since the integration base. `material` is
 * the closure material gateEvidence hashed and `dirty` its working-tree verdict (both read only);
 * `base` overrides the comparison commit, otherwise the merge base of `commit` with origin/dev
 * (else dev). The base and both justfile blob ids are read once per process. Never throws, and
 * never runs past its budget: every failure is part of the line.
 */
export function recipeKeyShadow(project, { root, commit, material, dirty = false, env = process.env, spawn = spawnSync, base = null, justBin = null }) {
  try {
    if (recipeKeyMode(env) === 'off' || project.run?.kind !== 'root-just' || !material) return null;
    const stopped = () => {
      if (budget.announced) return null;
      budget.announced = true;
      return `stopped: the shadow's ${budget.totalMs / 1000} s budget for this run is spent; no further recipe-key lines`;
    };
    if (budget.exhausted) return stopped();
    const run = budgeted(spawn);
    const since = base || integrationBase(root, commit, run);
    if (budget.exhausted) return stopped();
    if (!since) {
      if (noBaseAnnounced) return null;
      noBaseAnnounced = true;
      return 'neither origin/dev nor dev resolves here, so there is no base to compare the justfile with this run';
    }
    const blobNow = blobAt(root, commit, run);
    const blobBefore = blobAt(root, since, run);
    if (budget.exhausted) return stopped();
    if (!blobNow || !blobBefore || blobNow === blobBefore) return null;
    const head = `justfile changed since ${String(since).slice(0, 9)}`;
    const after = recipeClosureAt(root, commit, project.run.recipe, { env, justBin, run });
    const before = recipeClosureAt(root, since, project.run.recipe, { env, justBin, run });
    if (budget.exhausted) return stopped();
    const missing = after.why || before.why;
    if (missing) return `${head}; no recipe closure (${missing}) — the whole-file key stands`;
    if (after.digest !== before.digest) return `${head}, recipe closure changed (${changedMembers(before, after)})`;
    if (dirty !== false) return `${head}, recipe closure unchanged; the working tree is dirty or unknown, so no claim about the key is made`;
    const parsed = typeof material === 'string' ? JSON.parse(material) : material;
    const other = otherHolder(project, parsed, { root, commit, run });
    if (budget.exhausted) return stopped();
    if (other.unknown) return `${head}, recipe closure unchanged; whether another piece of the key holds the justfile cannot be told (${other.unknown})`;
    if (other.holder) return `${head}, recipe closure unchanged, but ${other.holder} also holds the justfile — the justfile still moves this key`;
    return `${head}, recipe closure unchanged — the recipe file would not have moved this key`;
  } catch (error) {
    return `recipe closure unavailable (${String(error?.message || error).split('\n')[0]})`;
  }
}
