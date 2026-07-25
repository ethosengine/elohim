import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readdir, readFile } from 'node:fs/promises';
import { resolve, dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMA_ROOT = resolve(__dirname, '../..');
const TTL = resolve(SCHEMA_ROOT, 'generated-ttl/elohim-vocabulary.ttl');
const ENUM_DIR = resolve(SCHEMA_ROOT, 'v1/enums');

const ttl = await readFile(TTL, 'utf8');

// The RDF content with `#` comment lines stripped. The header prose explains the no-OWL
// invariant and therefore contains the very token that invariant forbids — checking the
// raw file would fail on its own documentation.
const ttlStatements = ttl
  .split('\n')
  .filter((l) => !l.trimStart().startsWith('#'))
  .join('\n');

// NOTE ON SCOPE: these are STRUCTURAL checks, not a full Turtle parse. There is no RDF
// parser in this workspace and adding one as a dependency to validate a projection that
// nothing reads would be a poor trade — the projection exists so OTHERS can parse it. What
// these do cover is the actual risk surface of a hand-rolled serializer: the escaper, and
// the one invariant that makes the projection safe to publish at all.

test('the projection emits NAMING, never axioms — no owl: terms, by construction', () => {
  // The load-bearing invariant. SKOS says "these are the terms and what they mean"; OWL
  // says "here is what follows". Publishing the second would hand a counterparty a
  // decision procedure whose ex-falso behaviour is a fail-open authorization catastrophe
  // and whose materialization order is unspecified across implementations. If this ever
  // fails, the projection has stopped being safe to publish.
  assert.ok(!/\bowl:/.test(ttlStatements), 'no owl: prefix may appear in the projection');
  assert.ok(
    !/equivalentClass|propertyChainAxiom|complementOf|sameAs|InverseFunctional/.test(ttlStatements),
    'no OWL axiom construct may appear, prefixed or not',
  );
});

test('every closure posture published is a value of the shared closure vocabulary', async () => {
  const closureEnum = JSON.parse(await readFile(join(ENUM_DIR, 'closure.schema.json'), 'utf8'));
  const declared = new Set(closureEnum.enum);
  const found = [...ttl.matchAll(/<epr:vocab#(?:fact|verdict)Closure>\s+"([^"]*)"/g)].map((m) => m[1]);
  assert.ok(found.length > 0, 'the projection must publish at least one closure posture');
  for (const v of found) {
    assert.ok(declared.has(v), `published closure "${v}" is outside epr:schema:enum:closure`);
  }
});

test('escaping round-trips — every enum description survives into a literal intact', async () => {
  // Directly tests the hand-rolled escaper against real inputs: the descriptions carry
  // quotes, backticks, em-dashes and colons, which is exactly where a naive serializer
  // silently corrupts or truncates.
  const unescape = (s) => s
    .replace(/\\n/g, '\n').replace(/\\r/g, '\r').replace(/\\t/g, '\t')
    .replace(/\\"/g, '"').replace(/\\\\/g, '\\');
  const literals = new Set([...ttl.matchAll(/"((?:[^"\\]|\\.)*)"/g)].map((m) => unescape(m[1])));

  const files = (await readdir(ENUM_DIR)).filter((f) => f.endsWith('.schema.json'));
  let checked = 0;
  for (const f of files) {
    const s = JSON.parse(await readFile(join(ENUM_DIR, f), 'utf8'));
    if (!s.description) continue;
    assert.ok(literals.has(s.description),
      `description for ${f} did not survive escaping into the projection`);
    checked += 1;
  }
  assert.ok(checked > 20, `expected to check many descriptions, checked ${checked}`);
});

test('every enum value appears as a concept in its own scheme', async () => {
  const files = (await readdir(ENUM_DIR)).filter((f) => f.endsWith('.schema.json'));
  let concepts = 0;
  for (const f of files) {
    const s = JSON.parse(await readFile(join(ENUM_DIR, f), 'utf8'));
    if (!s.$id || !Array.isArray(s.enum)) continue;
    for (const v of s.enum) {
      assert.ok(ttl.includes(`<${s.$id}#${v}> a skos:Concept ;`),
        `${s.$id}#${v} missing from the projection`);
      concepts += 1;
    }
  }
  assert.equal(concepts, [...ttl.matchAll(/ a skos:Concept ;/g)].length,
    'projection concept count must equal the enum value count exactly (no strays, no drops)');
});

test('statements terminate — no dangling clause from a truncated emit', () => {
  const body = ttl.split('\n')
    .filter((l) => l.trim() && !l.trim().startsWith('#') && !l.trim().startsWith('@prefix'));
  // Every logical statement ends with ` .`; continuation lines end with ` ;` or ` ,`.
  for (const line of body) {
    const t = line.trimEnd();
    assert.ok(/[.;,]$/.test(t), `line does not terminate as a Turtle clause: ${t.slice(0, 80)}`);
  }
  assert.ok(ttl.trimEnd().endsWith('axes'), 'projection ends with its summary comment');
});
