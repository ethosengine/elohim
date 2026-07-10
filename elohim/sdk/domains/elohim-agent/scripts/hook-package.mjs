// hook-package.mjs — the HookPackage sibling module for package-projections.mjs.
//
// Hooks are a NEW artifact class: executable CODE + a settings.json registration,
// NOT markdown + frontmatter. This module owns everything hook-specific — the
// importer (code → package), the VERBATIM projector (package → code, byte-for-
// byte), the registration recorder/reconciler, and the hook verify assertions —
// so the markdown-centric main file stays uncluttered and routes by `kind`.
//
// SAFETY (a mangled hook or a botched settings.json edit can wedge the whole
// PreToolUse gating toolchain — these are load-bearing, do NOT relax them):
//   1. Byte-identity is TRANSFORM-FREE and STRICT. import copies raw bytes into
//      `source.body`; project returns `source.body` UNCHANGED. project(import(x))
//      === x for ANY bytes, strict `===` on the raw read — no trailing-newline
//      normalization, no trim(), no YAML round-trip. `projectHook` is pure
//      passthrough for exactly this reason.
//   2. NEVER auto-write `.claude/settings.json`. Registration is RECORDED into
//      the package and RECONCILED READ-ONLY: verify WARNs (lodges a governance
//      finding) if the recorded registration diverges from the live settings.json
//      but MUST NEVER modify settings.json. Planting a hook anchors its code; it
//      does not rewrite its wiring.
//   3. The executable `.py` is never mutated to carry an authority marker — no
//      banner, no comment. Discoverability lives in the package + compose-graph
//      + the `.claude/hooks/.epr-meta` note, not in the code bytes.
//   4. CID is single-sourced in eprfs (`eprfs-core::BlobCid::compute`) — this
//      module NEVER computes a code CID in JS. Byte-identity (rule 1) is the
//      fidelity floor and needs no CID; `source.cid`, when present, is stamped
//      by `eprfs-agent`, recorded read-only, and not recomputed here.

import { basename, relative } from 'node:path';

export const HOOK_KIND = 'HookPackage';

// Derive a human description from a hook's module docstring first line. Hooks
// carry no declared description; the triple-quoted docstring after the shebang
// is the best available source. Falls back to the id when absent/terse.
export function firstDocstringLine(rawBytes) {
  const open = rawBytes.indexOf('"""');
  if (open === -1) return null;
  const close = rawBytes.indexOf('"""', open + 3);
  if (close === -1) return null;
  const doc = rawBytes.slice(open + 3, close);
  for (const line of doc.split('\n')) {
    const trimmed = line.trim();
    if (trimmed) return trimmed;
  }
  return null;
}

// Scan settings.json `hooks.<Event>[].hooks[]` for the entry whose command
// references `<hookRelPath>` (the `.claude/hooks/<name>.py`), and return the
// reconstructable registration fragment. Returns null when the hook is
// UNREGISTERED — an unregistered hook is inert by design, and the package
// records that fact (registration: null) rather than inventing wiring.
//
// READ-ONLY: this only reads settings; nothing here writes it.
export function registrationFromSettings(settings, hookRelPath) {
  const hooks = settings?.hooks;
  if (!hooks || typeof hooks !== 'object') return null;
  // Match by the repo-relative hook path appearing in the command string; the
  // command uses `$CLAUDE_PROJECT_DIR/.claude/hooks/<name>.py`, so the relative
  // path is a stable substring regardless of the env-var prefix.
  const needle = hookRelPath;
  for (const [event, groups] of Object.entries(hooks)) {
    if (!Array.isArray(groups)) continue;
    for (const group of groups) {
      const entries = Array.isArray(group?.hooks) ? group.hooks : [];
      for (const entry of entries) {
        const command = typeof entry?.command === 'string' ? entry.command : '';
        if (command.includes(needle)) {
          const reg = {
            event,
            matcher: group.matcher ?? null,
            command,
            // Timeout is in SECONDS (normalized 2026-07-02 — values had been
            // mis-written as milliseconds). Record it verbatim; the reconcile
            // compares in the same unit so it never lodges false drift.
            timeout: typeof entry.timeout === 'number' ? entry.timeout : null,
          };
          if (entry.type) reg.type = entry.type;
          return reg;
        }
      }
    }
  }
  return null;
}

// Normalize a registration for a stable deep-equal (key order independent).
function normalizeRegistration(reg) {
  if (!reg || typeof reg !== 'object') return null;
  return {
    event: reg.event ?? null,
    matcher: reg.matcher ?? null,
    command: reg.command ?? null,
    timeout: typeof reg.timeout === 'number' ? reg.timeout : null,
    type: reg.type ?? null,
  };
}

export function registrationsCoherent(a, b) {
  return JSON.stringify(normalizeRegistration(a)) === JSON.stringify(normalizeRegistration(b));
}

// Import a hook from its executable source. The body is copied VERBATIM (rule 1);
// no transform of any kind is applied to the bytes.
export function hookPackageFromSource(sourcePath, rawBytes, settings, { repoRoot, governance }) {
  const id = basename(sourcePath, '.py');
  const relPath = relative(repoRoot, sourcePath);
  const description = firstDocstringLine(rawBytes) ?? id;
  const registration = registrationFromSettings(settings, relPath);

  return {
    apiVersion: 'elohim-agent/v1alpha1',
    kind: HOOK_KIND,
    metadata: {
      id,
      name: id,
      version: '1.0.0',
      description,
      sourceRuntime: 'claude', // origin: born as a Claude Code hook
      runtimeTargets: ['claude'],
      // Wiring facets mirrored from the live registration (null when inert).
      event: registration?.event ?? null,
      matcher: registration?.matcher ?? null,
      interpreter: 'python3',
      language: 'python',
      assetRefs: [],
      governance,
    },
    // `source` (NOT `instructions`): the executable body VERBATIM. `cid` is
    // intentionally omitted here — it is single-sourced from eprfs-agent, never
    // computed in JS (rule 4). Byte-identity is the fidelity floor.
    source: {
      language: 'python',
      interpreter: 'python3',
      path: relPath,
      body: rawBytes,
    },
    // The reconstructable settings.json fragment — RECORDED, reconciled
    // read-only, NEVER auto-written back (rule 2).
    registration,
    // A hook has a single projected artifact (the code), addressed at the same
    // path it was imported from. No codex projection today — a codex hook target
    // is deferred (the registration adapter's second backend).
    projections: {
      claude: { path: relPath },
    },
  };
}

// The hook projector: PURE VERBATIM PASSTHROUGH. Runtime-agnostic — the code
// body is the same bytes regardless of target. This is the single most important
// safety property: a code file that gates tool use is never reformatted,
// re-indented, or banner-injected on projection.
export function projectHook(pkg) {
  return pkg.source.body;
}

// Hook-specific verify assertions (rules 1-2). The byte-identity of the
// projected fixture/runtime file is proved by the shared verifyProjectionFixture
// / verifyRuntimeProjectionIfPresent in the main file (they compare
// projectHook(pkg) === on-disk with strict `===`). This function adds what is
// unique to hooks: a non-empty verbatim body and the READ-ONLY registration
// reconcile. It NEVER writes settings.json.
export async function verifyHookPackage(pkg, { assert, settings, lodge }) {
  assert(
    typeof pkg.source?.body === 'string' && pkg.source.body.length > 0,
    `${pkg.metadata.id} has verbatim source body`,
  );

  const live = registrationFromSettings(settings, pkg.projections.claude.path);
  const recorded = pkg.registration ?? null;
  const coherent = registrationsCoherent(live, recorded);
  assert(
    coherent,
    `${pkg.metadata.id} recorded registration matches live settings.json (read-only reconcile — settings.json is NEVER auto-written)`,
  );
  if (!coherent && lodge) {
    await lodge({
      assertionClass: 'registration-drift',
      detail:
        `hook registration drift: recorded registration for ${pkg.projections.claude.path} ` +
        `does not match the live .claude/settings.json entry (reconcile is read-only; ` +
        `resolve by re-recording the package registration, never by auto-editing settings.json)`,
    });
  }
}
