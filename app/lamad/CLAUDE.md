# Lamad Reference Client Views

This directory contains the reference Angular client's view layer for the
lamad (learning) domain. Generated types, renderers, and components.

The domain vocabulary (manifest, schemas, coupling declarations) lives in
`elohim/sdk/domains/lamad/`. This directory consumes those definitions.

## Generated Types

Types in `generated/` are produced by `sdk/domains/lamad/scripts/codegen.mjs`.
Do not hand-edit — regenerate with `pnpm run lamad:codegen`.

## See Also

- Domain vocabulary: `elohim/sdk/domains/lamad/CLAUDE.md`
- Protocol schemas: `elohim/sdk/schemas/CLAUDE.md`
