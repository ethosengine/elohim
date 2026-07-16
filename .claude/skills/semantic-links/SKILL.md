---
name: semantic-links
description: "Use when authoring or fixing a doc's cites: — the content-addressed citation convention (slug + desc + fingerprint + generated status/path) that survives file moves. Covers doc-roots AND gospel CLAUDE.mds. Run cite-gen; never hand-write a slug, fingerprint, or path. Triggers: \"add a cites: entry\", \"this cite is dead\", \"migrate cites\", \"what's HELD-CITE vs DEAD-CITE\", \"add concern-routing pointers to a CLAUDE.md\"."
metadata:
  sourceRuntime: claude
  master: package
  governance: "epr:elohim-agent/skills/semantic-links"
---

# Semantic-Computable Links

Doc/spec/memory/gospel cites are **content-addressed**, not path-based — so a cite survives the cited doc
moving (e.g. into `held/`). A cite is a single human-readable line:

```yaml
cites:
  - <ref-slug> | <one-sentence desc> | <fingerprint> [| status: <health hint>] [| path: <locator>]
```

- **`ref-slug`** — the target's `id:` (its permanent address; survives moves + edits). NEVER a path.
- **`desc`** — a one-sentence **relationship hint** (the epr-head envelope): *what the target is AND why THIS
  doc points at it*, so a reader/tool decides whether to follow WITHOUT resolving (progressive discovery).
  Anchor it from the citing doc's perspective — not the target's bare title (that's the weak migration default).
- **`fingerprint`** — `sha256:<…>` of the target's content-body at cite-time (a body edit drifts it → `STALE`).
- **`status:`** — OPTIONAL, **tool-managed** (`cite-propagate` stamps/clears it); absent on healthy links.
- **`path:`** — **tool-managed** (2026-06-05): the MATERIALIZED LOCATOR — a cache of the slug→path
  resolution so an agent follows a cite with a plain Read, no resolver run. Stamped at mint, refreshed by
  every propagate pass (a move self-heals). Never hand-written; never identity — slug + fingerprint stay truth.

A legacy path-string cite (a code path, an external file, no `id:` target) stays a plain path — that's fine.

**The fingerprint is a CID short-form (not a separate hash system).** `sha256:hex16` is, mathematically,
the first 16 hex of the sha2-256 digest that the target body's canonical content address wraps —
`CIDv1(raw 0x55, sha2-256(canonical_body))` (`bafkrei…`). One digest, two renderings. So the fingerprint slot
also accepts a **full CIDv1 token** (`bafk…`/`bafy…`): human-facing envelopes keep the short form, machine-facing
surfaces may seal the full CID, and verdicts handle both. Python never *encodes* a CID — the `eprfs cid <path>
--body --short` CLI is the single-source encoder. Details:
`genesis/docs/superpowers/specs/2026-07-12-cite-fingerprint-cid-convergence-design.md`.

## The graph's membership (who gets envelopes)

- **Doc roots** — `genesis/docs/` + `.claude/memory/`.
- **Gospel CLAUDE.mds** (2026-06-05) — `CLAUDE.md` files repo-wide (vendored/dot dirs pruned), membership
  OPT-IN by declaring `id:`. Concern-routing pointers in a CLAUDE.md are cites like any other.
- **NOT in the graph** — genesis/data entity docs (humans/presences/timeline/backlog — deliberately
  plain-path), code paths, URLs, `.claude/` dot-dir process gospels.

Membership is answered in ONE place — `_lib.managed_surfaces.in_cite_graph` (the edit-time registry every
hook + sweep consults). Never re-hardcode scope in a tool; that's how the 2026-06-05 gospel episode happened.

## Never hand-write a slug, fingerprint, or path — run the tool

```
cite-gen.py <target>            # emit the envelope line for a doc target (path: stamped at mint)
cite-gen.py --assign-id <doc>   # give a doc a collision-guarded id: slug
cite-gen.py --into <doc>        # convert this doc's legacy doc-cites → envelopes (+ refresh path: caches)
cite-gen.py --verify <doc>      # the dissolution gate: are all cites content-addressed + resolvable?
cite-gen.py --seal <doc>        # born-linked COMPOSITE: assign-id → --into → --verify, + flag title-default descs
cite-gen.py --seal-all          # end-of-sprint sweep: seal every GRAPH MEMBER (doc-roots + gospels) with cite debt
cite-gen.py --refresh <doc> [<ref>...]   # DELIBERATE stale-dequeue: after RE-VERIFYING claims against the
                                # drifted target, re-bless fingerprint (+ status + path). Verification is
                                # EDGE-granular — name the ref(s) you verified; bare form blesses ALL edges
cite-describe.py <doc> '{"<ref>":"<relationship hint>"}'   # enrich desc (the title default → progressive-discovery hint)
cite-propagate.py [--apply]     # corpus pass: stamp/clear status:, refresh every path: locator
```

### Deterministic enforcement (you don't have to remember)

`--seal` is the one command that makes a new doc born-linked; the discipline is wired so it isn't left to recall:

- **preHook** — `.claude/hooks/managed-surface-context.py` (PreToolUse Edit|Write) injects the surface's
  discipline + exact tooling BEFORE you edit any managed-memory surface (gospel/spec/plan/doc/memory/…),
  once per file per session. Scope comes from `_lib.managed_surfaces` — the single edit-time registry.
- **Ceremony POST-step** — `/brainstorm` and `/plan` run `cite-gen --seal <new-doc>` right after writing it.
- **postHook** — `.claude/hooks/cite-seal-signal.py` (PostToolUse Edit|Write) nudges the moment a GRAPH
  MEMBER (doc-root .md or gospel CLAUDE.md — registry-scoped) is written with un-sealed cite debt (legacy
  path-cite to an id-bearing doc, or no `id:` yet). Self-limiting: goes silent once sealed.
- **End-of-sprint** — `/shift`'s decompose-self close and the `memory-stasis-loop` `cites` dimension run
  `--seal-all` so nothing graduates into the permanent graph un-sealed. Pair with `cite-describe` for the
  title-default descs the seal flags.

`cite-gen --into` seeds each new cite's `desc` with the target's title (a placeholder). Upgrade it to a real
relationship hint with `cite-describe.py` — it sets `desc` only, preserving ref + fingerprint + status + path.
The whole corpus was enriched this way once; new cites just need the one follow-up call.

Authoring a new spec/plan via `/brainstorm` or `/plan`: write `cites:` as plain paths, then run
`cite-gen --into <doc>` once — it slug-ifies every doc-cite. The migration (`cites-migrate.py`) already
did this for the whole corpus; new docs just run `--into`.

## The audit verdicts (memory-coherence-audit.py)

- **`HELD-CITE`** — target is sequestered in `held/`. **NOT dead** — do not delete the link; it resolves
  again when the target returns (scope-tree reconciliation).
- **`DEAD-CITE`** — the slug resolves nowhere. A real dangling link.
- **`STALE-CANDIDATE`** — the target's content fingerprint drifted. This is a re-verify QUEUE: confirm the
  citing doc's claims still hold against the moved-on target, then `cite-gen --refresh <doc>` (the
  deliberate blessing — `--into` never auto-blesses drift).
- **`CITE-FORMAT-CANDIDATE`** — a legacy doc path-string whose target HAS an `id:` (migratable); run `--into`.

## Key files

- `.claude/scripts/_lib/cite_graph.py` — slug / fingerprint / envelope / verdict / path-materialization primitives.
- `.claude/scripts/_lib/managed_surfaces.py` — the edit-time registry: surface classes → discipline + tooling +
  graph membership (single source of scope truth for hooks and sweeps).
- `.claude/scripts/memory-kit/cite-gen.py` — author / migrate / verify / refresh one doc.
- `.claude/scripts/memory-kit/cite-describe.py` — enrich a doc's cite descriptions (title default → relationship hint).
- `.claude/scripts/memory-kit/cites-migrate.py` — one-time corpus migration (assign-id + --into).
- `.claude/scripts/memory-kit/cite-propagate.py` — materialize the `status:` hint + `path:` locator (the self-describing edge).
- Spec: `genesis/docs/superpowers/specs/2026-06-02-semantic-computable-links-design.md` (§9.1 = the 2026-06-05
  amendment) + `genesis/docs/superpowers/specs/2026-06-05-managed-surface-edit-discipline-design.md`.
