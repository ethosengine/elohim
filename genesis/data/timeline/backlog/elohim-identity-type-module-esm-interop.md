---
title: Add "type":"module" to elohim-identity package.json so Node source-consumers drop the CJS/ESM interop shim
created: 2026-06-10
domain: process-meta (schema-sdk; SDK packaging)
source: arc plan Task 1.2 (commit 0026de6b1) — dependency-mechanism wrinkle
severity: low
---

`app/elohim-library/projects/elohim-identity/package.json` has no `"type":"module"`,
so Node-side consumers importing the framework-free TS source (a2o via tsconfig
path alias + tsx) get CJS treatment: named imports fail under tsx's ESM loader,
default imports fail under cucumber's CJS require hook. a2o works around it with
a commented `namespace.default ?? namespace` interop at its single import site
(doorway-client.ts). Adding `"type":"module"` collapses the shim to a plain named
import. Verify the Angular consumers first (elohim-app consumes the same package
via bundler path alias — bundlers generally ignore `type` for TS sources, but
prove it: `pnpm --filter elohim-app build` + library `ng build` if packaged).
Also sweep the other elohim-library projects for the same latent issue before
more Node consumers appear (rea-runtime is next per the arc plan Phase 4).

Sharper shape (post-1.2 review): the real packaging decision is a **framework-free
core entrypoint per SDK library** — a2o had to deep-import
`@elohim/identity/lib/doorway-session-client.js` because the package root
`public-api.ts` pulls Angular-dependent modules unresolvable in Node. A subpath
export (e.g. `@elohim/identity/core`) that exports ONLY framework-free modules
formalizes the boundary the arc created. DECIDE THIS BEFORE arc Phase 4:
`@elohim/rea-runtime`'s CommitmentService has the same Node+browser consumer set
and should be born with the pattern, not retrofit the interop hack.
