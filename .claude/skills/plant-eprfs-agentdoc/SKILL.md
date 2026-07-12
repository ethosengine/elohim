---
name: plant-eprfs-agentdoc
description: Plant ONE runtime-authored agent-doc (CLAUDE.md / AGENTS.md) in its elohim-native package on the eprfs layer. The package becomes the authoritative root for the gospel doc's frontmatter (cite envelopes preserved verbatim) + body; the doc file stays BYTE-IDENTICAL because the projection is verbatim passthrough, so cite-gen is untouched and the cite fingerprints survive intact. The agent-doc member of the plant-eprfs family; the most sensitive plant type (gospel / managed-memory surfaces).
metadata:
  sourceRuntime: elohim-agent
  master: package
  governance: "epr:elohim-agent/skills/plant-eprfs-agentdoc"
---
# Plant EPRFS Agent-Doc

**Plant** a runtime-authored agent-doc (`CLAUDE.md` / `AGENTS.md`) in its elohim-native package on the **eprfs** layer: the package under `.epr-meta/elohim/packages/agentdocs/<id>.json` becomes the authoritative root. Its `source.body` — the ENTIRE raw file, frontmatter (incl. cite envelopes) AND body — is the source of truth. The doc file grows from that root as a generated projection that traces back to it, **byte-for-byte identical**, content-addressed (eprfs `BlobCid`), with the composing model recorded.

This is the **agent-doc** member of the plant-eprfs family (siblings, grown one at a time: `plant-eprfs-skill`, `plant-eprfs-agent`, `plant-eprfs-hook`). It is the **most safety-sensitive** plant type: agent-docs are GOSPEL / managed-memory surfaces with existing cite tooling and `.epr-meta` compose-gate governance. A mangled CLAUDE.md or a broken cite envelope damages the memory substrate. For the shared authoring discipline see the `elohim-package-authoring` skill.

**One capability type, one target per run.** Plant ONE doc, prove it byte-identical, then the next. **Never bulk-adopt the CLAUDE.md corpus** — planting is package-driven and opt-in, one doc at a time.

## The FLIP that composes with cite-gen

Agent-docs DO flip (the package becomes `master`, like every other plant type). The flip composes WITH cite-gen instead of fighting it, and this is the whole design:

- **The doc's frontmatter is a LIVE index.** cite-gen writes `id:` + content-addressed `cites:` envelopes (`slug | desc | sha256:… | path: …`) whose fingerprints are computed from the *cited* docs' content. A generate-from-metadata projector (the skill/agent flip) could never reproduce those, and would fight cite-gen.
- **So the projector is VERBATIM PASSTHROUGH, not generate-from-metadata.** `project(package)` returns `source.body` unchanged — the entire raw file. The flip moves *authority* into the package; it rewrites **zero bytes** of the doc. cite-gen stays the sole author of the doc surface, and every cite fingerprint survives byte-identical. This is why the flip composes: the eprfs/epr-meta root is a content-addressed substrate (cites are sha256 envelopes, the package graph is BlobCid), so the package absorbing cite provenance is a natural extension, not a foreign burden.
- **The authority marker lives in the PACKAGE, never in the doc.** `metadata.master: "package"` goes in the package JSON only. The loader reads authority from the package (package-aware skip), never from the doc frontmatter — so `master: package` is **never** injected into the CLAUDE.md. cite-gen + `managed_surfaces.py` + the `.epr-meta` compose-gate remain the EDIT authority over the doc file itself.

## What planting does

- **Origin preserved.** The package keeps `metadata.sourceRuntime` (`"claude"` for CLAUDE.md, `"codex"` for AGENTS.md) — where the doc was born.
- **Authority rooted in the package.** The package gains `metadata.master: "package"`. Editing the doc directly is still done through cite-gen (the doc is the cite surface), but the package is now the authoritative composition root and the freshness anchor.
- **Doc bytes untouched.** The agent-doc projector is pure passthrough — it emits `source.body` verbatim. There is no frontmatter to generate, no body to re-wrap, no authority-marker banner injected. This is the safety property that matters most for a gospel doc and its cite envelopes.
- **The composition note.** The package carries a short authored `composition` note describing how this doc composes / is-managed relative to the others — the one piece of authored intent that has nowhere to live in the doc's own frontmatter without perturbing cite-gen.
- **Provenance recorded.** `eprfs-agent compose-graph` records the content-addressed edge from the projected doc back to its native package (`packageCid`), attributed to the model that composed it (`composedBy`). Because master stays a verbatim mirror, the projection CID **equals** the source CID — the witness is exact, not a fork.

## The floor this stands on

1. **Transform-free byte-identity.** `project(import(doc)) === doc` holds for *any* bytes with *zero* transform — import copies the raw file into `source.body`, project returns it unchanged. Strict `===` on the raw read (no trailing-newline normalization, no `trim()`, no YAML round-trip, no frontmatter re-emission). For an un-flipped adopted doc this is `verifySourceFidelity`; for a planted (flipped) doc it is `verifyRuntimeProjectionIfPresent` — `project(package) === the doc file` — both in `scripts/package-projections.mjs`, run by `elohim-agent:packages:verify`.
2. **cite-gen is UNTOUCHED.** Because the doc bytes never change on projection, `cite-gen.py --verify <doc>` resolves exactly as before and every `sha256:…` fingerprint is preserved. Planting must never trip the cite gate.
3. **CID is single-sourced in eprfs.** `source.cid`, when present, is the eprfs `BlobCid` stamped by `eprfs-agent` — never recomputed in JS. Byte-identity (floor 1) is the fidelity floor and needs no CID.

## Procedure (one doc `X`)

1. **Author the AgentDocPackage** at `.epr-meta/elohim/packages/agentdocs/<id>.json` (adoption is creating the package — there is no bulk import). Set:
   - `kind: "AgentDocPackage"`, `apiVersion: "elohim-agent/v1alpha1"`.
   - `source: { language: "markdown", path: "<repo-rel path to X>", body: "<the ENTIRE raw file, verbatim>" }`. Copy the bytes exactly — frontmatter (incl. every cite envelope) and body.
   - `metadata`: `id` (a globally-unique slug — 142 docs share the basename CLAUDE.md, so derive it from the directory, e.g. `mishpat-domain-gospel`), `name` = `id`, `version`, `description`, `sourceRuntime` (`claude`/`codex`), `runtimeTargets`, `assetRefs: []`, `master: "package"`, `composedBy: "<your model id>"`, and `governance = { "eprRef": "epr:elohim-agent/agentdocs/<id>", "policy": "capability-governance@1", "gates": ["epr-meta-resolver","elohim-agent:packages:verify"], "ledger": ".claude/data/governance-findings.jsonl" }` (hand-set — nothing generates it). Record the doc's own frontmatter `id:` as `metadata.gospelId`.
   - `composition`: a short authored note on how X composes / is-managed relative to the other docs.
   - `projections: { "<runtime>": { "path": "<repo-rel path to X>" } }` — a single projection keyed by the doc's one runtime.
2. **Regenerate projections** from the package: `node elohim/sdk/domains/elohim-agent/scripts/package-projections.mjs project --write-fixtures` then `... project --write-runtime`. For a doc this writes X back **byte-identical** to what was there (verbatim passthrough) — `git diff <X>` MUST be empty. A non-empty diff means the bytes were transformed somewhere and is a STOP condition. **If the flip cannot preserve the cite envelopes byte-identically, STOP — never ship a mangled gospel doc.**
3. **Verify package-first.** `... verify` is green: X takes the package-first path (the loader skips re-importing a `master: package` doc), and the byte-identity assertion holds for both the fixture and the runtime doc.
4. **Confirm cite-gen still resolves.** `python3 .claude/scripts/memory-kit/cite-gen.py --verify <X>` exits 0 — every cite is still content-addressed and resolvable. The flip touched nothing cite-gen cares about.
5. **Confirm the doc is byte-identical.** `git diff --exit-code <X>` MUST be clean. Planting anchors authority; it never rewrites the doc.
6. **Record composition.** `eprfs-agent compose-graph .epr-meta/elohim/packages --projections-root .epr-meta/elohim/projections --composed-by "<your model id>"` — X's node shows `kind: "AgentDocPackage"`, `master: "package"`, `packageCid` <-> projection CID (equal, since the mirror is verbatim), and `composedBy`. (Agent-doc support in the Rust compose-graph adapter may still be pending — byte-identity is the floor that needs no CID.)
7. **Commit path-scoped**, then move to the next doc. Prove one byte-identical, then the next.

## Which docs are pilot-safe

Plant **small, self-contained, leaf** gospel docs first (a domain CLAUDE.md with one or two cites), and confirm the cite gate before and after. Defer the high-fan-out root gospel surfaces (repo-root `CLAUDE.md`, the memory MEMORY.md) until the machinery has a track record — a projection regression on a root surface has the widest blast radius. Never risk the memory substrate for a plant.

## Per-runtime adapters (the surface seam)

For skills and agents, the per-runtime projection is a *frontmatter dialect* over a shared markdown body — one identity, two projections. For an agent-doc it is different: the codex-runtime analog of a directory's `CLAUDE.md` is a **different file** (`AGENTS.md`) for the same directory, not a second projection of one identity. So an agent-doc has exactly **one** projection, keyed by its one runtime (`claude` for CLAUDE.md, `codex` for AGENTS.md). There is no cross-runtime fork to unify — the runtime is fixed by the basename.

## Invariants

- **Never rewrite doc bytes on project.** The agent-doc projector is verbatim passthrough. A non-empty `git diff` on the projected doc is a bug in the projector, not an acceptable reformatting — and on a gospel doc it is a memory-substrate hazard.
- **Never inject an authority marker into the doc.** `master: package` lives in the package JSON only. The doc frontmatter stays exactly as cite-gen authored it, so the `gospel-claude-md` classifier and every cite envelope stay intact.
- **cite-gen owns the doc surface.** Any change to the doc's frontmatter (`id:`, `cites:`) goes through cite tooling, never a hand-edit and never the projector. Planting is orthogonal to authoring.
- **CID is single-sourced in eprfs** (`eprfs-core::BlobCid::compute`). Never recompute a CID in JS — call `eprfs-agent`.
- **Adoption is opt-in and one-at-a-time.** Never bulk-adopt the CLAUDE.md corpus. A doc is a source package only once its AgentDocPackage exists.
- **Identity never forks per runtime.** One doc, one root, one runtime projection.
