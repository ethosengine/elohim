# Library B — the designed pattern library

This directory is **`graphos-designer`'s landing zone**. Stories here bind the Elohim brand tokens to component-architect's blank-slate primitives, supply realistic ts-rs view fixtures, and compose multi-element pattern stories that demonstrate the protocol in lived context.

See `app/elohim-library/CLAUDE.md` for the full library boundary and three-sources synthesis.

## Inviolable rule

Stories here **never modify the primitives themselves** (CSS, JSDoc, tag name, behavior). Token binding happens at the story decorator level — above the primitive, never below. If a primitive needs a new `@cssprop` override hook, file a follow-up for `component-architect`.

## Three layers within Library B

### `designed/foundations/__docs__/`

The brand-binding catalog — palette swatches with hex + role, type stack samples (Fraunces + Source Serif 4 + DM Sans + JetBrains Mono), spacing scale, iconography, motion-language demos at each stimulus tier.

Title prefix: `Designed/Foundations/...`

### `designed/<pillar>/__docs__/<element>.designed.stories.ts`

Per-primitive composition stories — same cell coverage as the matching default story, but with Elohim brand tokens bound via decorators and brand voice in any visible copy.

Title prefix: `Designed/<Pillar>/<element>`

### `designed/patterns/__docs__/<pattern>.designed.stories.ts`

Multi-element compositions that tell the protocol's lived story — `Household-Welcome`, `Provision-Completed`, `Steward-Setting-View`, `Hub-Aggregation-Shift`. Each pattern story should be recognizable as Elohim, not as a generic dashboard with Elohim colors.

Title prefix: `Designed/Patterns/<pattern>`

## What every designed story file must contain

1. Import the relevant ts-rs view from `@elohim/storage-client` as the prop fixture type (same as the matching default story).
2. Token-binding decorator at the meta level mapping `--elohim-*` overrides to `--el-*` brand values.
3. Same lens × theme × content-state coverage as the default story, but in the brand voice.
4. Realistic protocol vocabulary in any visible copy — `household` not `user`, `provision` not `transaction`, `commons` not `platform`.
5. RTL canary, light/dark pair, motion stories named explicitly.

## Reference template

See `app/elohim-library/CLAUDE.md` for the canonical "same primitive as default vs designed" comparison.
