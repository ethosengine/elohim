# Curated overrides

Hand-corrected artifacts that override `../raw/` when the round-trip through claude.ai/design fails to honor design intent.

## When to fork into curated/

- The design tool produced an output that doesn't match the spec or sketches, and re-prompting won't fix it (sketch-derived visual construction is the canonical example)
- A piece of the design system needs to be authored against codebase context the tool doesn't have (e.g., real Holochain state semantics for state icons)
- A primitive needs accessibility or implementation details the prototype HTML can't express

## Workflow

1. Copy the relevant file/directory from `../raw/` into the same relative path under `curated/`
2. Hand-edit to correct
3. Document **what the round-trip got wrong** at the top of the file (or in a sibling `NOTES.md`) so the next round-trip can be evaluated against the same constraints
4. Production code in `app/elohim-elements/` consumes from `curated/<path>` first, falls back to `raw/<path>`

## Currently parked

*(none yet — state icons will land here when hand-authored)*
