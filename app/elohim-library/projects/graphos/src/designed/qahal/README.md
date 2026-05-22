# designed/qahal — Library B Qahal-pillar pattern stories

Library B (the designed pattern library) for the Qahal pillar. Composes
Sprint 1A's elohim-qahal + elohim-imagodei Lit primitives into the
convergent Qahal homepage experience.

## Composition pattern

All 19 homepage stories share the same shape:

```typescript
import { qahalLightDecorator } from '../../../_lib/qahal-decorator';
import { renderQahalHomepage } from '../../../_lib/render-qahal-homepage';
import { someScene } from '../../../../../default/qahal/fixtures/{canonical,variations}/some-scene';

const meta: Meta = {
  title: 'Designed/Qahal/Homepage/.../...',
  decorators: [qahalLightDecorator],
};
export default meta;

export const Default: StoryObj = {
  render: () => renderQahalHomepage(someScene, {
    viewerTier: 'steward',
    powerUserVisible: false,
    lens: 'standard',
    activePanel: 'stream',
  }),
};
```

Every story file is ~30 lines. The composition logic lives once in
`_lib/render-qahal-homepage.ts`; the theme binding lives once in
`_lib/qahal-decorator.ts`.

## Adding a new homepage story

1. **Choose a fixture.** Canonical fixtures live at
   `default/qahal/fixtures/canonical/`; variations at
   `default/qahal/fixtures/variations/`. If you need a new fixture, add
   one there first (composing the existing `primitives/` mock-data
   modules) and add narrative-fidelity or typed-validity tests for it.
2. **Pick a bucket.** Four sidebar buckets exist:
   `canonical/`, `variations/`, `user-toggles/`, `capability-gating/`.
   Authors of new behavioral edge cases that need their own sidebar entry
   go in `variations/`; orthogonal toggles into `user-toggles/`; new
   viewer-tier rendering into `capability-gating/`. Choose by question:
   what does this story let a viewer recognize?
3. **Write the story file** at
   `homepage/__docs__/<bucket>/<name>.designed.stories.ts` using the
   template above.
4. **Run smoke test.** `pnpm storybook --ci --quiet` then
   `pnpm test-storybook --url http://localhost:6006 --include "Designed/Qahal/Homepage/<bucket>/**"`.

## Adding a new chrome element

If a new element is needed in the chrome assembly:

1. **Don't reach inside `renderQahalHomepage`** to add it inline. Update
   the composer module, adding the element to the appropriate column
   render function.
2. **Add a composer test** for the new element's presence + props in
   `_lib/render-qahal-homepage.spec.ts`.
3. **Bind its tokens** in `_lib/qahal-decorator.ts` light/dark/high-contrast
   blocks.

## Library boundary

Per `app/elohim-library/CLAUDE.md`: Library B never modifies primitives'
CSS, JSDoc, tag names, or behavior. If you need a `@cssprop` that doesn't
exist on a primitive, raise a `component-architect` follow-up — don't
reach inside the element.

## Cross-references

- Design spec: `genesis/docs/superpowers/specs/2026-05-22-sprint-1b-library-b-design.md`
- Implementation plan: `genesis/docs/superpowers/plans/2026-05-22-sprint-1b-library-b-pattern-stories.md`
- UX design spec: `genesis/docs/superpowers/specs/2026-05-22-qahal-homepage-ux-design.md`
- Canonical narratives: `genesis/docs/superpowers/specs/2026-05-21-qahal-section-4-canonical-narratives.md`
- Library A elements: `app/elohim-elements/elohim-qahal/`, `app/elohim-elements/elohim-imagodei/`
- Library boundary doctrine: `app/elohim-library/CLAUDE.md`
