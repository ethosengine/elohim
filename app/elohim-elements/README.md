# elohim-elements

Single source of truth for elohim-protocol UI substrate, organised as a constellation of single-concern pnpm workspace packages. Each package ships **both** the styles (CSS custom properties / light-DOM globals where they exist) and the **Lit-based Custom Elements** that consume them.

## Modules

| Module             | Concern                                                                              |
| ------------------ | ------------------------------------------------------------------------------------ |
| `elohim-core`      | Tokens, light-DOM globals, atoms (button, card, input, badge, …)                     |
| `elohim-shell`     | Landing and host chrome — hero, footer, theme-toggle, etc.                           |
| `elohim-imagodei`  | Identity pillar — auth, profile, presence, recovery, agency, stewardship.            |
| `elohim-lamad`     | Learning pillar — content, paths, quiz engine, content-io, learner dashboard.        |
| `elohim-shefa`     | Economy pillar — stewardship, banking, REA flows, signals.                           |
| `elohim-qahal`     | Community pillar — governance, affinity, consent.                                    |
| `elohim-doorway`   | Doorway pillar — the in-app gateway-integration surface.                             |
| `elohim-avodah`    | Avodah meta-pillar — protocol-as-process reference implementation views.             |

## Layer model

- **Layer 1 — Tokens & light-DOM globals:** CSS custom properties in `elohim-core/tokens.scss`. Penetrate Shadow DOM via `var(--*)`.
- **Layer 2 — Custom Elements:** Lit components per package. Encapsulated styles via `static styles = css\`…\``. Consume tokens.
- **Layer 3 — Composition:** Storybook (`app/elohim-library/projects/graphos`). Documents and composes layers 1+2.

## Dependency direction

```
elohim-core
   ↑
   └── elohim-shell, elohim-imagodei, elohim-lamad, elohim-shefa,
       elohim-qahal, elohim-doorway, elohim-avodah
```

Pillar modules consume `elohim-core`. Pillars never consume each other — cross-pillar needs are a signal that the primitive belongs in `elohim-core`.

## Tag naming convention

- Core atoms: `<elohim-button>`, `<elohim-card>`, … (no pillar segment)
- Pillar components: `<elohim-imagodei-login>`, `<elohim-lamad-content-viewer>`, … (mirror package name 1:1)
- Always vendor-prefixed; third parties shipping their own pillar follow the same `<vendor>-<segment>-<name>` shape

## Consumers

- **`app/elohim-library` (graphos storybook)** — composition surface, documents every module.
- **`app/elohim-app`** — runtime; consumes via `CUSTOM_ELEMENTS_SCHEMA` in standalone components.
- **`doorway/doorway-app`** — admin UI; may consume `elohim-core` and selected pillars.

## Status

Sprint 1 (in progress): `<elohim-button>` end-to-end proof loop in `elohim-core`. All 7 other packages remain placeholder.
