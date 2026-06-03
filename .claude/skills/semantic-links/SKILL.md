---
name: semantic-links
description: Use when authoring or fixing a doc's cites: — the content-addressed citation convention (slug + desc + fingerprint) that survives file moves. Run cite-gen; never hand-write a slug or fingerprint. Triggers: "add a cites: entry", "this cite is dead", "migrate cites", "what's HELD-CITE vs DEAD-CITE".
---

# Semantic-Computable Links

Doc/spec/memory cites are **content-addressed**, not path-based — so a cite survives the cited doc moving
(e.g. into `held/`). A cite is a single human-readable line:

```yaml
cites:
  - <ref-slug> | <one-sentence desc> | <fingerprint> [| status: <health hint>]
```

- **`ref-slug`** — the target's `id:` (its permanent address; survives moves + edits). NEVER a path.
- **`desc`** — a one-sentence **relationship hint** (the epr-head envelope): *what the target is AND why THIS
  doc points at it*, so a reader/tool decides whether to follow WITHOUT resolving (progressive discovery).
  Anchor it from the citing doc's perspective — not the target's bare title (that's the weak migration default).
- **`fingerprint`** — `sha256:<…>` of the target's content-body at cite-time (a body edit drifts it → `STALE`).
- **`status:`** — OPTIONAL, **tool-managed** (`cite-propagate` stamps/clears it); absent on healthy links.

A legacy path-string cite (a code path, an external file, no `id:` target) stays a plain path — that's fine.

## Never hand-write a slug or fingerprint — run the tool

```
cite-gen.py <target>            # emit the envelope line for a doc target
cite-gen.py --assign-id <doc>   # give a doc a collision-guarded id: slug
cite-gen.py --into <doc>        # convert this doc's legacy doc-cites → envelopes
cite-gen.py --verify <doc>      # the dissolution gate: are all cites content-addressed + resolvable?
cite-gen.py --seal <doc>        # born-linked COMPOSITE: assign-id → --into → --verify, + flag title-default descs
cite-gen.py --seal-all          # end-of-sprint sweep: seal every doc-root .md carrying un-sealed cite debt
cite-describe.py <doc> '{"<ref>":"<relationship hint>"}'   # enrich desc (the title default → progressive-discovery hint)
```

### Deterministic enforcement (you don't have to remember)

`--seal` is the one command that makes a new doc born-linked; the discipline is wired so it isn't left to recall:

- **Ceremony POST-step** — `/brainstorm` and `/plan` run `cite-gen --seal <new-doc>` right after writing it.
- **postHook** — `.claude/hooks/cite-seal-signal.py` (PostToolUse Edit|Write) nudges the moment a doc-root `.md`
  is written with un-sealed cite debt (legacy path-cite to an id-bearing doc, or no `id:` yet). Self-limiting:
  goes silent once sealed.
- **End-of-sprint** — `/shift`'s decompose-self close and the `memory-stasis-loop` `cites` dimension run
  `--seal-all` so nothing graduates into the permanent graph un-sealed. Pair with `cite-describe` for the
  title-default descs the seal flags.

`cite-gen --into` seeds each new cite's `desc` with the target's title (a placeholder). Upgrade it to a real
relationship hint with `cite-describe.py` — it sets `desc` only, preserving ref + fingerprint + status. The
whole corpus was enriched this way once; new cites just need the one follow-up call.

Authoring a new spec/plan via `/brainstorm` or `/plan`: write `cites:` as plain paths, then run
`cite-gen --into <doc>` once — it slug-ifies every doc-cite. The migration (`cites-migrate.py`) already
did this for the whole corpus; new docs just run `--into`.

## The audit verdicts (memory-coherence-audit.py)

- **`HELD-CITE`** — target is sequestered in `held/`. **NOT dead** — do not delete the link; it resolves
  again when the target returns (scope-tree reconciliation).
- **`DEAD-CITE`** — the slug resolves nowhere. A real dangling link.
- **`STALE-CANDIDATE`** — the target's content fingerprint drifted; re-verify the lesson, then re-`--into`.
- **`CITE-FORMAT-CANDIDATE`** — a legacy doc path-string whose target HAS an `id:` (migratable); run `--into`.

## Key files

- `.claude/scripts/_lib/cite_graph.py` — slug / fingerprint / envelope / verdict primitives.
- `.claude/scripts/memory-kit/cite-gen.py` — author / migrate / verify one doc.
- `.claude/scripts/memory-kit/cite-describe.py` — enrich a doc's cite descriptions (title default → relationship hint).
- `.claude/scripts/memory-kit/cites-migrate.py` — one-time corpus migration (assign-id + --into).
- `.claude/scripts/memory-kit/cite-propagate.py` — materialize the `status:` hint (the self-describing edge).
- Spec: `genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md`.
