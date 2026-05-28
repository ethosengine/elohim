#!/usr/bin/env node
// federation-routing-dryrun.mjs — structural assertions for per-human primary
// doorway routing in elohim/holochain/Jenkinsfile.
//
// MEASURE SEMANTICS
// This is the oracle for the federation-wiring-audit shift's Phase 1. It
// reads the Jenkinsfile (the implementation under test) plus deployments.json
// (the persona registry) and prints a single non-negative integer on stdout:
// the count of passing assertions. The shift's measure target is the total
// number of assertions (currently 5). Anything less than that means Phase 1
// is incomplete or wrong.
//
// The expected URL strings and selection logic embedded below are the FIXTURE.
// Editing them to match a broken Jenkinsfile is forbidden by shift discipline
// (no editing the judge). The Jenkinsfile is the implementation; this file is
// the spec the implementation must satisfy.
//
// USAGE
//   node genesis/agentic/bin/federation-routing-dryrun.mjs
//   node genesis/agentic/bin/federation-routing-dryrun.mjs --verbose
//
// Exit code is always 0 — caller distinguishes pass/fail by reading the
// numeric stdout against the Objective's target.

import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

const REPO_ROOT = resolve(new URL('../../..', import.meta.url).pathname);
const JENKINSFILE = `${REPO_ROOT}/elohim/holochain/Jenkinsfile`;
const DEPLOYMENTS = `${REPO_ROOT}/genesis/orchestrator/data/deployments.json`;

const EXPECTED = {
  doorwayA: {
    bootstrapUrl: 'https://doorway-alpha.elohim.host/bootstrap',
    signalUrl: 'wss://signal.doorway-alpha.elohim.host',
  },
  doorwayB: {
    bootstrapUrl: 'https://elohim.host/bootstrap',
    signalUrl: 'wss://signal.elohim.host',
  },
};

// Personas the shift uses as test cases — one on-prem (jessica), two remote
// (adam = file-driven manifest, nancy = template-driven). Keep the list small
// and meaningful; full-cluster coverage isn't the point.
const TEST_PERSONAS = [
  { name: 'jessica', expectedDoorway: 'doorwayA' },
  { name: 'adam', expectedDoorway: 'doorwayB' },
  { name: 'nancy', expectedDoorway: 'doorwayB' },
];

const verbose = process.argv.includes('--verbose');

function loadJenkinsfile() {
  return readFileSync(JENKINSFILE, 'utf8');
}

function loadDeployments() {
  return JSON.parse(readFileSync(DEPLOYMENTS, 'utf8'));
}

/**
 * Extract the alpha env block by line range. We grep for "alpha:" inside
 * getEnvConfig and slice until the next top-level env key (staging:) — the
 * regex flavor groovy uses makes a single-shot match brittle, so a line
 * scanner is more honest about the structure we care about.
 */
function extractAlphaEnvBlock(src) {
  const lines = src.split('\n');
  const start = lines.findIndex((l) => /^\s+alpha:\s*\[/.test(l));
  if (start < 0) return null;
  const end = lines.findIndex((l, i) => i > start && /^\s+staging:\s*\[/.test(l));
  return lines.slice(start, end > 0 ? end : lines.length).join('\n');
}

/**
 * Locate the deployHumanManifest function body — we'll inspect its sed args.
 */
function extractDeployHumanManifestBody(src) {
  const startMarker = 'def deployHumanManifest(Map humanConfig, String envFile, List allHumans)';
  const startIdx = src.indexOf(startMarker);
  if (startIdx < 0) return null;
  // Find the matching closing brace by depth counting from the next '{'.
  const openBrace = src.indexOf('{', startIdx);
  if (openBrace < 0) return null;
  let depth = 1;
  let i = openBrace + 1;
  while (i < src.length && depth > 0) {
    const c = src[i];
    if (c === '{') depth += 1;
    else if (c === '}') depth -= 1;
    i += 1;
  }
  if (depth !== 0) return null;
  return src.slice(openBrace, i);
}

function assertions() {
  const results = [];
  const src = loadJenkinsfile();
  const alphaBlock = extractAlphaEnvBlock(src);
  const deployBody = extractDeployHumanManifestBody(src);

  // Assertion 1: alpha envConfig contains a doorwayA sub-map with the
  // expected on-prem URLs (the existing single-doorway hostnames).
  results.push({
    id: 'alpha.doorwayA.urls',
    pass:
      alphaBlock != null &&
      /doorwayA\s*:/.test(alphaBlock) &&
      alphaBlock.includes(EXPECTED.doorwayA.bootstrapUrl) &&
      alphaBlock.includes(EXPECTED.doorwayA.signalUrl),
    detail: 'alpha envConfig must declare doorwayA with the on-prem bootstrap+signal URLs',
  });

  // Assertion 2: alpha envConfig contains a doorwayB sub-map with the shem
  // apex URLs (matching alpha-b.yaml's ingress).
  results.push({
    id: 'alpha.doorwayB.urls',
    pass:
      alphaBlock != null &&
      /doorwayB\s*:/.test(alphaBlock) &&
      alphaBlock.includes(EXPECTED.doorwayB.bootstrapUrl) &&
      alphaBlock.includes(EXPECTED.doorwayB.signalUrl),
    detail: 'alpha envConfig must declare doorwayB with the shem apex bootstrap+signal URLs',
  });

  // Assertion 3: deployHumanManifest contains a per-human doorway selection
  // expression keyed off nodeTypes containing "remote". The exact Groovy
  // shape can be `humanConfig.nodeTypes.contains('remote')` or equivalent;
  // we look for the discriminator literal "remote" in the same function body
  // as a doorway-bearing local (doorwayB referenced as the value branch).
  const hasRemoteDiscriminator =
    deployBody != null &&
    /nodeTypes[^\n]*['"]remote['"]/.test(deployBody) &&
    /doorwayB/.test(deployBody);
  results.push({
    id: 'deployHumanManifest.selection',
    pass: hasRemoteDiscriminator,
    detail:
      'deployHumanManifest must select between doorwayA/doorwayB based on humanConfig.nodeTypes containing "remote"',
  });

  // Assertion 4: the BOOTSTRAP_URL sed substitution references the per-human
  // selection (not envConfig.bootstrapUrl directly). We look for the sed
  // pattern followed by a Groovy interpolation that names a doorway-local
  // (primaryDoorway / doorwayPair / similar) — not the bare envConfig.
  results.push({
    id: 'sed.BOOTSTRAP_URL.per_human',
    pass:
      deployBody != null &&
      /BOOTSTRAP_URL_PLACEHOLDER\|\$\{([a-zA-Z_]+)\.bootstrapUrl\}/.test(deployBody) &&
      !/BOOTSTRAP_URL_PLACEHOLDER\|\$\{envConfig\.bootstrapUrl\}/.test(deployBody),
    detail:
      'BOOTSTRAP_URL_PLACEHOLDER sed must interpolate a per-human doorway local (e.g. ${primaryDoorway.bootstrapUrl}), not envConfig.bootstrapUrl',
  });

  // Assertion 5: same for SIGNAL_URL.
  results.push({
    id: 'sed.SIGNAL_URL.per_human',
    pass:
      deployBody != null &&
      /SIGNAL_URL_PLACEHOLDER\|\$\{([a-zA-Z_]+)\.signalUrl\}/.test(deployBody) &&
      !/SIGNAL_URL_PLACEHOLDER\|\$\{envConfig\.signalUrl\}/.test(deployBody),
    detail:
      'SIGNAL_URL_PLACEHOLDER sed must interpolate a per-human doorway local (e.g. ${primaryDoorway.signalUrl}), not envConfig.signalUrl',
  });

  return results;
}

/**
 * Verify the persona registry maps to expected doorways. This is informational
 * — it doesn't add to the count, but the verbose mode prints it so the operator
 * can see which personas would route where under the rule.
 */
function describePersonas() {
  const { humans } = loadDeployments();
  const byName = Object.fromEntries(humans.map((h) => [h.name, h]));
  const lines = [];
  for (const { name, expectedDoorway } of TEST_PERSONAS) {
    const h = byName[name];
    if (!h) {
      lines.push(`  ${name}: MISSING from deployments.json`);
      continue;
    }
    const isRemote = (h.nodeTypes || []).includes('remote');
    const derivedDoorway = isRemote ? 'doorwayB' : 'doorwayA';
    const match = derivedDoorway === expectedDoorway ? 'ok' : 'MISMATCH';
    lines.push(
      `  ${name}: nodeTypes=${JSON.stringify(h.nodeTypes || [])} → ${derivedDoorway} (expected ${expectedDoorway}) [${match}]`,
    );
  }
  return lines.join('\n');
}

const results = assertions();
const passing = results.filter((r) => r.pass).length;

if (verbose) {
  process.stderr.write(`federation-routing-dryrun: ${passing}/${results.length} passing\n`);
  for (const r of results) {
    process.stderr.write(`  [${r.pass ? 'PASS' : 'FAIL'}] ${r.id} — ${r.detail}\n`);
  }
  process.stderr.write('\npersonas (nodeTypes → derived doorway):\n');
  process.stderr.write(describePersonas() + '\n');
}

process.stdout.write(String(passing) + '\n');
