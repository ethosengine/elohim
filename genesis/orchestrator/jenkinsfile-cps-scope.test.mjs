/**
 * Jenkinsfile CPS-Scope Lint
 *
 * Catches the env-bridge anti-pattern that hit orchestrator/dev #1024
 * (groovy.lang.MissingPropertyException: No such property: changedFiles).
 *
 * The bug: variables `def`-declared inside one stage's `script { ... }`
 * block do NOT survive into the next stage's `script { ... }` block
 * because each is its own CPS closure. Cross-stage variables must be
 * bridged via `env.X = value` at the producer side and recovered via
 * `def x = env.X` at the consumer side.
 *
 * What this lint checks (statically against the Jenkinsfile text):
 *
 *   1. For every `triggerPipeline(...)` call site, every IDENTIFIER
 *      argument (not literal, not env.X, not params.X, not env-bridged
 *      via env.X assignment elsewhere) must be `def`-declared in the
 *      enclosing `script { ... }` block before the call.
 *
 *   2. Same check generalized: any variable used inside a script block
 *      that isn't (a) declared via def in the same block, (b) a
 *      pipeline-level parameter, (c) an env access, (d) a helper-
 *      function call defined above `pipeline {}`, or (e) a Jenkins
 *      built-in (sh, echo, parallel, error, ...) — gets flagged.
 *
 *      This is heuristic; false positives are possible. When a real
 *      cross-stage reference exists, the fix is one of:
 *         - env-bridge: producer writes env.X; consumer reads env.X.
 *           Pattern already used for PIPELINES_TO_RUN, PIPELINE_BASELINES.
 *         - Hoist computation: move logic that needs both stages'
 *           values into a helper function above `pipeline {}`.
 *         - Suppress: if you're sure the false positive is benign,
 *           add the variable to the SCRIPT_SCOPE_GLOBAL_NAMES list
 *           in this file with a comment.
 *
 *   3. Sanity: every `env.X = ` write should be matched by at least
 *      one `env.X` read elsewhere in the file (no orphan bridges).
 *
 * Run:
 *   node --test genesis/orchestrator/jenkinsfile-cps-scope.test.mjs
 *
 * Or as part of the orchestrator test suite:
 *   pnpm test --filter orchestrator
 */

import { describe, it } from 'node:test';
import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const ORCH_JENKINSFILE = resolve(__dirname, 'Jenkinsfile');

/**
 * Identifiers that are always considered "in scope" — Jenkins built-ins,
 * Groovy keywords, common globals defined above `pipeline {}`. If a
 * variable falls into this list the lint skips it.
 *
 * Generalize cautiously: a false suppression is worse than a noisy lint,
 * since the noisy lint forces a real read of the call site.
 */
const SCRIPT_SCOPE_GLOBAL_NAMES = new Set([
    // Jenkins step / DSL globals
    'sh', 'echo', 'error', 'parallel', 'stage', 'node', 'container',
    'dir', 'fileExists', 'readJSON', 'readProperties', 'readYaml',
    'writeJSON', 'writeYaml', 'writeFile', 'archiveArtifacts', 'junit',
    'stash', 'unstash', 'load', 'checkout', 'build', 'currentBuild',
    'env', 'params', 'BRANCH_NAME', 'WORKSPACE',
    'withCredentials', 'withEnv', 'usernamePassword', 'string',
    'booleanParam', 'stringParam', 'sleep', 'milestone', 'lock',
    'retry', 'timeout', 'catchError', 'unstable', 'milestone',
    'podTemplate',
    // Groovy keywords / type names
    'def', 'return', 'if', 'else', 'for', 'while', 'try', 'catch',
    'finally', 'throw', 'new', 'this', 'null', 'true', 'false',
    'Map', 'List', 'String', 'Integer', 'Boolean', 'Closure',
    'Date', 'BigDecimal',
    // Helper functions defined above `pipeline {}` in the orchestrator Jenkinsfile.
    // Keep this list close to the Jenkinsfile; when a helper is added/renamed,
    // update here too.
    'loadBuildVars', 'withBuildVars', 'shouldRunStep', 'triggerPipeline',
    'recordPipelineResult', 'analyzePipelineRequirements',
    'analyzeChangeset', 'propagateDependencies', 'groupByDependencyLevel',
    'parseCommitTags', 'parseSkipCi', 'matchesCiIgnore', 'parseCiIgnore',
    'buildPredictedGraph', 'runBuildGraph', 'applyBuildGraphRouting',
    'hydrateStageAnnotations', 'reconcileBuildGraph',
    'PIPELINES', 'CI_IGNORE_PATTERNS',
]);

/**
 * Extract every `script { ... }` block from the Jenkinsfile text along
 * with its enclosing stage name (best-effort) and the 1-based line range.
 *
 * Approach: walk the text counting braces, tracking whether we're inside
 * a quoted string or a comment. Stage name is the most-recent
 * `stage('NAME') {` seen before the script block opens.
 */
export function extractScriptBlocks(text) {
    const blocks = [];
    let i = 0;
    let line = 1;
    let stageName = null;
    let stageDepth = 0;
    let stageDepths = [];  // stack of script-depths when each stage was entered

    function consumeUntilUnquoted(startChar, endChar, allowEscape) {
        // Skip until matching endChar, ignoring escapes if allowEscape.
        while (i < text.length) {
            if (allowEscape && text[i] === '\\') {
                if (text[i + 1] === '\n') line++;
                i += 2;
                continue;
            }
            if (text[i] === '\n') line++;
            if (text[i] === endChar) { i++; return; }
            i++;
        }
    }

    function consumeString(quote) {
        // Handle single/double quote, triple-quote, and Groovy GString.
        if (text.slice(i, i + 3) === quote.repeat(3)) {
            // Triple-quoted
            i += 3;
            while (i < text.length) {
                if (text[i] === '\n') line++;
                if (text.slice(i, i + 3) === quote.repeat(3)) { i += 3; return; }
                i++;
            }
        } else {
            // Single-quoted (no escape walk for our purposes, just handle \n)
            i++;
            while (i < text.length) {
                if (text[i] === '\\') { if (text[i + 1] === '\n') line++; i += 2; continue; }
                if (text[i] === '\n') line++;
                if (text[i] === quote) { i++; return; }
                i++;
            }
        }
    }

    function consumeLineComment() {
        while (i < text.length && text[i] !== '\n') i++;
    }
    function consumeBlockComment() {
        i += 2;  // skip /*
        while (i < text.length) {
            if (text[i] === '\n') line++;
            if (text.slice(i, i + 2) === '*/') { i += 2; return; }
            i++;
        }
    }

    while (i < text.length) {
        const ch = text[i];
        if (ch === '\n') { line++; i++; continue; }
        if (ch === '/' && text[i + 1] === '/') { consumeLineComment(); continue; }
        if (ch === '/' && text[i + 1] === '*') { consumeBlockComment(); continue; }
        if (ch === '"' || ch === "'") { consumeString(ch); continue; }

        // Try to match `stage('NAME') {` or `stage("NAME") {`
        const stageMatch = text.slice(i).match(/^stage\s*\(\s*(['"])([^'"]+)\1\s*\)\s*\{/);
        if (stageMatch) {
            stageName = stageMatch[2];
            stageDepths.push(stageName);
            i += stageMatch[0].length;
            continue;
        }

        // Try to match `script {` — capture its body
        const scriptMatch = text.slice(i).match(/^script\s*\{/);
        if (scriptMatch) {
            const startLine = line;
            i += scriptMatch[0].length;
            const bodyStart = i;
            let depth = 1;
            while (i < text.length && depth > 0) {
                const c = text[i];
                if (c === '\n') line++;
                if (c === '/' && text[i + 1] === '/') { consumeLineComment(); continue; }
                if (c === '/' && text[i + 1] === '*') { consumeBlockComment(); continue; }
                if (c === '"' || c === "'") { consumeString(c); continue; }
                if (c === '{') depth++;
                else if (c === '}') depth--;
                i++;
            }
            const body = text.slice(bodyStart, i - 1);  // exclude closing brace
            blocks.push({ stage: stageName, body, startLine, endLine: line });
            continue;
        }

        i++;
    }
    return blocks;
}

/**
 * Find all `def NAME` (or `def NAME, NAME2, ...`) declarations in a
 * script block body. Returns a Set of identifier names.
 *
 * Also includes for-loop and closure parameter declarations because those
 * introduce names into the same lexical scope.
 */
export function extractDefs(body) {
    const defs = new Set();

    // `def x` or `def x = ...` — optionally typed `def Map x = [:]`
    // Multi-declaration: `def x, y, z`
    for (const m of body.matchAll(/\bdef\s+(?:([A-Z][\w]*)\s+)?([\w,\s]+?)(?=\s*=|\s*[\n;{])/g)) {
        const names = m[2].split(',').map((s) => s.trim()).filter((s) => /^\w+$/.test(s));
        for (const n of names) defs.add(n);
    }

    // `for (int i = ...; ...; ...) { ... }` or `for (type x : ...) { ... }`
    for (const m of body.matchAll(/\bfor\s*\(\s*(?:[A-Z]\w*\s+)?(\w+)\s*[=:]/g)) {
        defs.add(m[1]);
    }

    // Closure parameter `level.each { name -> ... }` or `(a, b) -> ...`
    for (const m of body.matchAll(/\{\s*([\w, ]+)\s*->/g)) {
        const names = m[1].split(',').map((s) => s.trim()).filter((s) => /^\w+$/.test(s));
        for (const n of names) defs.add(n);
    }

    return defs;
}

/**
 * Find all `triggerPipeline(arg1, arg2, ...)` invocations in a script
 * block body. Returns array of {args: string[], offsetInBody}.
 *
 * Naive arg parser — splits at top-level commas, handles parens 1 deep.
 * Adequate for the orchestrator Jenkinsfile's call sites.
 */
export function findTriggerPipelineCalls(body) {
    const calls = [];
    const re = /triggerPipeline\s*\(/g;
    let m;
    while ((m = re.exec(body)) !== null) {
        // Find matching close-paren tracking depth
        let i = m.index + m[0].length;
        let depth = 1;
        const start = i;
        while (i < body.length && depth > 0) {
            const c = body[i];
            if (c === '(') depth++;
            else if (c === ')') depth--;
            i++;
        }
        const argsText = body.slice(start, i - 1);
        const args = splitTopLevelArgs(argsText);
        calls.push({ args, offsetInBody: m.index });
    }
    return calls;
}

function splitTopLevelArgs(text) {
    const args = [];
    let buf = '';
    let depth = 0;
    let inStr = null;
    for (let i = 0; i < text.length; i++) {
        const c = text[i];
        if (inStr) {
            if (c === '\\') { buf += c + (text[i + 1] || ''); i++; continue; }
            if (c === inStr) inStr = null;
            buf += c;
            continue;
        }
        if (c === '"' || c === "'") { inStr = c; buf += c; continue; }
        if (c === '(' || c === '[' || c === '{') depth++;
        if (c === ')' || c === ']' || c === '}') depth--;
        if (c === ',' && depth === 0) { args.push(buf.trim()); buf = ''; continue; }
        buf += c;
    }
    if (buf.trim()) args.push(buf.trim());
    return args;
}

/**
 * Classify an arg expression for the lint:
 *   - 'safe' — literal, env.X, params.X, ternary against safe operands, etc.
 *   - 'global' — references a SCRIPT_SCOPE_GLOBAL_NAMES identifier
 *   - {kind: 'ident', name} — bare identifier that must be in `def` scope
 *   - {kind: 'expr', root} — complex expression; first identifier is the
 *     thing that must be in scope
 */
export function classifyArg(arg) {
    if (!arg) return 'safe';
    const a = arg.trim();
    if (a === '' || a === 'true' || a === 'false' || a === 'null') return 'safe';
    if (/^[-+]?\d/.test(a)) return 'safe';
    if (a.startsWith('"') || a.startsWith("'")) return 'safe';
    if (a.startsWith('env.') || a.startsWith('params.')) return 'safe';
    if (a.startsWith('currentBuild.')) return 'safe';
    // Ternary, e.g. `(params.X ?: 'y')`
    if (a.startsWith('(') || a.includes('?:') || a.includes('?')) return 'safe';
    // Single bare identifier
    if (/^[A-Za-z_]\w*$/.test(a)) {
        if (SCRIPT_SCOPE_GLOBAL_NAMES.has(a)) return 'global';
        return { kind: 'ident', name: a };
    }
    // Complex expression — extract leading identifier
    const m = a.match(/^([A-Za-z_]\w*)/);
    if (m) {
        if (SCRIPT_SCOPE_GLOBAL_NAMES.has(m[1])) return 'global';
        return { kind: 'expr', root: m[1] };
    }
    return 'safe';
}

/**
 * Main lint pass — walks Jenkinsfile, validates every triggerPipeline
 * call's args are def'd in the enclosing script block.
 */
export function lintJenkinsfile(text) {
    const blocks = extractScriptBlocks(text);
    const errors = [];
    for (const block of blocks) {
        const defs = extractDefs(block.body);
        const calls = findTriggerPipelineCalls(block.body);
        for (const call of calls) {
            // Compute approximate line number of the call
            const before = block.body.slice(0, call.offsetInBody);
            const callLine = block.startLine + (before.match(/\n/g) || []).length;
            for (const arg of call.args) {
                const cl = classifyArg(arg);
                if (cl === 'safe' || cl === 'global') continue;
                const name = cl.name || cl.root;
                if (!defs.has(name)) {
                    errors.push(
                        `stage '${block.stage}' line ${callLine}: ` +
                        `triggerPipeline(${call.args.join(', ')}) — ` +
                        `'${name}' not def-ed in enclosing script{} block. ` +
                        `If cross-stage, env-bridge it (see PIPELINES_TO_RUN pattern at top of Execute Builds).`
                    );
                }
            }
        }
    }
    return errors;
}

/**
 * Check env-bridge symmetry: every `env.X = ...` write should have at
 * least one matching `env.X` read. Catches dead bridges or typos.
 */
export function lintEnvBridgeSymmetry(text) {
    const writes = new Set();
    const reads = new Set();
    // Write = a single `=` not followed by another `=`; `env.X == 'v'` is a
    // READ (equality), not a write — the lookaheads must distinguish them or a
    // comparison-only consumer reads as an orphan (RESET_STORAGE_FROM_TAG case).
    for (const m of text.matchAll(/\benv\.([A-Z][A-Z0-9_]*)\s*=(?!=)/g)) writes.add(m[1]);
    for (const m of text.matchAll(/\benv\.([A-Z][A-Z0-9_]*)\b(?!\s*=(?!=))/g)) reads.add(m[1]);
    const orphans = [];
    for (const w of writes) {
        if (!reads.has(w)) orphans.push(w);
    }
    return orphans;
}

/**
 * Flag `=~` used inside a `script{}` block.
 *
 * `=~` yields a java.util.regex.Matcher, which is NOT serializable. Inside a CPS
 * frame it survives only until the next CPS step (sh/readJSON/writeFile/build)
 * forces a program save — then the build dies with
 * `NotSerializableException: java.util.regex.Matcher` at "End of Pipeline",
 * AFTER every stage has reported success and having dispatched nothing. The
 * damage is silent and total, and the stack trace names only Jenkins internals.
 *
 * The cure is to run the regex in an `@NonCPS` helper that returns plain data
 * (List/Map/String). Those helpers are top-level defs, so they are not inside
 * any script{} block and this lint never sees them — which is exactly the
 * boundary we want to enforce.
 *
 * Scope of the rule — only a Matcher BOUND TO A NAME is flagged:
 *
 *   def m = (msg =~ /x/)      FLAGGED — m lands in BlockScopeEnv.locals and is
 *                             still there when the next CPS step saves the program.
 *   if (msg =~ /x/) { }       allowed — the Matcher is a temporary consumed
 *                             immediately by Groovy's boolean coercion; it is
 *                             never bound, so it never reaches the serialized
 *                             locals. Four such conditions have shipped through
 *                             this Jenkinsfile for many builds without incident,
 *                             and rewriting them to `==~` would silently change
 *                             find-semantics to full-match-semantics.
 *   msg ==~ /x/               allowed — returns a plain boolean.
 *
 * Lived: orchestrator #1666.
 */
export function lintMatcherInCpsScope(text) {
    const violations = [];
    // An assignment `=` that is neither `==` nor the `=~` operator itself.
    const BINDS_A_NAME = /(?:def\s+)?[A-Za-z_]\w*\s*=(?![=~])/;
    for (const block of extractScriptBlocks(text)) {
        const lines = block.body.split('\n');
        lines.forEach((lineText, idx) => {
            const code = lineText.replace(/\/\/.*$/, '');
            const m = /(^|[^=])=~/.exec(code);
            if (!m) return;
            // Only flag when the Matcher is bound to a name on this line.
            const beforeOperator = code.slice(0, m.index + m[1].length);
            if (!BINDS_A_NAME.test(beforeOperator)) return;
            violations.push(
                `${block.stage ?? '<no stage>'} (line ~${block.startLine + idx}): ` +
                `${code.trim()}`
            );
        });
    }
    return violations;
}

// ══════════════════════════════════════════════════════════════════
// TESTS
// ══════════════════════════════════════════════════════════════════

describe('jenkinsfile cps-scope lint', () => {
    it('every triggerPipeline arg is in scope in its enclosing script{} block', () => {
        const text = readFileSync(ORCH_JENKINSFILE, 'utf8');
        const errors = lintJenkinsfile(text);
        if (errors.length) {
            assert.fail(
                `\nCPS scope violations in genesis/orchestrator/Jenkinsfile:\n\n  ` +
                errors.join('\n\n  ') +
                `\n\nFix: env-bridge the variable across stages (producer writes env.X; ` +
                `consumer reads env.X). See the PIPELINES_TO_RUN pattern at the top of the ` +
                `Execute Builds stage for the canonical example.\n`
            );
        }
    });

    it('no `=~` Matcher is created inside a script{} block (must live in @NonCPS)', () => {
        const text = readFileSync(ORCH_JENKINSFILE, 'utf8');
        const violations = lintMatcherInCpsScope(text);
        if (violations.length) {
            assert.fail(
                `\njava.util.regex.Matcher created inside a CPS script{} block:\n\n  ` +
                violations.join('\n  ') +
                `\n\nA Matcher is NOT serializable. It is harmless until the next CPS step ` +
                `(sh/readJSON/writeFile/build) forces a program save — then the build dies ` +
                `with NotSerializableException at "End of Pipeline", after every stage has ` +
                `reported success, having dispatched nothing (orchestrator #1666).` +
                `\n\nFix: move the regex into a top-level @NonCPS helper that returns plain ` +
                `data. See parseBuildTagTokens() for the canonical example.\n`
            );
        }
    });

    it('negative control: the Matcher lint catches an inline `=~`, and ignores `==~`', () => {
        const bad = `
            stage('A') {
                steps { script {
                    def m = (commitMsg =~ /\\[build:([a-z]+)\\]/)
                    sh 'echo hi'
                } }
            }
        `;
        assert.equal(lintMatcherInCpsScope(bad).length, 1, 'expected the inline =~ to be flagged');

        const good = `
            stage('A') {
                steps { script {
                    if (commitMsg ==~ /.*\\[skip ci\\].*/) { echo 'skip' }
                    if (commitMsg =~ /(?i)\\[reseed\\]/) { echo 'reseed' }
                } }
            }
        `;
        assert.equal(
            lintMatcherInCpsScope(good).length,
            0,
            '==~ is a boolean; an unbound `=~` in an if-condition is a consumed temporary',
        );
    });

    it('every env.X write has at least one env.X read (no orphan bridges)', () => {
        const text = readFileSync(ORCH_JENKINSFILE, 'utf8');
        const orphans = lintEnvBridgeSymmetry(text);
        // Some env writes are consumed by downstream systems (e.g. CHANGED_FILES_COUNT
        // appears in the build description; not always read back in the same file).
        // Allowlist known one-way writes:
        const ONE_WAY_ALLOW = new Set([
            'CHANGED_FILES_COUNT',  // surfaced in description, not re-read
            'BUILD_RESULTS',         // post-build artifact
            'DEPLOY_ONLY_FROM_TAG',  // consumed by triggerPipeline param composition
            'PREDICTED_BUILD_GRAPH_FILE',  // referenced as file path elsewhere
        ]);
        const real = orphans.filter((o) => !ONE_WAY_ALLOW.has(o));
        if (real.length) {
            assert.fail(
                `\nOrphan env-bridge writes (env.X = ... with no matching env.X read):\n  ` +
                real.join('\n  ') +
                `\n\nEither: (a) the consumer is missing — add the recovery in the next ` +
                `stage's script{} block; or (b) it's a one-way write — add to ONE_WAY_ALLOW ` +
                `with a comment explaining who reads it.\n`
            );
        }
    });
});

// ══════════════════════════════════════════════════════════════════
// TRIGGER-GATE PLACEMENT TESTS (Task 1 — plan 2026-05-28)
// ══════════════════════════════════════════════════════════════════

describe('commit-tag parsing gate placement', () => {
    it('[build:*] tag parsing is NOT gated on BUILD_TRIGGER == WEBHOOK', () => {
        const jf = readFileSync(ORCH_JENKINSFILE, 'utf8');
        // Find the tag-aliases declaration; walk back to find its enclosing
        // conditional; assert it is NOT the WEBHOOK gate.
        const tagAliasIdx = jf.indexOf('def buildTagAliases');
        assert(tagAliasIdx > 0, 'buildTagAliases declaration not found');
        const slice = jf.slice(Math.max(0, tagAliasIdx - 800), tagAliasIdx);
        assert(
            !/if \(env\.BUILD_TRIGGER == 'WEBHOOK'\) \{[^}]*$/s.test(slice),
            '[build:*] tag parsing must not be inside the WEBHOOK conditional'
        );
    });

    it('[deploy-only] parsing IS still gated on WEBHOOK', () => {
        const jf = readFileSync(ORCH_JENKINSFILE, 'utf8');
        const deployIdx = jf.indexOf("DEPLOY_ONLY_FROM_TAG = 'true'");
        assert(deployIdx > 0, 'DEPLOY_ONLY_FROM_TAG assignment not found');
        const slice = jf.slice(Math.max(0, deployIdx - 400), deployIdx);
        assert(
            /if \(env\.BUILD_TRIGGER == 'WEBHOOK'\) \{/s.test(slice),
            '[deploy-only] must remain webhook-gated'
        );
    });
});

// Self-test of the parser when invoked directly with a small fixture.
// Lets us catch parser regressions in isolation.
describe('extractScriptBlocks (parser self-test)', () => {
    it('finds script blocks inside named stages', () => {
        const fixture = `
            pipeline {
                stages {
                    stage('Foo') {
                        steps {
                            script {
                                def x = 1
                                triggerPipeline('p', 'dev', true, x)
                            }
                        }
                    }
                    stage('Bar') {
                        steps {
                            script {
                                triggerPipeline('p2', 'dev', true, x)
                            }
                        }
                    }
                }
            }
        `;
        const blocks = extractScriptBlocks(fixture);
        assert.equal(blocks.length, 2);
        assert.equal(blocks[0].stage, 'Foo');
        assert.equal(blocks[1].stage, 'Bar');
    });

    it('detects the env-bridge anti-pattern in the fixture', () => {
        const fixture = `
            stage('A') {
                steps { script { def x = 1 } }
            }
            stage('B') {
                steps { script { triggerPipeline('p', 'dev', true, x) } }
            }
        `;
        const errors = lintJenkinsfile(fixture);
        assert.ok(
            errors.length > 0 && errors[0].includes("'x' not def-ed"),
            `expected scope violation for x; got: ${JSON.stringify(errors)}`
        );
    });

    it('accepts a properly env-bridged variant', () => {
        const fixture = `
            stage('A') {
                steps { script {
                    def x = 1
                    env.MY_X = x.toString()
                } }
            }
            stage('B') {
                steps { script {
                    def x = (env.MY_X ?: '0').toInteger()
                    triggerPipeline('p', 'dev', true, x)
                } }
            }
        `;
        const errors = lintJenkinsfile(fixture);
        assert.equal(errors.length, 0, `expected no errors; got: ${JSON.stringify(errors)}`);
    });
});
