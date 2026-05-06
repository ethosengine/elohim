# elohim-core

Protocol substrate styles. Single source of truth for tokens, breakpoints, base resets, and shared mixins. Every other `elohim-styles` module `@use`s this; consumer apps and storybook pull from here for the foundation layer.

## Scope

- Color tokens, typography scale, spacing scale, motion vocabulary (CSS custom properties).
- Light/dark theme overrides via `prefers-color-scheme` media query.
- Responsive breakpoint variables (`--bp-sm` through `--bp-xl`).
- Global resets and base element styles.
- Cross-pillar protocol-substrate components (e.g. `device-tile`, `network-health-tab`) that aren't part of any single pillar.

## Status

Scaffold only. Content lands in the harvest step from `app/elohim-app/src/styles.css` and `app/elohim-app/src/app/elohim/components/`.

## Consumers

- `elohim-shell`, `elohim-imagodei`, `elohim-lamad`, `elohim-shefa`, `elohim-qahal`, `elohim-doorway`, `elohim-avodah` — sibling modules.
- `app/elohim-library` (storybook design surface).
- `app/elohim-app`, `doorway/doorway-app` — runtime apps.
