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

// ── Codex agent-doc adapter (the DERIVED-runtime projection) ────────────────
//
// A doc's NATIVE runtime (metadata.sourceRuntime) projects VERBATIM — the whole
// point of the plant floor. But a doc MAY additionally declare a DERIVED-runtime
// projection: e.g. a CLAUDE.md (sourceRuntime: claude) that also projects an
// AGENTS.md for codex. Codex has no PreToolUse hook system, so it cannot receive
// the edit-time governance contract Claude gets from hooks. The derived codex
// projection therefore carries a GENERATED governance-contract PREAMBLE the
// adapter owns, followed by the SAME gospel body verbatim (single-sourced from
// this package's source.body, so a CLAUDE.md edit flows into AGENTS.md — they
// cannot drift). The preamble is repo-wide governance (identical for any codex
// root agent-doc), so it lives in code, not the package; parameterize per-doc
// only if a second codex agent-doc ever needs a different contract.
//
// Byte-identity still holds per runtime: project(pkg, 'codex') === the AGENTS.md
// on disk exactly (adapter emits it, --write-runtime writes it), so the standing
// verifyProjectionFixture / verifyRuntimeProjectionIfPresent gates cover it.
export function codexAgentDocPreamble(pkg) {
  const nativePath = pkg.source?.path ?? 'CLAUDE.md';
  return `# AGENTS.md — Codex Governance Contract (Elohim Protocol)

This is Codex's root gospel. The gospel body below the horizontal rule is the
SAME text as \`${nativePath}\`, projected verbatim from the \`${pkg.metadata.id}\`
package — the two cannot drift. THIS preamble is the codex-localized governance
contract: what Claude Code receives from edit-time hooks but Codex, having no
PreToolUse hook system, cannot. Read it first. It binds you *morally* where it
cannot bind you *mechanically*, and it names the gates that bind you
mechanically regardless of harness.

## 1. Editing agentic surfaces is package-first

The skills, subagents, hooks, and agent-docs (\`.claude/*\`, \`.codex/*\`,
\`CLAUDE.md\`, \`AGENTS.md\`) are PROJECTIONS of Elohim-native packages under
\`.epr-meta/elohim/packages/\`. Two disciplines coexist; the marker decides which:

- **A package marked \`metadata.master: "package"\` (planted) is authoritative.**
  Edit its package JSON, then regenerate ONLY that surface and re-verify:
  \`node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project --write-fixtures --write-runtime --only <Kind:id>\`
  then \`… verify\`. NEVER hand-edit the projected \`.claude\`/\`.codex\` file — the
  next projection clobbers it and \`verify\` reds on the drift.
- **A Claude-sourced package that is NOT planted (no \`master\`) is source-first.**
  Edit its \`.claude/*\` source directly, then re-import:
  \`… project --write-fixtures\` (or \`elohim-agent:packages:write\`). The package is
  a certified mirror; the fidelity gate proves \`project(import(source)) === source\`.

If unsure which a surface is, read its package's \`metadata.master\`. When in
doubt, do not edit the projection — edit the package or the source.

## 2. The \`.epr-meta\` compose-gate binds you morally

Claude enforces directory-local governance at edit time via a PreToolUse hook;
Codex has no such hook, so the SAME rules bind you by contract instead of by
mechanism. Before writing a file, honor the nearest \`.epr-meta\` ancestor
(the cascade: closest manifest wins, root \`/.epr-meta/manifest.md\` is the base):
a \`specs/\` doc requires lifecycle frontmatter; \`elohim/\` carries an
interface-first-reuse advisory (reuse the existing trait/type before adding a
parallel one); code directories may carry YAGNI/over-engineering guards. A
manifest that would DENY a Claude edit should stop you too — you just won't get
a prompt. Treat a missing prompt as your own responsibility, not permission.

## 3. Gates that bind Codex mechanically (harness-independent)

These run outside any harness and WILL fail your push or CI:

- **\`.husky/pre-push\`** — project-detected quality gates (lint / format:check /
  typecheck / tests), plus \`sweettest-check\` when the push targets \`dev\`/\`main\`.
  Do not bypass with \`--no-verify\` unless the gates already ran green.
- **\`pnpm run elohim-agent:packages:verify\`** — projection/governance drift. If
  you touched any agentic surface or its package, this must be GREEN before commit.
- **cite-gen** owns \`cites:\` frontmatter. NEVER hand-write a slug, path, or
  \`sha256:\` fingerprint — run \`python3 .claude/scripts/memory-kit/cite-gen.py\`.
  A hand-edited fingerprint fails the cite gate.

## 4. Rails (non-negotiable)

- **Never \`git add -A\` / \`git commit -a\`** — the worktree is shared across
  concurrent sessions. Stage path-scoped: \`git add <explicit paths>\`.
- **Never push without green gates**, and never amend or force-push shared history.
- **Native (non-WASM) cargo needs \`CARGO_TARGET_DIR\`** at the pool slot per the
  disk policy; WASM/DNA workspaces stay plain cargo.
- **\`RUSTFLAGS\` gotcha:** the environment sets
  \`RUSTFLAGS=--cfg getrandom_backend="custom"\` for Holochain WASM. Native builds
  (doorway, steward/node) need \`RUSTFLAGS=""\`; \`elohim/elohim-storage\` keeps the
  custom backend flag.

---

`;
}

// The agent-doc projector. For a doc's NATIVE runtime (claude for a CLAUDE.md)
// it is PURE VERBATIM PASSTHROUGH — the doc bytes are emitted exactly as stored
// (frontmatter incl. cite envelopes + body), never reformatted or marker-injected,
// so cite fingerprints survive byte-identical. For a DERIVED runtime (a codex
// AGENTS.md projected from a claude-sourced package) it emits the generated
// governance-contract preamble followed by the verbatim gospel body. `runtime` is
// optional: absent (or equal to the native sourceRuntime) ⇒ verbatim passthrough,
// preserving every existing single-projection call site.
export function projectAgentDoc(pkg, runtime) {
  const body = pkg.source.body;
  const native = pkg.metadata?.sourceRuntime;
  if (runtime && runtime !== native && runtime === 'codex') {
    return `${codexAgentDocPreamble(pkg)}${body}`;
  }
  return body;
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
