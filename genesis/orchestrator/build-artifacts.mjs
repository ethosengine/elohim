// Build artifact filenames — single source of truth.
//
// Cross-language references (Groovy, TypeScript, JavaScript) all resolve
// through build-artifacts.json. Renaming an artifact means updating ONE
// entry there; consumers pick up the new name automatically.
//
// Library usage (JS / TS via tsx):
//   import { ARTIFACTS, ORCHESTRATOR, GENESIS } from './build-artifacts.mjs';
//   console.log(ORCHESTRATOR.predictedBuildGraph); // 'predicted-build-graph.json'
//
// Groovy usage (Jenkinsfile):
//   def artifacts = readJSON(file: 'genesis/orchestrator/build-artifacts.json')
//   writeJSON(file: artifacts.orchestrator.predictedBuildGraph, ...)

import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const __dirname = dirname(fileURLToPath(import.meta.url));
const MANIFEST_PATH = resolve(__dirname, 'build-artifacts.json');

const manifest = JSON.parse(readFileSync(MANIFEST_PATH, 'utf8'));

export const ARTIFACTS = manifest;
export const ORCHESTRATOR = manifest.orchestrator;
export const GENESIS = manifest.genesis;
export const MANIFEST_FILE_PATH = MANIFEST_PATH;
