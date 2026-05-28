/**
 * Commit-message tag parsing. Extracted from orchestrator-strategy.mjs
 * so it can survive that module's deletion.
 *
 * Exports:
 *   parseCommitTags(commitMsg, registry) → string[]
 *   parseSkipCi(commitMsg) → boolean
 *   BUILD_TAG_ALIASES
 */

import { nonManualPipelines } from './pipeline-registry.mjs';

export const BUILD_TAG_ALIASES = {
  edge: 'elohim-edge',
  dna: 'elohim-holochain',
  app: 'elohim',
  genesis: 'elohim-genesis',
  sophia: 'elohim-sophia',
  steward: 'elohim-steward',
};

/**
 * Parse [build:*] commit-message tags and return the resolved pipeline names.
 *
 * @param {string} commitMsg
 * @param {Map<string, object>} registry  from loadPipelineRegistry()
 * @returns {string[]}  deduplicated list of pipeline names to force-build
 */
export function parseCommitTags(commitMsg, registry) {
  const buildTags = [];
  const tagRegex = /\[build:([a-z,-]+)\]/gi;
  let match;
  while ((match = tagRegex.exec(commitMsg)) !== null) {
    for (const tag of match[1].split(',')) {
      const t = tag.trim().toLowerCase();
      if (t === 'all') {
        buildTags.push(...nonManualPipelines(registry));
      } else if (BUILD_TAG_ALIASES[t]) {
        buildTags.push(BUILD_TAG_ALIASES[t]);
      }
      // Unknown tags are silently dropped (mirrors Jenkinsfile ⚠️ echo behaviour
      // — the echo is cosmetic; the tag is not an error).
    }
  }
  return [...new Set(buildTags)];
}

/**
 * Returns true if the commit message contains a [skip ci] / [ci skip] /
 * [no ci] tag (case-insensitive).
 *
 * @param {string} commitMsg
 * @returns {boolean}
 */
export function parseSkipCi(commitMsg) {
  return /\[(skip ci|ci skip|no ci)\]/i.test(commitMsg);
}
