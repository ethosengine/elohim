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
import { isDeepStrictEqual } from 'node:util';

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

// Scan settings.json `hooks.<Event>[].hooks[]` for every entry whose command
// references `<hookRelPath>` (the `.claude/hooks/<name>.py`), and return the
// reconstructable registration fragments in settings order. Returns null when
// the hook is UNREGISTERED — an unregistered hook is inert by design, and the package
// records that fact (registration: null) rather than inventing wiring.
//
// READ-ONLY: this only reads settings; nothing here writes it.
export function registrationsFromSettings(settings, hookRelPath) {
  const hooks = settings?.hooks;
  if (!hooks || typeof hooks !== 'object') return null;
  const registrations = [];
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
          // Omit absent flags so legacy single-entry packages retain their bytes.
          if (typeof entry.async === 'boolean') reg.async = entry.async;
          if (typeof entry.asyncRewake === 'boolean') reg.asyncRewake = entry.asyncRewake;
          registrations.push(reg);
        }
      }
    }
  }
  return registrations.length ? registrations : null;
}

// Compatibility API: existing callers still receive only the first match.
export function registrationFromSettings(settings, hookRelPath) {
  return registrationsFromSettings(settings, hookRelPath)?.[0] ?? null;
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
    async: reg.async ?? false,
    asyncRewake: reg.asyncRewake ?? false,
  };
}

function normalizedRegistrations(registrations) {
  const list = registrations == null
    ? []
    : Array.isArray(registrations) ? registrations : [registrations];
  return list.map(normalizeRegistration).sort((a, b) => {
    for (const key of ['event', 'matcher', 'command']) {
      const order = (a?.[key] ?? '').localeCompare(b?.[key] ?? '');
      if (order) return order;
    }
    // Duplicate commands can have different timeouts/flags; keep their comparison
    // independent of settings order without losing duplicate registrations.
    return JSON.stringify(a).localeCompare(JSON.stringify(b));
  });
}

export function registrationsCoherent(a, b) {
  return isDeepStrictEqual(normalizedRegistrations(a), normalizedRegistrations(b));
}

// Import a hook from its executable source. The body is copied VERBATIM (rule 1);
// no transform of any kind is applied to the bytes.
export function hookPackageFromSource(sourcePath, rawBytes, settings, { repoRoot, governance }) {
  const id = basename(sourcePath, '.py');
  const relPath = relative(repoRoot, sourcePath);
  const description = firstDocstringLine(rawBytes) ?? id;
  const registrations = registrationsFromSettings(settings, relPath);
  const first = registrations?.[0];
  const registration = registrations?.length === 1 ? first : registrations;

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
      event: first?.event ?? null,
      matcher: first?.matcher ?? null,
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

function requiredHarnessCapabilities(registration) {
  if (registration?.event === 'Stop' && registration.asyncRewake === true) {
    return ['async-rewake-wakes-idle'];
  }
  if (registration?.event !== 'Notification') return [];
  const notifications = [
    ['permission_prompt', 'dialog-displayed-notification'],
    ['idle_prompt', 'idle-prompt-timing'],
  ];
  const matcher = registration.matcher;
  if (!matcher || matcher === '*') return notifications.map(([, capability]) => capability);
  try {
    const pattern = new RegExp(matcher);
    return notifications.filter(([event]) => pattern.test(event)).map(([, capability]) => capability);
  } catch {
    // An uninterpretable matcher cannot establish that either notification is excluded.
    return notifications.map(([, capability]) => capability);
  }
}

// Advisory only, like capability-unbound for agents. A runtime's support must
// come from its own table, never from another runtime sharing the hook bytes.
export function reportHookCapabilityWarnings(pkg, agentRuntimes, warn = console.warn) {
  if (pkg.kind !== HOOK_KIND) return;
  for (const runtime of Object.keys(pkg.projections)) {
    const runtimeId = runtime === 'claude' ? 'claude-code' : runtime;
    const capabilities = agentRuntimes?.runtimes?.find((entry) => entry.id === runtimeId)
      ?.harnessCapabilities ?? {};
    const registrations = [
      ...normalizedRegistrations(runtime === 'claude' ? pkg.registration : null),
      ...normalizedRegistrations(pkg.registrations?.[runtime]),
      ...normalizedRegistrations(pkg.metadata.intendedRegistrations),
    ];
    const required = new Set(registrations.flatMap(requiredHarnessCapabilities));
    for (const capability of required) {
      const status = capabilities[capability]?.status ?? 'unsupported';
      if (status !== 'yes') {
        warn(
          `WARN: harness-capability-unsupported: ${pkg.metadata.id} runtime=${runtimeId} capability=${capability} status=${status}`,
        );
      }
    }
  }
}

// Hook-specific verify assertions (rules 1-2). The byte-identity of the
// projected fixture/runtime file is proved by the shared verifyProjectionFixture
// / verifyRuntimeProjectionIfPresent in the main file (they compare
// projectHook(pkg) === on-disk with strict `===`). This function adds what is
// unique to hooks: a non-empty verbatim body and the READ-ONLY registration
// reconcile. It NEVER writes settings.json.
export async function verifyHookPackage(pkg, { assert, settings, codexSettings, lodge }) {
  assert(
    typeof pkg.source?.body === 'string' && pkg.source.body.length > 0,
    `${pkg.metadata.id} has verbatim source body`,
  );

  assert(
    JSON.stringify([...pkg.metadata.runtimeTargets].sort()) ===
      JSON.stringify(Object.keys(pkg.projections).sort()),
    `${pkg.metadata.id} hook projection runtimes match runtimeTargets`,
  );
  if (pkg.registrations) {
    assert(
      JSON.stringify(Object.keys(pkg.registrations).sort()) ===
        JSON.stringify(Object.keys(pkg.projections).sort()),
      `${pkg.metadata.id} hook registrations cover every declared runtime`,
    );
    assert(
      registrationsCoherent(pkg.registration, pkg.registrations.claude),
      `${pkg.metadata.id} legacy Claude registration matches its runtime registration`,
    );
  }
  for (const [runtime, projection] of Object.entries(pkg.projections)) {
    const config = runtime === 'claude' ? settings : codexSettings;
    const live = registrationsFromSettings(config, projection.path);
    const recorded =
      pkg.registrations?.[runtime] ?? (runtime === 'claude' ? pkg.registration : null);
    assert(
      runtime === 'claude' || Object.hasOwn(pkg.registrations ?? {}, runtime),
      `${pkg.metadata.id} ${runtime} registration is recorded`,
    );
    const coherent = registrationsCoherent(live, recorded);
    assert(
      coherent,
      `${pkg.metadata.id} recorded ${runtime} registration matches live config (read-only reconcile)`,
    );
    if (!coherent && lodge) {
      await lodge({
        assertionClass: runtime === 'claude' ? 'registration-drift' : `${runtime}-registration-drift`,
        detail:
          `hook registration drift: recorded registration for ${projection.path} ` +
          `does not match the live ${runtime} config entry (reconcile is read-only; ` +
          `resolve by re-recording the package registration, never by auto-editing config)`,
      });
    }
  }
}
