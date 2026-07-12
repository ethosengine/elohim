---
name: plant-eprfs-hook
description: Plant ONE runtime-authored hook in its elohim-native package on the eprfs layer (the package becomes the authoritative source-of-truth for the hook's executable code AND its settings.json registration; the .claude/hooks file stays byte-identical and the wiring is recorded, never rewritten). The hook member of the plant-eprfs family; verbatim-passthrough fidelity, registration reconciled read-only.
metadata:
  runtime: codex
  sourceRuntime: elohim-agent
  master: package
  sourcePath: .epr-meta/elohim/packages/skills/plant-eprfs-hook.json
  packageKind: SkillPackage
governance: "epr:elohim-agent/skills/plant-eprfs-hook"
---
# Plant EPRFS Hook

**Plant** a runtime-authored hook in its elohim-native package on the **eprfs** layer: the package under `.epr-meta/elohim/packages/hooks/` becomes the authoritative root. Its `source.body` is the source of truth for the hook's executable code; its `registration` block is the source of truth for how the hook is wired. The `.claude/hooks/<name>` file grows from that root as a generated projection that traces back to it — **byte-for-byte identical**, content-addressed (eprfs `BlobCid`), with the composing model recorded.

This is the **hook** member of the plant-eprfs family (siblings, grown one at a time: `plant-eprfs-skill`, `plant-eprfs-agent`, `plant-eprfs-claude-md`). Unlike its siblings, a hook is **executable code, not markdown with frontmatter** — so planting it is a governance/provenance move that leaves the executable bytes untouched. For the shared authoring discipline see the `elohim-package-authoring` skill.

**One capability type, one target per run.** Plant ONE hook, prove it, then the next. This is the most safety-sensitive plant type: a mangled hook or a botched `settings.json` edit can wedge the whole `PreToolUse` gating toolchain.

## What planting does

- **Origin preserved.** The package keeps `metadata.sourceRuntime` (`"claude"`) — where the hook was born.
- **Authority rooted in the package.** The package gains `metadata.master: "package"`. Editing `.claude/hooks/<name>` directly is now drift; the root is the package's `source.body`.
- **Executable bytes untouched.** The hook projector is pure passthrough — it emits `source.body` verbatim. Planting a hook changes *authority*, not a single byte of gating code. There is no frontmatter to generate, no body to re-wrap, no authority-marker banner injected into the code. This is the safety property that matters most for code that runs on `PreToolUse`.
- **Registration recorded, never rewritten.** The package's `registration` block mirrors the hook's `.claude/settings.json` entry (event, matcher, command, timeout). Planting records it; it does NOT auto-write `settings.json`. Wiring is a human-reviewed surface — a bad settings.json merge can silently un-wire or double-wire a gate.
- **Provenance recorded.** `eprfs-agent compose-graph` records the content-addressed edge from the projected code back to its native package (`packageCid`), attributed to the model that composed it (`composedBy`).

## The floor this stands on

1. **Transform-free byte-identity.** `project(import(source)) === source` holds for *any* bytes with *zero* transform — import copies raw bytes into `source.body`, project returns them unchanged. Strict `===` on the raw read (no trailing-newline normalization, no trim(), no YAML round-trip). Proven by `verifySourceFidelity` (for un-flipped hooks) and by `verifyRuntimeProjectionIfPresent` — `project(package) === .claude/hooks/<name>` — for a planted (flipped) hook, both in `scripts/package-projections.mjs`, run by `elohim-agent:packages:verify`.
2. **Registration reconcile is a READ-ONLY gate.** `registration` deep-equals the live `.claude/settings.json` entry, or a governance finding is lodged. It NEVER mutates `settings.json`.
3. **CID is single-sourced in eprfs.** `source.cid`, when present, is the eprfs `BlobCid` stamped by `eprfs-agent` — never recomputed in JS. Byte-identity (floor 1) is the fidelity floor and needs no CID.

## Procedure (one hook `X`)

1. **Confirm the package is faithful.** `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs verify` is green — `X`'s package round-trips its `.claude/hooks/X` losslessly. If `X` has no package yet, import it first (the importer copies the raw bytes into `source.body` verbatim and records the registration from `settings.json`).
2. **Root authority** in `.epr-meta/elohim/packages/hooks/X.json`: keep `metadata.sourceRuntime`; add `metadata.master: "package"`; ensure `metadata.governance = { "eprRef": "epr:elohim-agent/hooks/X", "policy": "capability-governance@1", "gates": ["epr-meta-resolver","elohim-agent:packages:verify"], "ledger": ".claude/data/governance-findings.jsonl" }` (hand-set — nothing generates it for a native/flipped package); record `metadata.composedBy: "<your model id>"`. Confirm `registration` matches the live `.claude/settings.json` entry for `X`.
3. **Regenerate projections** from the package: `... project --write-fixtures` then `... project --write-runtime`. For a hook this writes `X`'s code back **byte-identical** to what was there (verbatim passthrough) — `git diff .claude/hooks/X` MUST be empty. A non-empty diff means the code was reformatted somewhere and is a STOP condition.
4. **Verify package-first.** `... verify` is green: `X` takes the package-first path (the loader skips re-importing a `master: package` hook), projection freshness holds byte-for-byte, and the registration is coherent with `settings.json` (no `registration-drift` finding lodged).
5. **Confirm `settings.json` is UNCHANGED.** `git diff .claude/settings.json` MUST be empty across the whole operation. Planting anchors the code; it never rewrites the wiring.
6. **Record composition.** `eprfs-agent compose-graph .epr-meta/elohim/packages --projections-root .epr-meta/elohim/projections --composed-by "<your model id>"` — `X`'s node shows `kind: "HookPackage"`, `master: "package"`, `packageCid` <-> projection CID, and `composedBy`. (Hook support in the Rust compose-graph adapter may still be pending — byte-identity is the floor that needs no CID.)
7. **Smoke the live hook.** Feed the hook a synthetic event on stdin (`echo '<event json>' | python3 .claude/hooks/X`) and confirm it still exits 0 and behaves — because the bytes are identical this MUST be unchanged, but prove it for anything on the gating path.
8. **Commit path-scoped**, then move to the next hook. Prove one, then the next.

## Which hooks are pilot-safe

Plant advisory and `PostToolUse` hooks FIRST — they structurally cannot block a tool call. Defer deny-capable `PreToolUse` gates (`epr-meta-resolver`, `cargo-disk-guard`, sensitive-file guards) until the machinery has a track record: a projection regression on a deny-capable gate could wedge all Edit/Write/Bash. Never risk the gating toolchain for a plant.

## Per-runtime adapters (the wiring seam)

For skills and agents, the per-runtime projection is a *frontmatter dialect* over a shared markdown body. For a hook it is different: the **code body is runtime-agnostic** (a program that reads a JSON event on stdin), and what localizes per runtime is the **registration** — the `.claude/settings.json` fragment for Claude Code; a codex hook-config fragment for codex (deferred). The native package holds the runtime-agnostic code plus intent; each runtime adapter localizes only the *wiring*. **The identity (name/id) and the code never fork per runtime** — one hook, one root, projected into whichever runtimes register it.

## Invariants

- **Never rewrite executable bytes on project.** The hook projector is verbatim passthrough. A non-empty `git diff` on the projected code is a bug in the projector, not an acceptable reformatting.
- **Never auto-write `settings.json`.** Registration reconcile is READ-ONLY; drift lodges a governance finding for a human to resolve. Auto-rewiring gating hooks is out of scope.
- **Never mutate the executable to carry an authority marker.** No comment banner in the `.py`. Authority lives in the package (`master: package`); discoverability lives in the compose-graph and the `.claude/hooks/.epr-meta` note.
- **CID is single-sourced in eprfs** (`eprfs-core::BlobCid::compute`). Never recompute a CID in JS — call `eprfs-agent`.
- **A hook is inert until registered.** Registration is the review surface — planting does not register, and an unregistered planted hook is dead code by design.
- **Identity never forks per runtime.** One capability, one root, many registrations.
