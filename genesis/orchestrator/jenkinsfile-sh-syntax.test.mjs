// Every literal shell block in a tracked Jenkinsfile must at least PARSE.
//
// A `sh '''…'''` block is handed to the agent's `sh` verbatim (no Groovy
// interpolation), so `sh -n` on its text is exactly the parse the build will do.
// elohim-eprfs #47 died in its first stage on `Syntax error: ")" unexpected`: a
// comment inside a `bash -c '…'` string contained an apostrophe, which ended the
// string. Nothing local parses these blocks; CodeNarc reads the Groovy, not the shell.
//
// `sh """…"""` blocks are skipped: Groovy interpolates them first, so their text
// here is not the text the agent sees.

import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const REPO = resolve(dirname(fileURLToPath(import.meta.url)), '..', '..');

/** Literal `'''` shell blocks of one Jenkinsfile: [{ line, script }]. */
export function literalShBlocks(text) {
  const blocks = [];
  const open = /\bsh\s*\(?\s*(?:script\s*:\s*)?'''/g;
  let m;
  while ((m = open.exec(text)) !== null) {
    const start = m.index + m[0].length;
    const end = text.indexOf("'''", start);
    if (end < 0) break;
    blocks.push({ line: text.slice(0, m.index).split('\n').length, script: text.slice(start, end) });
    open.lastIndex = end + 3;
  }
  return blocks;
}

/** `sh -n` verdict for one script: null when it parses, else the shell's message. */
export function shParseError(script) {
  const r = spawnSync('sh', ['-n'], { input: script, encoding: 'utf8' });
  return r.status === 0 ? null : (r.stderr || `exit ${r.status}`).trim();
}

function trackedJenkinsfiles() {
  return execFileSync('git', ['ls-files', '--', '*Jenkinsfile*'], { cwd: REPO, encoding: 'utf8' })
    .split('\n')
    .filter(f => f && !f.includes('node_modules/'));
}

test('an apostrophe in a comment inside bash -c is a parse error this rail sees', () => {
  const groovy = [
    "sh '''",
    "  nix develop --command bash -c '",
    '    cargo fmt --check',
    "    # the local gate's pin (see the other file)",
    '    cargo clippy',
    "  '",
    "'''",
  ].join('\n');
  const [block] = literalShBlocks(groovy);
  assert.ok(block, 'the block is found');
  assert.notEqual(shParseError(block.script), null);
});

test('a well-formed literal block parses', () => {
  const [block] = literalShBlocks("sh '''\n  set -eu\n  echo ok\n'''");
  assert.equal(shParseError(block.script), null);
});

test('every literal sh block in every tracked Jenkinsfile parses', () => {
  const failures = [];
  let seen = 0;
  for (const file of trackedJenkinsfiles()) {
    for (const { line, script } of literalShBlocks(readFileSync(join(REPO, file), 'utf8'))) {
      seen += 1;
      const err = shParseError(script);
      if (err) failures.push(`${file}:${line}: ${err}`);
    }
  }
  assert.ok(seen > 0, 'found no literal sh blocks at all; the extractor is broken');
  assert.deepEqual(failures, [], `shell blocks that do not parse:\n${failures.join('\n')}`);
});
