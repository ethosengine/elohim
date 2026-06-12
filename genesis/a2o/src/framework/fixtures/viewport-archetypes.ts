/**
 * Viewport archetypes — the common device classes every UI surface is judged
 * against, the same way light/dark themes and the a11y gates are first-class
 * dimensions of the component matrix.
 *
 * SYNC: this table has a twin at
 * `app/elohim-elements/elohim-core/src/viewport-archetypes.ts` (consumed by
 * the element viewport gate + Storybook preview). The set is deliberately
 * small and stable; if you change one home, change the other.
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

export function viewportArchetype(name: string): ViewportArchetype {
  const archetype = (VIEWPORT_ARCHETYPES as Record<string, ViewportArchetype>)[name];
  if (!archetype) {
    throw new Error(
      `Unknown viewport archetype "${name}" — expected one of: ` +
        Object.keys(VIEWPORT_ARCHETYPES).join(', ')
    );
  }
  return archetype;
}
