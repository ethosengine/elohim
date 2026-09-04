---
id: "backlog-psephos-naming-drift"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Psephos naming drift — reserve psephos for psychometric self-knowledge and rename the ballot renderer to ballot"
slug: "psephos-naming-drift"
written: "2026-09-05"
author: "operator ruling, 2026-09-05"
status: "open"
priority: "medium"
tags: [psephos, psychometrics, qahal, ballot, renderer, naming, codegen, frontend]
cites:
  - genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md
---

# Psephos names the psychometric instrument, not the ballot renderer

## Ruling

**Operator, 2026-09-05:** `psephos` is the psychometric assessment instrument for stated
preferences and self-knowledge, as defined in
`genesis/docs/content/elohim-protocol/architecture/imagodei-surfaces-design.md:58`. It is not the
formal-governance ballot renderer. The ballot-renderer vocabulary below must move to the unambiguous
target `ballot`: `renderTarget: 'ballot'`, `BallotWrapperComponent`, and
`@elohim/ballot-plugin`. Status remains open until all four surfaces move together; a partial rename
would split the generated discriminator from its consumers.

## Rename surfaces

- [ ] **Angular qahal wrapper → `BallotWrapperComponent`.** Rename
  `app/elohim-app/src/app/qahal/components/psephos-ballot-wrapper/` and its component class,
  selector, imports, and export in `app/elohim-app/src/app/qahal/index.ts`. Update the dispatch in
  `app/elohim-app/src/app/qahal/components/feedback-mechanism-gateway/feedback-mechanism-gateway.component.ts`
  and the matching graphos story/docs references under
  `app/elohim-library/projects/graphos/src/`.

- [ ] **Generated renderer discriminator → `renderTarget: 'ballot'`.** The authored source for
  `app/lamad/src/generated/mechanism-selection.ts` is
  `elohim/sdk/schemas/v1/views/mechanism-selection.schema.json`; its runtime policy input is the
  qahal pillar-projection `Manifest` payload shaped by
  `elohim/sdk/schemas/v1/manifest-payloads/pillar-projection.schema.json` (the domain catalog
  `elohim/sdk/domains/qahal/manifest.json` does not generate this discriminator). Change the view
  schema enum/description and the producer in
  `elohim/elohim-storage/src/db/mechanism_selection.rs`, then regenerate rather than editing any
  distributed `mechanism-selection.ts` copy. Update the consuming cast in
  `app/lamad/src/app/components/content-viewer/content-viewer.component.ts` and the
  `FeedbackRenderTarget` type/render branch/tests in
  `app/elohim-elements/elohim-core/src/elohim-feedback-mechanism-gateway.ts` and its spec.

- [ ] **Angular library package → `@elohim/ballot-plugin`.** Rename
  `app/elohim-library/projects/psephos-plugin/`, its `package.json` name, public API, component and
  loader symbols/files, selectors, bundle URL, and the built asset location currently at
  `app/elohim-app/src/assets/psephos-plugin/`. The package path found in-tree is
  `app/elohim-library/projects/psephos-plugin`; callers must import the new package name in the same
  change so no compatibility alias prolongs the collision.

- [ ] **A2O census language → ballot.** Change the
  `genesis/a2o/layering/surface-census.md` row for
  `features/qahal/collective-governance.feature` from “renders via Psephos” to “renders via the ballot
  renderer”, and update the scenario wording/step bindings in
  `genesis/a2o/features/qahal/collective-governance.feature` if they carry the old product name.

## Codegen and landing order

1. Change the authored view schema and the Rust mechanism-selection producer/tests together; keep
   `pillar-projection.schema.json` unchanged unless its policy vocabulary actually changes.
2. Run `pnpm run schema:codegen:ts` to regenerate the distributed
   `mechanism-selection.ts` files, including `app/lamad/src/generated/mechanism-selection.ts`.
3. Run the storage binding export (`cargo test export_bindings` in its governed gate environment)
   so `elohim/sdk/storage-client-ts/src/generated/MechanismSelectionView.ts` agrees with the wire
   producer; never hand-edit that generated file.
4. Rename the Angular wrapper/package/assets and update every consumer, type, story, and test to
   `ballot`; then update the a2o feature and its generated census through the census's owning tool.
5. Verify schema/codegen freshness first, then the library/app and a2o owning gates. Land the four
   checkboxes atomically so no deployed client receives a discriminator it cannot render.
