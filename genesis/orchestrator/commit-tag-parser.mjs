/**
 * Commit-message tag parsing. Extracted from orchestrator-strategy.mjs
 * so it can survive that module's deletion.
 *
 * Exports:
 *   parseCommitTags(commitMsg, registry) → string[]
 *   parseSkipCi(commitMsg) → boolean
 *   parseConductorOptions(commitMsg) → object
 *   BUILD_TAG_ALIASES
 *   CONDUCTOR_FEATURE_PRESETS
 */

import { nonManualPipelines } from './pipeline-registry.mjs';

export const BUILD_TAG_ALIASES = {
  edge: 'elohim-edge',
  dna: 'elohim-holochain',
  app: 'elohim',
  genesis: 'elohim-genesis',
  sophia: 'elohim-sophia',
  steward: 'elohim-steward',
  conductor: 'elohim-conductor',
};

/**
 * Conductor feature-set presets for the [conductor:<variant>] commit tag.
 *
 * These mirror the elohim-edgenode job's Holochain 0.7 feature contract:
 *   - `prof` replaces the production allocator with the isolated profiling build.
 *   - `iroh` is retained for old commit messages, but is a no-op because iroh is
 *     the only 0.7 transport and has no cargo feature.
 */
export const CONDUCTOR_FEATURE_PRESETS = {
  prof: 'encryption,wasmer-sys-cranelift,jemalloc-prof',
  iroh: 'encryption,wasmer-sys-cranelift,jemalloc',
};

/**
 * Parse the [conductor:...] commit-message tag into elohim-edgenode job
 * parameters, so a conductor variant is buildable from a commit instead of a
 * hand-filled Jenkins parameter form.
 *
 * Grammar — comma-separated items inside one tag, repeatable:
 *
 *   [conductor:iroh]                 compatibility no-op; selects the 0.7 default
 *   [conductor:prof]                 jemalloc-prof heap-profiling canary (isolated)
 *   [conductor:canary]               also build the deployable storage image
 *   [conductor:no-push]              build only, no Harbor push (validation)
 *   [conductor:hc=<branch>]          override the holochain fork branch
 *   [conductor:tx5=<branch>]         compatibility-only; parsed but no longer forwarded
 *   [conductor:features=a+b+c]       raw HC_FEATURES ('+' separates; ',' is taken
 *                                    by the item separator). Wins over a preset.
 *
 * The `hc=` branch override is only consulted by the job when HC_REF is empty,
 * since an exact submodule SHA outranks a branch tip. Use it to build a fork
 * branch that has no submodule pointer yet.
 *
 * @param {string} commitMsg
 * @returns {{present: boolean, features?: string, hcBranch?: string,
 *            tx5Branch?: string, canary: boolean, skipPush: boolean,
 *            unknown: string[]}}
 */
export function parseConductorOptions(commitMsg) {
  const opts = { present: false, canary: false, skipPush: false, unknown: [] };
  const tagRegex = /\[conductor:([^\]]+)\]/gi;
  let preset;
  let explicitFeatures;
  let match;
  while ((match = tagRegex.exec(commitMsg)) !== null) {
    opts.present = true;
    for (const raw of match[1].split(',')) {
      const item = raw.trim();
      if (!item) continue;
      const eq = item.indexOf('=');
      if (eq === -1) {
        const flag = item.toLowerCase();
        if (flag === 'canary') opts.canary = true;
        else if (flag === 'no-push' || flag === 'dry') opts.skipPush = true;
        else if (CONDUCTOR_FEATURE_PRESETS[flag]) preset = CONDUCTOR_FEATURE_PRESETS[flag];
        else opts.unknown.push(item);
        continue;
      }
      const key = item.slice(0, eq).trim().toLowerCase();
      const value = item.slice(eq + 1).trim();
      if (!value) {
        opts.unknown.push(item);
      } else if (key === 'hc') {
        opts.hcBranch = value;
      } else if (key === 'tx5') {
        // Compatibility-only parse for old commit messages. The orchestrator no
        // longer forwards this value to the 0.7 conductor job.
        opts.tx5Branch = value;
      } else if (key === 'features') {
        explicitFeatures = value.replace(/\+/g, ',');
      } else {
        opts.unknown.push(item);
      }
    }
  }
  const features = explicitFeatures || preset;
  if (features) opts.features = features;
  return opts;
}

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

/**
 * CLI: emit [conductor:…] options as JSON for the orchestrator Jenkinsfile.
 *
 *   node commit-tag-parser.mjs conductor-opts <commit-message-file>
 *
 * The Jenkinsfile shells out rather than reimplementing the grammar, because
 * the [build:*] aliases are already duplicated there and that duplication is a
 * standing drift source. The message arrives via a FILE, never argv — commit
 * messages are untrusted multi-line text.
 */
if (process.argv[1] && process.argv[1].endsWith('commit-tag-parser.mjs')) {
  const [, , command, msgFile] = process.argv;
  if (command === 'conductor-opts') {
    const { readFileSync } = await import('node:fs');
    const msg = msgFile ? readFileSync(msgFile, 'utf8') : '';
    process.stdout.write(JSON.stringify(parseConductorOptions(msg)));
  } else if (command) {
    process.stderr.write(`unknown command '${command}' — expected 'conductor-opts'\n`);
    process.exit(2);
  }
}
