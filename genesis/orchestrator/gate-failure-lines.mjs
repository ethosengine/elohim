// Failure-level lines in a running gate's output, recognized as they print.
//
// A recognized line is a hint for whoever is reading the run; it never decides
// anything. The gate's verdict is its exit status (gate-runner.mjs), and the
// failure class recorded on the cycle row is classifyFailure's reading of the
// signals gathered here (gate-cycle.mjs). Spec:
// genesis/docs/superpowers/specs/2026-10-26-gate-live-failure-channel-design.md

import { createHash } from 'crypto';

// eslint-disable-next-line no-control-regex
const ANSI = /\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07]*\x07/g;

export const MAX_EXCERPT_LINES = 20;
export const MAX_EXCERPT_BYTES = 2048;
export const MAX_FAILURES = 200;

export function stripAnsi(line) {
  return line.replace(ANSI, '');
}

/** Build-invariant normalization, ported from .claude/scripts/ci-harvest.py normalize(). */
export function normalize(line) {
  return stripAnsi(line)
    .replace(/\s+/g, ' ').trim()
    .replace(/\b\d{1,3}(?:\.\d{1,3}){3}\b/g, '#')
    .replace(/\b[a-z0-9]+(?:-[a-z0-9]+)*-[a-z0-9]{5}-[a-z0-9]{5}-[a-z0-9]{5}\b/g, '#')
    .replace(/:\d+(?::\d+)?/g, ':#')
    .replace(/\b[0-9a-f]{7,64}\b/g, '#')
    .replace(/\b\d+(?:\.\d+)?(?:ms|s|m|h)\b/g, '#')
    .replace(/\b20\d{2}-\d{2}-\d{2}[T ][\d:.]+\S*/g, '#')
    .slice(0, 300);
}

export function fingerprint(project, cls, identifier) {
  return createHash('sha256').update(`${project}|${cls}|${identifier}`).digest('hex').slice(0, 12);
}

// Each rule reads one line. `cls` names the failure; `detail` rules add text to a
// failure already raised (cargo prints `test x ... FAILED` long before its panic).
const RULES = [
  { cls: 'test', re: /^test (.+?) \.\.\. FAILED$/, locus: m => ({ test: m[1] }) },
  { cls: 'test', re: /^\s*[×✗]\s+(.+?)(?:\s+\d+(?:\.\d+)?\s*ms)?$/, locus: m => ({ test: m[1] }) },
  { cls: 'test', re: /^\s*✖\s+(.+?)(?:\s+\(\d+(?:\.\d+)?ms\))?$/, locus: m => ({ test: m[1] }), skip: m => /^\d+ problems?\b/.test(m[1]) || /^failing tests:?$/.test(m[1]) },
  { cls: 'test', re: /^not ok \d+ - (.+)$/, locus: m => ({ test: m[1] }) },
  { cls: 'test', re: /^\s*FAIL\s+(\S+)(?:\s+>\s+(.+))?$/, locus: m => (m[2] ? { file: m[1], test: m[2] } : { file: m[1] }) },
  { cls: 'test', re: /^FAILED (\S+::\S+)/, locus: m => ({ test: m[1] }) },
  // vitest and node print the assertion under a failure already raised; it is a signal, not a second failure.
  { cls: 'test', re: /\bAssertionError\b/, signalOnly: true },
  { cls: 'compile', re: /^(.+?)\((\d+),\d+\): error (TS\d+): (.+)$/, locus: m => ({ file: m[1], line: Number(m[2]) }) },
  { cls: 'compile', re: /^(.+?):(\d+):\d+ - error (TS\d+): (.+)$/, locus: m => ({ file: m[1], line: Number(m[2]) }) },
  { cls: 'compile', re: /^error(?:\[E\d+\])?: (?!could not compile|aborting due to|test failed|Recipe `|\d+ previous error)(.+)$/, locus: () => ({}), rust: true },
  { cls: 'format', re: /^Diff in (\S+?)(?::(\d+))?(?: at line (\d+))?:?\s*$/, locus: m => ({ file: m[1], line: Number(m[2] || m[3]) || undefined }) },
  { cls: 'lint', re: /^\s+(\d+):\d+\s+error\s+(.+?)\s{2,}(\S+)$/, locus: (m, ctx) => ({ file: ctx.lastPath, line: Number(m[1]) }) },
  { cls: 'env', re: /No space left on device|Cannot allocate memory|signal: 9, SIGKILL|\(signal: 15, SIGTERM/, locus: () => ({}) },
];

const STEP_FAILED = /^error: Recipe `(.+?)` failed(?: on line \d+)? with exit code (\d+)/;
const CARGO_TEST_BLOCK = /^---- (.+?) stdout ----$/;
const PANIC = /panicked at (?:'.*?', )?([^\s:]+\.rs):(\d+)(?::\d+)?/;
const RUST_LOCUS = /^\s*--> ([^\s:]+):(\d+)(?::\d+)?/;
const RUST_LINT = /^\s*= note: `-D [\w:-]+`|#\[deny\(|^\s*= help: for further information visit https:\/\/rust-lang\.github\.io\/rust-clippy/;
const ESLINT_PATH = /^(\/\S+|[\w./-]+\.(?:[cm]?[jt]sx?|vue|html))$/;
const TEST_OUTPUT = /^running \d+ tests?$|^test result: |^\s*RUN\s+v\d|^ (?:Test Files|Tests)\s+\d|^# (?:tests|pass|fail) \d|^ℹ tests \d/;
const RECIPE_STARTED = /^\s+(?:Compiling|Checking|Finished|Running|Fresh|Doc-tests) |^running \d+ tests?$|^\s*RUN\s+v\d|^> \S/;
const PREFLIGHT = /^gate \S+: (?:pinned Rust toolchain|cargo workspace and targetDir|unsupported run kind)/;

function boundedExcerpt(lines) {
  const kept = [];
  let bytes = 0;
  for (const line of lines.slice(0, MAX_EXCERPT_LINES)) {
    const size = Buffer.byteLength(line, 'utf8') + 1;
    if (bytes + size > MAX_EXCERPT_BYTES) break;
    kept.push(line);
    bytes += size;
  }
  return kept.join('\n');
}

/**
 * A line-at-a-time recognizer. `emit(event)` receives `failure`, `detail` and
 * `step-failed` events in the order the gate printed them; `signals()` is the
 * input classifyFailure reads.
 */
export function createRecognizer({ project, emit, now = () => Date.now() }) {
  const seen = new Set();
  const overflowSeen = new Set();
  const signals = { recipeStarted: false, assertionFailed: false, compilerError: false, testOutput: false };
  const started = now();
  let firstRedMs = null;
  let failures = 0;
  let overflow = 0;
  let lastPath = null;
  let block = null; // { kind: 'cargo-test' | 'rust', lines, event }

  function raise(cls, locus, text, extra = {}) {
    if (cls === 'test') signals.assertionFailed = true;
    if (cls === 'compile' || cls === 'lint' || cls === 'format') signals.compilerError = true;
    const identifier = locus.test || `${locus.file || ''}:${normalize(text)}`;
    const fp = fingerprint(project, cls, identifier);
    if (seen.has(fp)) return null;
    if (failures >= MAX_FAILURES) {
      // Saturate when dedupe fills, rather than count repeated untracked failures.
      if (!overflowSeen.has(fp) && overflowSeen.size < 4 * MAX_FAILURES) {
        overflowSeen.add(fp);
        overflow += 1;
      }
      return null;
    }
    seen.add(fp);
    failures += 1;
    if (firstRedMs === null) firstRedMs = now() - started;
    const failure = { class: cls, fingerprint: fp, locus: clean(locus), nth: failures, excerpt: text };
    const event = { kind: 'failure', project, failure, ...extra };
    emit(event);
    return event;
  }

  function closeBlock() {
    if (!block) return;
    const { kind, lines, ref } = block;
    block = null;
    if (kind === 'cargo-test') {
      const panic = lines.map(l => PANIC.exec(l)).find(Boolean);
      const locus = { test: ref, ...(panic ? { file: panic[1], line: Number(panic[2]) } : {}) };
      const fp = fingerprint(project, 'test', ref);
      if (!seen.has(fp)) raise('test', locus, lines[0] || ref);
      if (!seen.has(fp)) return;
      emit({ kind: 'detail', project, failure: { class: 'test', fingerprint: fp, locus: clean(locus), excerpt: boundedExcerpt(lines) } });
      return;
    }
    // A rust diagnostic: its first line names it, `-->` names the place, and a
    // `-D` note says clippy denied a warning rather than the compiler refusing.
    const cls = lines.some(l => RUST_LINT.test(l)) ? 'lint' : 'compile';
    const at = lines.map(l => RUST_LOCUS.exec(l)).find(Boolean);
    raise(cls, at ? { file: at[1], line: Number(at[2]) } : {}, boundedExcerpt(lines));
  }

  function push(raw) {
    const line = stripAnsi(raw).replace(/\r$/, '');
    if (!signals.recipeStarted && PREFLIGHT.test(line)) signals.preflight = 'refused';
    if (RECIPE_STARTED.test(line)) signals.recipeStarted = true;
    if (TEST_OUTPUT.test(line)) signals.testOutput = true;

    if (block) {
      const ends = block.kind === 'cargo-test'
        ? CARGO_TEST_BLOCK.test(line) || /^(?:failures:|test result: )/.test(line)
        : line.trim() === '' || /^(?:error|warning)(?:\[\w+\])?: /.test(line);
      if (!ends) {
        if (block.lines.length < MAX_EXCERPT_LINES * 2) block.lines.push(line);
        return;
      }
      closeBlock();
    }

    const testBlock = CARGO_TEST_BLOCK.exec(line);
    if (testBlock) { block = { kind: 'cargo-test', ref: testBlock[1], lines: [] }; return; }

    const step = STEP_FAILED.exec(line);
    if (step) {
      signals.recipeStarted = true;
      emit({ kind: 'step-failed', project, step: step[1], exit: Number(step[2]) });
      return;
    }

    if (ESLINT_PATH.test(line.trim())) lastPath = line.trim();
    for (const rule of RULES) {
      const m = rule.re.exec(line);
      if (!m || rule.skip?.(m)) continue;
      if (rule.rust) { block = { kind: 'rust', lines: [line] }; return; }
      if (rule.signalOnly) { signals.assertionFailed = true; return; }
      raise(rule.cls, rule.locus(m, { lastPath }), line);
      return;
    }
  }

  return {
    push,
    end() { closeBlock(); },
    signals: () => ({ ...signals }),
    summary: () => ({ failures, overflow, firstRedSeconds: firstRedMs === null ? null : firstRedMs / 1000 }),
  };
}

function clean(locus) {
  return Object.fromEntries(Object.entries(locus).filter(([, v]) => v !== undefined && v !== null && !Number.isNaN(v)));
}
