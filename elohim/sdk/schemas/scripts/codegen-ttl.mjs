#!/usr/bin/env node
/**
 * Emits the protocol's vocabularies as a Turtle/SKOS projection, so a counterparty who
 * does not share our code can read our terms AND learn how to read our silence.
 *
 * This is the reciprocal half of the closure discipline. We ask a foreign gate to declare
 * what its silence means (app-manifest `GateDeclaration.closure`); publishing the same
 * declaration about ourselves in a syntax anyone can parse is what makes that ask fair
 * rather than unilateral.
 *
 * THE LOAD-BEARING CONSTRAINT — naming, never axioms:
 *
 *   This emitter writes SKOS concepts and `epr:` annotations. It emits ZERO `owl:` terms,
 *   by construction. SKOS is deliberately weak — a concept scheme says "these are the terms
 *   and what they mean", never "here is how to decide". No reasoner can chain anything here
 *   into a verdict, which is precisely why this projection is safe to publish while a
 *   reasoner-bearing ontology would not be (see the OWL2 graduation ledger: ex falso is a
 *   fail-open authorization catastrophe, and an entailment regime whose materialization
 *   order is unspecified forks the DHT).
 *
 *   The test that keeps it honest: DELETING THIS OUTPUT CHANGES ZERO VERDICTS. Nothing in
 *   the protocol reads it. It is a projection outward, never a source.
 *
 * Usage:
 *   node codegen-ttl.mjs           # generate
 *   node codegen-ttl.mjs --verify  # fail if the checked-in output is stale
 */
import { readdir, readFile, writeFile, mkdir } from 'node:fs/promises';
import { join, resolve, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const SCHEMA_DIR = resolve(__dirname, '../current');
const ENUM_DIR = join(SCHEMA_DIR, 'enums');
const AXES_DIR = join(SCHEMA_DIR, 'registries', 'axes');
const OUT_DIR = resolve(__dirname, '../generated-ttl');
const OUT_FILE = join(OUT_DIR, 'elohim-vocabulary.ttl');

const VERIFY = process.argv.includes('--verify');

const loadJson = async (p) => JSON.parse(await readFile(p, 'utf8'));

/** Turtle string literal. Escapes per the Turtle grammar; never interpolate raw. */
function lit(s) {
  const escaped = String(s)
    .replace(/\\/g, '\\\\')
    .replace(/"/g, '\\"')
    .replace(/\n/g, '\\n')
    .replace(/\r/g, '\\r')
    .replace(/\t/g, '\\t');
  return `"${escaped}"`;
}

/** Full IRI in angle brackets — `epr:` ids carry colons that a prefixed name cannot hold. */
const iri = (s) => `<${s}>`;

async function build() {
  const lines = [];
  const p = (s = '') => lines.push(s);

  p('# Elohim Protocol — vocabulary projection (SKOS).');
  p('#');
  p('# GENERATED from elohim/sdk/schemas/v1/. Do not hand-edit: run');
  p('#   node elohim/sdk/schemas/scripts/codegen-ttl.mjs');
  p('#');
  p('# This file is a PROJECTION OUTWARD, never a source. Nothing in the protocol reads it,');
  p('# and deleting it changes zero verdicts — that is the test that keeps it safe to publish.');
  p('#');
  p('# It is NAMING, NOT DECIDING. There are no owl: terms here by construction: no');
  p('# equivalentClass, no property chains, no complement, nothing a reasoner could chain');
  p('# into a verdict. What you get is the terms we use, what they mean, and — the part that');
  p('# matters when we have never met — WHAT OUR SILENCE MEANS on each axis (epr:factClosure');
  p('# / epr:verdictClosure below).');
  p('#');
  p('# Reading our silence: on a layer declared `open`, the absence of an assertion is');
  p('# UNKNOWN and never a negative fact. On a layer declared `closed`, the absence of a');
  p('# permit is a REFUSAL and never "unknown, therefore possibly allowed".');
  p();
  p('@prefix skos: <http://www.w3.org/2004/02/skos/core#> .');
  p('@prefix dct:  <http://purl.org/dc/terms/> .');
  p('@prefix rdfs: <http://www.w3.org/2000/01/rdf-schema#> .');
  p('@prefix xsd:  <http://www.w3.org/2001/XMLSchema#> .');
  p('@prefix epr:  <epr:vocab#> .');
  p();
  p('# --- epr: annotation terms (documentation only; no formal semantics) ---');
  p(`${iri('epr:vocab#factClosure')} a rdfs:Datatype ;`);
  p(`  rdfs:comment ${lit('Posture of an axis FACT layer toward absence: open = unknown, never a negative fact.')} .`);
  p();
  p(`${iri('epr:vocab#verdictClosure')} a rdfs:Datatype ;`);
  p(`  rdfs:comment ${lit('Posture of an axis VERDICT layer toward absence: closed = absence of a permit is a refusal.')} .`);
  p();

  // --- Concept schemes, one per protocol enum ---
  const enumFiles = (await readdir(ENUM_DIR)).filter((f) => f.endsWith('.schema.json')).sort();
  let conceptCount = 0;
  p(`# =============== vocabularies (${enumFiles.length}) ===============`);
  p();
  for (const file of enumFiles) {
    const s = await loadJson(join(ENUM_DIR, file));
    if (!s.$id || !Array.isArray(s.enum)) continue;
    const schemeIri = iri(s.$id);
    p(`${schemeIri} a skos:ConceptScheme ;`);
    p(`  skos:prefLabel ${lit(s.title ?? basename(file, '.schema.json'))} ;`);
    if (s.description) p(`  dct:description ${lit(s.description)} ;`);
    // `_ordinal` is carried as a plain annotation, not as an rdf:List: an ordered
    // collection invites a consumer to infer comparisons we have not sanctioned.
    p(`  ${iri('epr:vocab#ordered')} ${s._ordinal === true ? 'true' : 'false'} ;`);
    p(`  skos:hasTopConcept ${s.enum.map((v) => iri(`${s.$id}#${v}`)).join(', ')} .`);
    p();
    s.enum.forEach((value, idx) => {
      conceptCount += 1;
      p(`${iri(`${s.$id}#${value}`)} a skos:Concept ;`);
      p(`  skos:inScheme ${schemeIri} ;`);
      p(`  skos:prefLabel ${lit(value)} ;`);
      if (s._ordinal === true) {
        p(`  ${iri('epr:vocab#ordinalPosition')} "${idx}"^^xsd:integer ;`);
      }
      p(`  skos:notation ${lit(value)} .`);
      p();
    });
  }

  // --- Axis cards: the closure declaration, published about ourselves ---
  let axisFiles = [];
  try {
    axisFiles = (await readdir(AXES_DIR)).filter((f) => f.endsWith('.axis.json')).sort();
  } catch {
    // no axes registry yet — the vocabularies still project
  }
  p(`# =============== axes (${axisFiles.length}) — what our silence means ===============`);
  p();
  for (const file of axisFiles) {
    const a = await loadJson(join(AXES_DIR, file));
    if (!a.$id) continue;
    p(`${iri(a.$id)} a ${iri('epr:vocab#Axis')} ;`);
    p(`  skos:prefLabel ${lit(a.axis)} ;`);
    if (a.semantics) p(`  dct:description ${lit(a.semantics)} ;`);
    if (a.vocabulary?.$ref) p(`  ${iri('epr:vocab#vocabulary')} ${iri(a.vocabulary.$ref)} ;`);
    if (a.kind) p(`  ${iri('epr:vocab#kind')} ${lit(a.kind)} ;`);
    if (a.closure?.facts) p(`  ${iri('epr:vocab#factClosure')} ${lit(a.closure.facts)} ;`);
    if (a.closure?.rationale) p(`  ${iri('epr:vocab#closureRationale')} ${lit(a.closure.rationale)} ;`);
    // Orthogonality is published because axis confusion is the recurring drift — a
    // consumer mapping their vocabulary onto ours needs to know which of our axes must
    // NOT be collapsed together.
    for (const o of a.orthogonalTo ?? []) {
      p(`  ${iri('epr:vocab#orthogonalTo')} ${lit(o)} ;`);
    }
    p(`  ${iri('epr:vocab#verdictClosure')} ${lit(a.closure?.verdicts ?? 'undeclared')} .`);
    p();
  }

  p(`# ${enumFiles.length} vocabularies · ${conceptCount} concepts · ${axisFiles.length} axes`);
  return { text: lines.join('\n') + '\n', enumCount: enumFiles.length, conceptCount, axisCount: axisFiles.length };
}

const { text, enumCount, conceptCount, axisCount } = await build();

if (VERIFY) {
  let current = null;
  try {
    current = await readFile(OUT_FILE, 'utf8');
  } catch {
    console.error(`STALE: ${OUT_FILE} does not exist. Run: node elohim/sdk/schemas/scripts/codegen-ttl.mjs`);
    process.exit(1);
  }
  if (current !== text) {
    console.error(`STALE: ${OUT_FILE} differs from the schemas. Run: node elohim/sdk/schemas/scripts/codegen-ttl.mjs`);
    process.exit(1);
  }
  console.log(`Turtle projection is up to date (${enumCount} vocabularies, ${conceptCount} concepts, ${axisCount} axes).`);
} else {
  await mkdir(OUT_DIR, { recursive: true });
  await writeFile(OUT_FILE, text, 'utf8');
  console.log(`Wrote ${OUT_FILE} — ${enumCount} vocabularies, ${conceptCount} concepts, ${axisCount} axes.`);
}
