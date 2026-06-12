/**
 * Viewport archetypes — the common device classes every primitive is judged
 * against, a first-class dimension of the component matrix exactly like the
 * light/dark theme cells and the a11y gates.
 *
 * Consumed by:
 *  - the viewport precondition gate (`src/testing/viewport.ts`, WTR specs)
 *  - the Storybook preview (viewport selector options, `.storybook/preview.ts`)
 *
 * SYNC: this table has a twin at
 * `genesis/a2o/src/framework/fixtures/viewport-archetypes.ts` (consumed by
 * a2o browser steps + the graphos sheet tool). The set is deliberately small
 * and stable; if you change one home, change the other.
 */

export interface ViewportArchetype {
  width: number;
  height: number;
  /** What this archetype stands in for — keeps additions honest. */
  represents: string;
}

export const VIEWPORT_ARCHETYPES = {
  /** Small/older phones and split-screen Android — the floor. */
  'phone-small': { width: 320, height: 568, represents: 'small phone / split-screen floor' },
  /** Mainstream phone portrait (iPhone 12-16 class). */
  phone: { width: 390, height: 844, represents: 'mainstream phone portrait' },
  /** Tablet portrait / phone landscape class. */
  tablet: { width: 768, height: 1024, represents: 'tablet portrait' },
  /** Default desktop window — matches the look/device default. */
  desktop: { width: 1280, height: 800, represents: 'desktop window' },
} as const satisfies Record<string, ViewportArchetype>;

export type ViewportArchetypeName = keyof typeof VIEWPORT_ARCHETYPES;
