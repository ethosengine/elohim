---
id: elohim-library-pattern-gospel
cites:
  - elohim-elements-ui-substrate-gospel | blank-slate gospel + layer rails that this library's two-library discipline serves | sha256:99e918ce9147e2f1
  - genesis/data/timeline/backlog/bundle-styling-token-contract.md
---

# elohim-library — Pattern Library Gospel

This document is the **shared synthesis** for everyone (humans + agents) authoring inside `app/elohim-library/`. It establishes the three sources of truth, the boundary between the "default" and "designed" pattern libraries, and the ownership rules.

**Agents that read this:** `component-architect`, `graphos-designer`, `angular-architect` (when migrating Angular components to Lit primitives), `content-pipeline` (when the manifest vocabulary affects pattern shape).

---

## The three sources of truth

Every story, fixture, and binding in this library is composed from exactly three inputs:

| # | Source | Provides | Lives in |
|---|---|---|---|
| 1 | **ts-rs generated views** | Typed prop shape (camelCase, JSON-parsed, real booleans) | `elohim/sdk/storage-client-ts/src/generated/*.ts` (`@elohim/storage-client`) |
| 2 | **App-manifest schemas** | Content-type / content-format / renderer / relationship vocabulary | `elohim/sdk/domains/<pillar>/manifest.json` + `manifest-types.ts` codegen |
| 3 | **graphos design tokens** | The Elohim brand binding — one example theme that resolves the `--elohim-*` override surface | `app/elohim-elements/elohim-core/tokens.scss` |

A story that does not come from this composition is wrong. If you find yourself inventing a local interface, hardcoding a brand color, or using vocabulary outside the manifest, stop and reach for one of the three sources above.

### Why these three

- **Views** are the truth boundary between Rust and TypeScript. Every story that renders data renders a view. If a view doesn't exist for a story you want to write, that's a `rust-architect` follow-up — never paper over with a local interface.
- **Manifests** are the truth boundary between the protocol substrate and the application layer. The manifests declare what content types, formats, and renderers an app can offer. Stories that demonstrate content rendering MUST use the vocabulary the manifest declares.
- **Tokens** are the truth boundary between the protocol's blank-slate primitives and the Elohim brand. The brand is *one* binding among many; the override surface is published in each element's `@cssprop` declarations.

---

## The two pattern libraries inside one storybook

`app/elohim-library/projects/graphos` hosts **two distinct pattern libraries** that share the same storybook composition surface but answer different questions:

### Library A — the default (out-of-the-box) pattern library

**Question it answers:** *"What does the protocol ship in the box? What is the unstyled, accessible, contract-passing primitive on its own?"*

**What lives here:**
- One story file per primitive — `<element>.default.stories.ts` — exercising every claimed `(lens × theme × contrast × locale × stimulus × textuality × standing)` cell.
- Foundation stories that catalog the token *interface* (what `--elohim-*` properties each primitive exposes) — NOT the brand binding.
- An `Unstyled (blank-slate proof)` story for every primitive — wrapped in `style="all: initial;"` — proving the element works in a fresh page with zero tokens bound.
- A `CustomTheme` story for every primitive demonstrating a deliberately non-Elohim theme binding (different palette, different typography) — proving the override surface is honest.

**Owner:** `component-architect` writes; everyone reads.

**Constraint:** Stories here MUST NOT bind the Elohim brand tokens. They use CSS system colors (`Canvas`, `CanvasText`, `ButtonFace`, …) and `font: inherit`. If you want to see what the element looks like with the brand bound, that's Library B's job.

### Library B — the designed pattern library

**Question it answers:** *"How does this primitive look and feel when assembled into the Elohim Protocol's actual experience? What patterns express the protocol's vision?"*

**What lives here:**
- One story file per primitive — `<element>.designed.stories.ts` — that binds the Elohim brand tokens via story-decorator overrides and shows the same lens × state matrix in the protocol's voice.
- Pattern stories — `<pattern>.designed.stories.ts` — that compose multiple primitives into recognizable Elohim scenes (`Household-Welcome`, `Provision-Completed`, `Steward-Setting-View`, `Hub-Aggregation-Shift`).
- Foundation stories that demonstrate the brand binding — color palette swatches, type stack samples, motion language demos at each stimulus tier, iconography catalog at the Elohim stroke + color.

**Owner:** `graphos-designer` writes.

**Constraint:** Stories here NEVER modify the primitive itself (CSS, JSDoc, tag name, behavior). Binding happens at the story decorator level, never below. If a needed `@cssprop` doesn't exist on a primitive, file it as a `component-architect` follow-up; don't reach inside.

### The boundary, in one sentence

**component-architect writes Library A; graphos-designer reads Library A and writes Library B.** Neither agent crosses into the other's library.

> **Shipping rail (2026-06-05):** Library B bindings are Storybook-only — no production bundle reads them today. The shippable binding artifact (graphos-tokens) is captured in `bundle-styling-token-contract` (see `cites:`); until it ships, bundles carry interim `_chrome-binding.scss` files per the layer rails in `elohim-elements-ui-substrate-gospel`.

### Directory convention (the scaffold is live)

The empty pillar directories already exist; agents drop stories straight in. Storybook discovery uses the glob `../projects/**/__docs__/**/*.@(stories.ts|mdx)`, so **the `__docs__/` segment in the path is mandatory** — it's how stories get picked up.

```
projects/graphos/src/
├── default/                                       # Library A — component-architect writes
│   ├── README.md                                  # landing-zone instructions
│   ├── core/__docs__/                             # elohim-core atoms + cross-pillar primitives
│   ├── shell/__docs__/                            # elohim-shell elements
│   ├── imagodei/__docs__/                         # elohim-imagodei elements
│   ├── lamad/__docs__/                            # elohim-lamad elements
│   ├── shefa/__docs__/                            # elohim-shefa elements
│   ├── qahal/__docs__/                            # elohim-qahal elements
│   ├── doorway/__docs__/                          # elohim-doorway elements
│   └── avodah/__docs__/                           # elohim-avodah elements
└── designed/                                      # Library B — graphos-designer writes
    ├── README.md                                  # landing-zone instructions
    ├── foundations/__docs__/                      # brand-binding catalog (palette, type, motion, iconography)
    ├── patterns/__docs__/                         # multi-element pattern stories
    ├── core/__docs__/                             # designed views of elohim-core primitives
    ├── shell/__docs__/                            # designed views of elohim-shell elements
    ├── imagodei/__docs__/                         # ...
    ├── lamad/__docs__/
    ├── shefa/__docs__/
    ├── qahal/__docs__/
    ├── doorway/__docs__/
    └── avodah/__docs__/
```

**File naming:** `<element>.default.stories.ts` for Library A, `<element>.designed.stories.ts` for Library B. Pattern stories follow `<pattern>.designed.stories.ts`.

**Story title prefixes:**
- Library A stories: `Default/<Pillar>/<element>` (e.g., `Default/Core/elohim-button`, `Default/Shefa/elohim-shefa-balance-card`)
- Library B element stories: `Designed/<Pillar>/<element>`
- Library B foundation stories: `Designed/Foundations/<topic>`
- Library B pattern stories: `Designed/Patterns/<pattern>`

**Pre-scaffold story (pending migration):** `src/foundations/__docs__/components/elohim-button.stories.ts` predates this split. It belongs in `src/default/core/__docs__/elohim-button.default.stories.ts` with retitled `Default/Core/elohim-button` and the `Unstyled` + `CustomTheme` proofs added. This migration is the natural first task for the next `component-architect` invocation that touches the button.

---

## Two source-registries the pattern libraries serve

The two libraries above each cover two registry surfaces:

### Registry 1 — `elohim-core` atoms and cross-pillar primitives

The base layer — `<elohim-button>`, `<elohim-card>`, `<elohim-input>`, `<elohim-badge>`, plus any cross-pillar primitive that proves itself foundational. Owns by `elohim-core` package.

Default stories here demonstrate the atom in isolation; designed stories show the brand-bound version. Pattern stories at this layer are small (a button group, a card layout) — bigger compositions belong to the pillar layer.

### Registry 2 — app-manifest content patterns

The app-manifest layer — content-type renderers, content-format mappings, relationship visualizations, signal kinds. These are declared by manifests at `elohim/sdk/domains/<pillar>/manifest.json` and the patterns demonstrate how a primitive renders each manifest entry.

Examples:
- The `sophia-quiz-json` content format → demonstrated in a designed pattern showing the assessment surface bound.
- The `html5-app` content format → demonstrated with a sandboxed iframe primitive.
- The `gherkin` format → a story demonstrating the gherkin-renderer primitive.

When the manifest declares a new vocabulary, the pattern library is the artifact that proves it's supported end-to-end.

---

## Mock data discipline (applies to both libraries)

Every fixture follows these rules:

1. **Imports the ts-rs view type.** Never a local interface unless data is purely operational (UI state).
2. **Matches the view's shape exactly.** camelCase keys, nested objects pre-parsed, real booleans.
3. **Uses realistic protocol vocabulary** in any string field — household names, provision verbs, REA commitment kinds the manifest declares.
4. **Uses BCP 47 locale codes** correctly (`en`, `en-US`, `es-MX`, `he-IL`, …).
5. **Uses well-formed content addresses** — fake CIDs/hashes are `sha256-` + 64 hex chars, never `"some-cid-here"`.
6. **Respects manifest vocabulary** — when a view carries `contentType` or `contentFormat`, the value MUST be one the manifest declares for that pillar.

---

## Authoring expectations (one-line summary per agent)

- **`component-architect`** — *writes Library A. Builds accessible blank-slate Lit primitives in `app/elohim-elements/<pillar>/`. Default stories follow the discipline above. Token defaults use CSS system colors; brand bake is rejected.*
- **`graphos-designer`** — *writes Library B. Reads Library A as input. Binds the Elohim brand tokens via story decorators; composes pattern stories. Never edits the primitives themselves.*
- **`angular-architect`** — *consumes the pattern library when migrating Angular components to Lit primitives. May propose new primitives, but the primitive itself is built by component-architect.*
- **`content-pipeline`** — *the manifest is its source of truth; pattern stories that demonstrate manifest vocabulary should pass review with this agent if manifest changes are in flight.*

---

## What the three-sources composition looks like in a story

Reference template — a single designed story binding all three sources:

```ts
// 1. ts-rs generated view = data shape
import type { ShefaBalanceView } from '@elohim/storage-client';

// 2. (implicit) manifest vocabulary lives in the field VALUES
// (e.g., quotaPolicy: 'hub-aggregated' must be a value the shefa manifest declares)

// 3. graphos design tokens = theme binding via decorator
import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components';
import 'elohim-shefa/register';

const mockBalance: ShefaBalanceView = {
  totalBytes: 16_106_127_360,  // 15 GB
  freeBytes: 5_368_709_120,     //  5 GB
  stewardedBytes: 0,
  hubMembers: 1,
  quotaPolicy: 'single-device',   // value from shefa manifest
  // ...all other fields per the view
};

const meta: Meta = {
  title: 'Designed/Shefa/elohim-shefa-balance-card',
  decorators: [
    story => html`
      <div style="
        --elohim-card-bg: var(--el-cream);
        --elohim-card-fg: var(--el-stone);
        --elohim-card-shadow: var(--el-shadow-soft);
        font-family: var(--el-font-body);
      ">${story()}</div>
    `,
  ],
  render: args => html`
    <elohim-shefa-balance-card .balance=${args.balance}></elohim-shefa-balance-card>
  `,
};
export default meta;
type Story = StoryObj;

export const Standard: Story = { args: { balance: mockBalance } };
export const Detail: Story = { args: { balance: { ...mockBalance, hubMembers: 4, quotaPolicy: 'hub-aggregated' } } };
```

Same primitive, written as a **default** story instead — no brand binding:

```ts
import type { ShefaBalanceView } from '@elohim/storage-client';
import { html } from 'lit';
import type { Meta, StoryObj } from '@storybook/web-components';
import 'elohim-shefa/register';

const mockBalance: ShefaBalanceView = { /* same fixture */ };

const meta: Meta = {
  title: 'Default/Shefa/elohim-shefa-balance-card',
  render: args => html`
    <elohim-shefa-balance-card .balance=${args.balance}></elohim-shefa-balance-card>
  `,
};
export default meta;

export const Standard: StoryObj = { args: { balance: mockBalance } };

export const Unstyled: StoryObj = {
  name: 'Unstyled (blank-slate proof)',
  decorators: [story => html`<div style="all: initial;">${story()}</div>`],
  args: { balance: mockBalance },
};

export const CustomTheme: StoryObj = {
  name: 'CustomTheme (override-surface proof)',
  decorators: [
    story => html`
      <div style="
        --elohim-card-bg: #1a1a2e;
        --elohim-card-fg: #f7f7ff;
        --elohim-card-border: 2px solid #ff6b6b;
        font-family: ui-monospace, monospace;
      ">${story()}</div>
    `,
  ],
  args: { balance: mockBalance },
};
```

The two stories live in different directories with different titles. They cover the SAME primitive but answer different questions.

---

## Library scope — thin-client discipline

Both Library A and Library B exist to **demonstrate UI primitives**. The pattern library, the fixtures it ships, and the services exposed from `@elohim/service` / `@elohim/identity` / `@elohim/rea-runtime` are all scoped to legitimate client-side concerns:

1. **UX** — what the user sees, how it looks, how it behaves
2. **Accessibility** — keyboard, screen reader, focus, contrast, motion preferences
3. **Sense-and-respond** — reacting to substrate-provided state, emitting user-intent signals

That is the legitimate scope. Code that crosses out of it does not belong in `app/elohim-library/` (and does not belong in `app/elohim-elements/` either — see `app/elohim-elements/CLAUDE.md`).

### Anti-patterns to flag (in services, in story fixtures, in pattern compositions)

If you find any of the following being added to a library service, a pattern story, or proposed for migration INTO this library, **stop**. It is a substrate concern, not a client concern, and the substrate-correct home is the backend (doorway route handler, zome coordinator function, or storage projection layer):

| Anti-pattern | Substrate-correct home |
|---|---|
| **HTTP / zome-call orchestration** that aggregates across multiple substrate entries | Storage projection (`elohim-storage/src/views.rs`); the library service receives a single ts-rs view |
| **REA economic-event creation** (signing, validating, projecting `EconomicEvent` / `Commitment` / `FeedbackSignal`) | Mishpat / elohim zome coordinator function; doorway route emits the projection; library service receives the resulting view |
| **Signal-aggregation arithmetic across many entries** (computing consensus strength, controversy thresholds, settled-state derivation across all participants) | Storage projection. Library service consumes the aggregate view, doesn't compute it. (Per-row helper functions that derive a status flag from one already-aggregated view are fine — see Slice 2.2b `SignalAccumulationService`.) |
| **Submission-flow orchestration** (multi-step transactions, retry policy, error reconciliation, idempotency) | Doorway route owns the transaction; library service exposes the route's HTTP surface; the consumer renders submit / busy / success / error states from a single observable |
| **Mediation / consent / governance authority logic** | Zome coordinator function (the substrate has standing to enforce); library service exposes the view |
| **Recognition / standing / mastery wiring** (weight computation, affinity distribution, attestation recording) | REA primitive in `mishpat::Commitment`; library service exposes the view |
| **Cache invalidation across multiple substrate views** beyond simple per-view caching | `elohim-storage` cache layer publishes invalidation signals; library service subscribes |
| **Cross-pillar transaction coordination** (one user action that has to update multiple pillars atomically) | Doorway route owns the orchestration; library service exposes one observable per pillar view |

Pure helpers — functions that derive a status flag from one already-supplied view, or interpret a string-union, or compute a UI-shape from a substrate-shape — ARE legitimate. The line is between **interpreting one view** (client-OK) and **orchestrating across substrate** (backend-only).

### Why this discipline matters

The substrate ([stewardship-over-sovereignty](../../genesis/docs/architecture/stewardship-over-sovereignty.md) §3) is the steward. When orchestration moves client-side, the client begins to own responsibilities the substrate should hold — and that breaks:

- **Cross-host consistency** — three client instances compute the same aggregation three different ways
- **Offline correctness** — client orchestration assumes connectivity; substrate-mediated flows degrade gracefully
- **Auditability** — REA events created client-side bypass the substrate's notarization story
- **Multi-client uniformity** — every consumer (`elohim-app`, Tauri desktop, doorway projection, third-party) renders the same elements; if elements/library services own orchestration, every client re-implements it

The five libraries (`@elohim/service`, `elohim-core`, `@elohim/storage-client`, `@elohim/identity`, `@elohim/rea-runtime`) are Category C (operational) per [elohim-sdk](../../genesis/docs/architecture/elohim-sdk.md) §1. They hold no source-of-truth state. Anything that LOOKS like state-owning code on the client is a sign the substrate owes a new route or zome function — not a sign the library needs a thicker service.

### When you find this

You don't fix it by thickening a library service. You don't fix it by inlining the orchestration into a consuming pillar. You open a backend-migration ticket naming the substrate-correct destination (doorway route name, zome function signature, projection view name), and you leave the existing Angular orchestration in place until the backend lands. The Slice 2.2b deferral (the three `@app/qahal` Angular components retained in `content-viewer.component.ts`) is the worked example. See [pillar-bundle-split runbook §6.14](../../genesis/docs/architecture/pillar-bundle-split-runbook.md#614--client-side-stateful-orchestrator-anti-pattern-smell-to-fix) for the canon shape.

---

## Cross-references

- Element-substrate gospel (scope discipline): `app/elohim-elements/CLAUDE.md`
- SDK boundary canon: `genesis/docs/architecture/elohim-sdk.md`
- Pillar-bundle-split runbook (includes §6.14 backend-migration anti-pattern): `genesis/docs/architecture/pillar-bundle-split-runbook.md`
- Substrate principle: `genesis/docs/architecture/stewardship-over-sovereignty.md`
- Capability Profile primitive spec: `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`
- Brand design spec: `genesis/graphos/elohim-protocol-design-spec.md`
- Protocol vocabulary register: `genesis/graphos/vocabulary.md`
- elohim-elements substrate: `app/elohim-elements/README.md`
- Agent definitions: `.claude/agents/{component-architect,graphos-designer,angular-architect,content-pipeline}.md`

When in doubt, the brand spec is gospel for Library B aesthetic; the Capability Profile spec is gospel for primitive contracts; this file is gospel for how the three sources compose into the pattern library.
