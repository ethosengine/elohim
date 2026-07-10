// agent-doc-packages.mjs — the AgentDocPackage sibling module for
// package-projections.mjs.
//
// Agent-docs (CLAUDE.md / AGENTS.md) are a NEW artifact class: GOSPEL /
// managed-memory markdown whose frontmatter is a LIVE index into other files
// (cite-gen writes `id:` + content-addressed `cites:` envelopes there). They are
// the most sensitive plant type — a mangled CLAUDE.md or a broken cite envelope
// damages the memory substrate. This module owns everything agent-doc-specific —
// the importer (doc → package), the VERBATIM projector (package → doc,
// byte-for-byte), and the agent-doc verify assertions — so the markdown-centric
// main file stays uncluttered and routes by `kind`.
//
// AUTHORITY MODEL — the FLIP (agent-docs become package-master, like every other
// plant type). The eprfs/epr-meta root IS the composition substrate that makes
// the flip compose: cites are content-addressed envelopes (sha256), the package
// graph is content-addressed (BlobCid), so the package absorbing cite provenance
// is a natural extension. The flip composes WITH cite-gen instead of fighting it
// because the projection is byte-identical — the doc file is never rewritten, so
// the cite fingerprints survive intact.
//
// SAFETY (load-bearing — do NOT relax):
//   1. Byte-identity is TRANSFORM-FREE and STRICT. import copies the ENTIRE raw
//      file (frontmatter incl. cite envelopes + body) into `source.body`; project
//      returns `source.body` UNCHANGED. project(import(doc)) === doc for ANY
//      bytes, strict `===` on the raw read — no trailing-newline normalization,
//      no trim(), no YAML round-trip, no frontmatter re-emission. `projectAgentDoc`
//      is pure passthrough for exactly this reason. This is what keeps cite-gen's
//      `id:`/`cites:` shape and the `gospel-claude-md` classifier intact — the
//      doc bytes never change, so the cite fingerprints never change.
//   2. The doc's frontmatter is NEVER injected with an authority marker. Unlike a
//      flipped skill (whose frontmatter is GENERATED from metadata), a flipped
//      agent-doc's frontmatter is passed through verbatim. `master: 'package'`
//      lives ONLY in the package JSON; the loader reads authority from the
//      package (package-aware skip), never from the doc surface. cite-gen +
//      managed_surfaces + the `.epr-meta` compose-gate remain the EDIT authority
//      over the doc file itself.
//   3. CID is single-sourced in eprfs (`eprfs-core::BlobCid::compute`) — this
//      module NEVER computes a doc CID in JS. Byte-identity (rule 1) is the
//      fidelity floor and needs no CID; `source.cid`, when present, is stamped by
//      `eprfs-agent`, recorded read-only, and not recomputed here.

import { basename, relative } from 'node:path';

export const AGENT_DOC_KIND = 'AgentDocPackage';

// The runtime a doc projects into is fixed by its basename: CLAUDE.md is Claude
// Code's ambient context, AGENTS.md is codex's. There is no cross-runtime fork —
// AGENTS.md is a DIFFERENT file for the same directory, not a second projection
// of one identity. So an agent-doc has exactly ONE projection, keyed by runtime.
export function runtimeForDoc(sourcePath) {
  return basename(sourcePath) === 'AGENTS.md' ? 'codex' : 'claude';
}

// Read a single top-level frontmatter scalar (`id:` / `description:`) from a doc
// that opens with a `---\n…\n---\n` block. Returns null for a frontmatter-less
// doc (most of the 142 CLAUDE.mds have none) or a missing key. Deliberately does
// NOT parse the multiline `cites:` list — those envelopes are owned by cite-gen
// and only ever travel verbatim inside `source.body`.
export function frontmatterScalar(rawBytes, key) {
  if (!rawBytes.startsWith('---\n')) return null;
  const close = rawBytes.indexOf('\n---\n', 4);
  if (close === -1) return null;
  const block = rawBytes.slice(4, close);
  for (const line of block.split('\n')) {
    const match = line.match(/^([A-Za-z0-9_-]+):(.*)$/);
    if (match && match[1] === key) {
      const value = match[2].trim();
      if (!value) return null;
      if (
        (value.startsWith('"') && value.endsWith('"')) ||
        (value.startsWith("'") && value.endsWith("'"))
      ) {
        return value.slice(1, -1);
      }
      return value;
    }
  }
  return null;
}

// Derive a human description for a doc with no `description:` frontmatter: the
// first non-empty markdown heading line, stripped of leading `#`. Falls back to
// the id. Used only by the importer; a hand-authored (flipped) package carries
// its own description.
function firstHeading(rawBytes, fallback) {
  for (const line of rawBytes.split('\n')) {
    const trimmed = line.trim();
    if (trimmed.startsWith('#')) return trimmed.replace(/^#+\s*/, '').trim() || fallback;
  }
  return fallback;
}

// Import an agent-doc from its source markdown. `source.body` is the ENTIRE raw
// file copied VERBATIM (rule 1) — frontmatter (incl. cite envelopes) AND body,
// with zero transform. `id` is the CALLER's uniqueness key (142 files share the
// basename CLAUDE.md, so id must be globally unique across adopted docs — the
// dir-derived slug the plant procedure assigns); the frontmatter `id:` (when
// present) is recorded separately as `metadata.gospelId`.
export function agentDocPackageFromSource(
  sourcePath,
  rawBytes,
  { repoRoot, id, governance, composition, composedBy, master } = {},
) {
  const relPath = relative(repoRoot, sourcePath);
  const runtime = runtimeForDoc(sourcePath);
  const gospelId = frontmatterScalar(rawBytes, 'id');
  const docId = id ?? gospelId ?? basename(sourcePath, '.md');
  const description =
    frontmatterScalar(rawBytes, 'description') ?? firstHeading(rawBytes, docId);

  const metadata = {
    id: docId,
    name: docId,
    version: '1.0.0',
    description,
    sourceRuntime: runtime, // origin: born as a Claude (CLAUDE.md) / codex (AGENTS.md) doc
    runtimeTargets: [runtime],
    docPath: relPath,
    assetRefs: [],
    governance,
  };
  if (gospelId) metadata.gospelId = gospelId;
  if (composedBy) metadata.composedBy = composedBy;
  if (master) metadata.master = master;

  const pkg = {
    apiVersion: 'elohim-agent/v1alpha1',
    kind: AGENT_DOC_KIND,
    metadata,
    // `source` (NOT `instructions`): the ENTIRE raw file VERBATIM. `cid` is
    // intentionally omitted — single-sourced from eprfs-agent, never computed in
    // JS (rule 3). Byte-identity is the fidelity floor.
    source: {
      language: 'markdown',
      path: relPath,
      body: rawBytes,
    },
    // A single projected artifact (the doc), addressed at the same path it was
    // imported from, keyed by its one runtime. No cross-runtime fork.
    projections: {
      [runtime]: { path: relPath },
    },
  };
  // The composition note — a short authored description of how this doc composes
  // / is-managed relative to the others. Owned by the package (there is nowhere
  // in the doc's own frontmatter to carry it without perturbing cite-gen).
  if (composition) pkg.composition = composition;
  return pkg;
}

// The agent-doc projector: PURE VERBATIM PASSTHROUGH. The doc bytes are the same
// regardless of anything — frontmatter (incl. cite envelopes) and body are
// emitted exactly as stored. This is the single most important safety property:
// a gospel/managed-memory doc is never reformatted, re-fenced, or marker-injected
// on projection, so cite fingerprints survive byte-identical.
export function projectAgentDoc(pkg) {
  return pkg.source.body;
}

// Agent-doc-specific verify assertions (rule 1's non-emptiness leg + the
// authority-marker-never-in-the-doc guard). The byte-identity of the projected
// fixture / runtime file is proved by the shared verifyProjectionFixture /
// verifyRuntimeProjectionIfPresent in the main file (they compare
// projectAgentDoc(pkg) === on-disk with strict `===`). This adds what is unique
// to agent-docs: a non-empty verbatim body, and that the runtime marker is NOT
// present as literal frontmatter in the doc bytes (it must live in the package).
export function verifyAgentDocPackage(pkg, { assert }) {
  assert(
    typeof pkg.source?.body === 'string' && pkg.source.body.length > 0,
    `${pkg.metadata.id} has verbatim source body`,
  );
  // Guard rule 2: a flipped agent-doc must NOT have had `master: package`
  // injected into the doc's own frontmatter. The doc surface stays cite-gen
  // authored; authority lives in the package JSON only. (We only inspect the
  // leading frontmatter block, where such a marker would sit.)
  if (pkg.metadata.master === 'package' && pkg.source.body.startsWith('---\n')) {
    const close = pkg.source.body.indexOf('\n---\n', 4);
    const fm = close === -1 ? '' : pkg.source.body.slice(4, close);
    assert(
      !/(^|\n)\s*master:\s*package\b/.test(fm),
      `${pkg.metadata.id} authority marker is NOT injected into the doc frontmatter (master lives in the package; cite-gen owns the doc surface)`,
    );
  }
}
