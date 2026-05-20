# Library A — the default pattern library

This directory is **`component-architect`'s landing zone**. Stories here demonstrate the protocol's blank-slate primitives as shipped — no Elohim brand tokens bound, CSS system colors as defaults, `font: inherit`.

See `app/elohim-library/CLAUDE.md` for the full library boundary and three-sources synthesis.

## Where to put a new element story

```
default/<pillar>/__docs__/<element>.default.stories.ts
```

Pillars correspond 1:1 to `app/elohim-elements/<pillar>/`:
`core`, `shell`, `imagodei`, `lamad`, `shefa`, `qahal`, `doorway`, `avodah`.

Cross-pillar primitives (atoms) live under `default/core/__docs__/`.

## Story title convention

```ts
title: 'Default/<Pillar>/<element>'
```

For example: `'Default/Core/elohim-button'`, `'Default/Shefa/elohim-shefa-balance-card'`.

## What every default story file must contain

1. Import the relevant ts-rs view from `@elohim/storage-client` as the prop fixture type.
2. One named story per claimed lens (`Minimal`, `Simple`, `Standard`, `Detail`, etc.).
3. Light + Dark + RTL canary (`Hebrew` if `he` locale is claimed) named stories.
4. Content-state stories per the contract (`Empty`, `Loading`, `Error`, `Stale`, `Contested`, `Offline`).
5. **`Unstyled (blank-slate proof)`** — wrapped in `style="all: initial;"` proving the primitive renders without any token binding.
6. **`CustomTheme (override-surface proof)`** — binding to a deliberately non-Elohim theme (different palette, different typography) proving the override surface is honest.

**Never bind Elohim brand tokens (`--el-*`) in a default story.** That's Library B (`designed/`) territory.

## Reference template

See `app/elohim-library/CLAUDE.md` for the canonical "same primitive as default vs designed" comparison.
