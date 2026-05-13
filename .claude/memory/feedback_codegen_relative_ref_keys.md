---
name: codegen-ts.mjs refMap must register all relative-path key forms
description: $refs from sibling files use both bare-filename and "./prefix" forms; refMap must register both or json-schema-to-typescript falls through to filesystem ENOENT
type: feedback
originSessionId: cdffa1f9-7b63-4657-ae44-2cafff5156bf
---
`elohim/sdk/schemas/scripts/codegen-ts.mjs` `loadRefMap` registers schemas under multiple key forms because $refs in different schema directories use different relative-path conventions:
- Bare filename: `epr-envelope-view.schema.json` (sibling reference, used by some)
- Same-dir prefix: `./epr-envelope-view.schema.json` (sibling reference, used by epr-list-view and epr-view)
- Cross-dir prefix: `../views/epr-envelope-view.schema.json` (used from inputs/, enums/)
- URI-style $id: `epr:schema:view:human` (canonical refs)

Missing any of these means the lookup falls back to filesystem resolution from CWD, which produces ENOENT on the build agent.

**Why:** json-schema-to-typescript doesn't natively know about the workspace layout — it tries the literal $ref string against the filesystem from process CWD. The refMap is the bridge, and it has to enumerate every relative-path form an author might write.

**How to apply:** When adding a new schema directory or a new $ref convention, mirror the enum-handling pattern (lines 112-114) for both directions. Audit `git grep '"\$ref":' elohim/sdk/schemas/v1/` for unique relative-path forms and ensure each maps to a refMap entry. The fix is always a one-line addition to `loadRefMap`, not a rewrite of inlineRefs.
