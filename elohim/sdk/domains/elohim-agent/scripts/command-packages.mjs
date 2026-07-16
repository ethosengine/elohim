// command-packages.mjs — the CommandPackage sibling module for
// package-projections.mjs.
//
// Slash commands (`.claude/commands/<name>.md`) are flat markdown prompts with
// optional frontmatter (`description:`), invoked as `/name`. Claude Code and
// Codex both consume a prompt-per-file convention, but from DIFFERENT homes:
// `.claude/commands/<name>.md` (Claude) and `.codex/commands/<name>.md` (Codex —
// Codex has no dedicated commands directory today, so this is the projection
// target of record; if a native codex prompts/ home lands later, only the codex
// projection path moves).
//
// WHY A DISTINCT KIND (not SkillPackage + metadata.form): a slash command's
// source is a flat file at `.claude/commands/<name>.md` (not a directory with a
// SKILL.md), its projection homes differ from a skill's, and its bare id can
// COLLIDE with a skill's (a `deliver` command AND a `deliver` skill both exist).
// A dedicated kind namespaces the id (`CommandPackage:deliver` ≠
// `SkillPackage:deliver`) and routes the paths cleanly, which is lighter in
// practice than special-casing the SkillPackage importer/projector/pathing on a
// `metadata.form` marker.
//
// AUTHORITY MODEL — dual, like the rest of the plant-eprfs family. An un-planted
// command is source-fidelity: the Claude file stays authored and the package is a
// certified mirror carrying the governance backref. A command with
// `metadata.master === "package"` is package-first: the package's `source.body`
// becomes authoritative and both runtime files are generated projections. The
// projector is VERBATIM PASSTHROUGH in either mode, so planting changes authority,
// never bytes. Before the flip the standing gate proves
// `project(import(source)) === source`; afterward it proves
// `project(package) === projection` for both runtimes.

import { basename, relative } from 'node:path';
import { frontmatterScalar } from './agent-doc-packages.mjs';

export const COMMAND_KIND = 'CommandPackage';

// The projected codex home for a command id. Centralized so a future native
// codex prompts/ convention is a one-line change.
export function codexCommandPath(id) {
  return `.codex/commands/${id}.md`;
}

// Derive a description for a command with no `description:` frontmatter: the first
// non-empty markdown heading, stripped of leading `#`. Falls back to the id.
function firstHeading(rawBytes, fallback) {
  for (const line of rawBytes.split('\n')) {
    const trimmed = line.trim();
    if (trimmed.startsWith('#')) return trimmed.replace(/^#+\s*/, '').trim() || fallback;
  }
  return fallback;
}

// Import a slash command from its source markdown. `source.body` is the ENTIRE raw
// file copied VERBATIM (frontmatter incl. any `description:` + body), zero
// transform — the byte-identity floor. `id` defaults to the basename without
// `.md` (unique across commands).
export function commandPackageFromSource(sourcePath, rawBytes, { repoRoot, id, governance } = {}) {
  const relPath = relative(repoRoot, sourcePath);
  const cmdId = id ?? basename(sourcePath, '.md');
  const description = frontmatterScalar(rawBytes, 'description') ?? firstHeading(rawBytes, cmdId);

  return {
    apiVersion: 'elohim-agent/v1alpha1',
    kind: COMMAND_KIND,
    metadata: {
      id: cmdId,
      name: cmdId,
      version: '1.0.0',
      description,
      form: 'command',
      sourceRuntime: 'claude', // born as a Claude slash command
      runtimeTargets: ['claude', 'codex'],
      assetRefs: [],
      governance,
    },
    // `source` (verbatim, like an agent-doc/hook), NOT `instructions` — a command
    // is portable prose, not a frontmatter-dialect markdown surface. `cid` omitted
    // (single-sourced from eprfs-agent, never computed in JS).
    source: {
      language: 'markdown',
      path: relPath,
      body: rawBytes,
    },
    projections: {
      claude: { path: relPath },
      codex: { path: codexCommandPath(cmdId) },
    },
  };
}

// The command projector: PURE VERBATIM PASSTHROUGH, runtime-agnostic. Both the
// claude source home and the codex mirror emit the exact same bytes, so
// project(import(source)) === source holds for claude (fidelity gate) and the
// codex mirror is byte-identical. (v1 note: the codex projection is a faithful
// mirror, not yet codex-localized — Claude-specific references like
// `superpowers:*` skills or `.claude/` paths ride through unchanged; localizing
// them is a residual parity gap, tracked in the report.)
export function projectCommand(pkg) {
  return pkg.source.body;
}

// Command-specific verify: a non-empty verbatim body. Byte-identity of the
// projected fixture / runtime files (both runtimes) is proved by the shared
// verifyProjectionFixture / verifyRuntimeProjectionIfPresent in the main file,
// and claude source round-trip by verifySourceFidelity.
export function verifyCommandPackage(pkg, { assert }) {
  assert(
    typeof pkg.source?.body === 'string' && pkg.source.body.length > 0,
    `${pkg.metadata.id} has verbatim command body`,
  );
}
