# Design Assets

Implementation artifacts for the Elohim Protocol design system.

## What this is

This directory holds the **build product** of `genesis/graphos/elohim-protocol-design-spec.md` — the spec round-tripped through [claude.ai/design](https://claude.ai/design) and exported as a handoff bundle.

```
genesis/graphos/
├── elohim-protocol-design-spec.md    ← canonical spec (source of truth)
├── vocabulary.md                     ← canonical vocabulary swap rules
├── fonts/                            ← canonical font files (.ttf)
└── design-assets/
    ├── README.md                     ← you are here
    ├── raw/                          ← claude.ai/design output (regenerable)
    └── curated/                      ← hand-corrected overrides
```

## raw/ — the round-trip output

Verbatim extraction of the design tool's tarball. Contains:

- `raw/README.md` — bundle's "coding agents read this first"
- `raw/project/README.md` — design system canonical README
- `raw/project/SKILL.md` — agent skill manifest
- `raw/project/colors_and_type.css` — design tokens (the most important file)
- `raw/project/assets/` — logo PNGs
- `raw/project/preview/` — 23 component preview HTML cards
- `raw/project/ui_kits/lamad/` — Lamad reference UI kit (JSX)
- `raw/project/elohim-state-icons/` — state icons (see Known issues)
- `raw/project/uploads/` — reference images attached during the design session
- `raw/chats/` — 4 chat transcripts showing intent and iteration

**Treat raw/ as regenerable.** If we re-export from claude.ai/design, the new bundle replaces this directory. Any hand-corrections belong in `curated/`.

### Regenerating

1. Download the latest tarball from claude.ai/design (the project URL produces a `*-handoff.tar.gz`)
2. `rm -rf raw/` then `rsync -a --exclude='fonts/' <extracted>/elohim-protocol-design-system/ raw/`
3. Diff to see what the design tool changed

### Fonts are de-duplicated

The bundle ships `.ttf` files inside `project/fonts/` and `project/elohim-state-icons/fonts/`. Both are duplicates of canonical fonts at `genesis/graphos/fonts/`. They are excluded on extraction (see `.gitignore`) to avoid bloat and drift. Preview HTML files reference fonts via relative paths; if you need to render previews locally, symlink or copy the canonical fonts into the expected paths.

## curated/ — hand-corrected overrides

When the round-trip fails to honor design intent (sketches the tool can't transcribe, complex visual constructions), the corrected version lives here. Curated artifacts override raw/ when both exist.

**Currently parked here:** `state-icons` is a known round-trip failure — see chat3 and `curated/README.md`.

## Consumption contract

Production code in `app/elohim-elements/` derives from this directory:

| Production package | Pulls from |
|---|---|
| `elohim-core/tokens.scss` | `raw/project/colors_and_type.css` (CSS custom properties) |
| `elohim-core` primitives (button, badge, card, form-field) | `raw/project/preview/components-*.html` as visual reference, spec as authority |
| `elohim-lamad/state-icons` | `curated/elohim-state-icons/` (when authored) |
| Pillar elements | `raw/project/ui_kits/lamad/` for applied patterns; spec sections for vocabulary |

Tokens flow one direction: spec → raw → curated (override) → production. Never edit `raw/` by hand; either fix the spec and re-round-trip, or fork into `curated/`.

## Known issues

- **State icons did not round-trip cleanly.** The 5-axis progressive icon system (Local / Hub / DHT / Doorway projection / Elohim vouch) requires sketch-derived visual construction the design tool struggled to honor across two corrective passes. See `raw/chats/chat3.md`. Hand-author into `curated/` from the original sketches when ready.
- **No "Dear Alice"-style scene illustrations.** The brand calls for hand-painted scene art; commission, don't synthesize.
- **Sound spec (§11)** is documented in the spec but not prototyped in the bundle.

## Vocabulary discipline

`genesis/graphos/vocabulary.md` and the README §1 vocabulary swap (household not user, neighbors not network, etc.) want to graduate from documentation into a content-lint rule. Track separately.
