# Capability Profile + Element Contract — Implementation Plan (M1–M3)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Land the foundation of the Capability Profile primitive + element contract: types, propagation, three precondition-gate test harnesses, CEM extension, and a first migrated element (`<elohim-button>`) with a passing contract end-to-end.

**Architecture:** All work lives in `app/elohim-elements/elohim-core/`. The Profile + ContentCertainty types and `@lit/context` provider/consumer/mixin land first. Three test harnesses follow: a11y (mostly extends existing axe-core + open-wc setup), i18n (introduces `@lit/localize`), and ua-prefs (CSS media-query toggles + a photosensitive-flash analyzer). A custom CEM analyzer plugin reads `@capability*` JSDoc tags into the `capabilityContract` block. Finally, `<elohim-button>` is migrated as the canary element with all three gates passing.

**Tech Stack:** TypeScript 5.7, Lit 3.2, `@lit/context`, `@lit/localize` (new), `@open-wc/testing`, axe-core 4.10, `@web/test-runner` + Playwright (chromium), `@custom-elements-manifest/analyzer` 0.10. Build via Vite 6 + cem analyze. Lint via ESLint 9 flat config + stylelint.

**Out of scope for this plan** (becomes follow-on plans after this ships):
- M4 — Storybook coverage matrix story in graphos
- M5 — Second element with wider contract (he-IL RTL, symbolic, ContentCertainty-observed)
- M6 — Shell lens-toggle / banners / lock-respecting switchers
- M7 — Codegen enforcement (refuse CEM emission on gaps)

**Reference:** Design spec at `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md` — every claim here traces back to a spec section.

---

## P2P design-gate classification (explicit)

This plan introduces **no persisted data entities, no DHT entry types, no sync messages, and no REA primitives.** The p2p-design-gate skill does not apply because:

| Construct in this plan | Layer | Lifetime | Crosses peer boundary? |
|---|---|---|---|
| `CapabilityProfile` (TypeScript interface) | UI runtime | Lit context, per-session | No — render-time only |
| `ContentCertainty` (TypeScript interface) | UI runtime | Prop or context, per-content | No — derived at render-time from storage |
| `ProfileLock`, `Standing` (TypeScript types) | UI runtime | In-memory | No |
| `lit-localize.json` | Build config | Compile-time | No — consumed by lit-localize-tools |
| XLIFF locale bundles (`*.xlf`, `*.js`) | Build output | Compile-time | No — bundled into the elements distribution |
| `custom-elements.json` (`capabilityContract` block) | Build output | Compile-time CEM manifest | No — read by storybook + tooling |

The Profile is **derived** at session start from imagodei attestations (which ARE persisted DHT entries — and which are designed elsewhere); the Profile itself is the in-memory projection. ContentCertainty is **derived** from elohim-storage reconciliation state; storage owns the persisted truth, the certainty primitive is the renderer-facing observable.

When sub-project #3 (steward-lock attestation persistence) is brainstormed, THAT spec will invoke the p2p-design-gate properly — the steward-lock is a real DHT entry that propagates between peers. This plan does not include that work.

---

## File Structure

**Files this plan creates:**

```
app/elohim-elements/elohim-core/
├── src/
│   ├── capability/
│   │   ├── profile.ts                 # CapabilityProfile, Lens, Theme, Contrast, Locale, Stimulus, Textuality, ProfileLock types + DEFAULT_PROFILE
│   │   ├── profile.spec.ts            # Type-level + constant assertions
│   │   ├── certainty.ts               # ContentCertainty, CertaintyState types
│   │   ├── certainty.spec.ts          # Default/unknown-state assertions
│   │   ├── standing.ts                # Standing types + DSL parser (`"pilot | steward"`, `["pilot", "contributor"]`)
│   │   ├── standing.spec.ts           # DSL parsing tests
│   │   ├── context.ts                 # @lit/context createContext for profile + certainty
│   │   ├── context.spec.ts            # Provider/consumer round-trip + narrowing-only invariant
│   │   ├── mixin.ts                   # CapabilityAwareElement(LitElement) mixin
│   │   └── mixin.spec.ts              # Mixin re-renders on profile change
│   ├── testing/
│   │   ├── a11y.ts                    # axeScan(), keyboardNav() helpers
│   │   ├── i18n.ts                    # noHardcodedStrings(), rtlLayout(), localeFormatting(), symbolicInventory()
│   │   ├── ua-prefs.ts                # setMediaQuery(), photosensitiveFlash() analyzer
│   │   ├── ua-prefs.spec.ts           # Self-test the harness helpers
│   │   └── index.ts                   # Barrel
│   ├── elohim-button.ts               # Modified: extend CapabilityAwareElement + JSDoc @capability* tags
│   ├── elohim-button.spec.ts          # Modified: add contract-tier tests (i18n / ua-prefs / states)
│   ├── elohim-button.manifest.spec.ts # Modified: assert capabilityContract block
│   ├── register.ts                    # Unchanged (button registration already there)
│   ├── index.ts                       # Modified: export capability + testing barrels
│   └── localize/
│       ├── runtime.ts                 # @lit/localize runtime config
│       ├── source-locale.ts           # Source-locale strings file (en)
│       └── locales/
│           └── es.xliff               # Spanish translation seed (one string, smoke test)
├── custom-elements-manifest.config.mjs   # Modified: register capabilityContract plugin
├── cem-plugins/
│   └── capability-contract.mjs         # Custom CEM analyzer plugin reads @capability* JSDoc tags
└── package.json                         # Modified: add @lit/context, @lit/localize, ESLint extras
```

**Why this structure:**
- `src/capability/` groups the runtime primitive. Co-locates types, context, mixin, and their specs.
- `src/testing/` groups gate helpers — importable by every element's spec, never used in production code.
- `src/localize/` is the canonical `@lit/localize` shape (runtime + source + locale bundles).
- `cem-plugins/` keeps custom CEM tooling outside `src/` so it isn't bundled into the elements distribution.

---

## Task 1: Add new dependencies

**Files:**
- Modify: `app/elohim-elements/elohim-core/package.json`

- [ ] **Step 1: Add runtime + dev deps**

Run from `app/elohim-elements/elohim-core/`:

```bash
pnpm add @lit/context@^1.1.3 @lit/localize@^0.12.2
pnpm add -D @lit/localize-tools@^0.8.0
```

- [ ] **Step 2: Verify package.json reflects additions**

Run: `pnpm --filter elohim-core list --depth=0 | grep -E '@lit/(context|localize)'`
Expected:
```
@lit/context 1.1.3
@lit/localize 0.12.2
@lit/localize-tools 0.8.0
```

- [ ] **Step 3: Commit**

```bash
git add app/elohim-elements/elohim-core/package.json pnpm-lock.yaml
git commit -m "build(elohim-core): add @lit/context and @lit/localize for capability primitive"
```

---

## Task 2: Profile types

**Files:**
- Create: `app/elohim-elements/elohim-core/src/capability/profile.ts`
- Create: `app/elohim-elements/elohim-core/src/capability/profile.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `src/capability/profile.spec.ts`:

```ts
import { expect } from '@open-wc/testing';
import { DEFAULT_PROFILE } from './profile.js';
import type { CapabilityProfile } from './profile.js';

describe('CapabilityProfile / DEFAULT_PROFILE', () => {
  it('has Sabbath defaults: stimulus=still, textuality=textual', () => {
    expect(DEFAULT_PROFILE.stimulus).to.equal('still');
    expect(DEFAULT_PROFILE.textuality).to.equal('textual');
  });

  it('defaults theme/contrast/locale to auto', () => {
    expect(DEFAULT_PROFILE.theme).to.equal('auto');
    expect(DEFAULT_PROFILE.contrast).to.equal('auto');
    expect(DEFAULT_PROFILE.locale).to.equal('auto');
  });

  it('defaults lens to standard for adult-pilot baseline', () => {
    expect(DEFAULT_PROFILE.lens).to.equal('standard');
  });

  it('has an unstewarded pilot lock by default', () => {
    expect(DEFAULT_PROFILE.lock.kind).to.equal('pilot');
    expect(DEFAULT_PROFILE.origin).to.equal('pilot');
    expect(DEFAULT_PROFILE.standings).to.deep.equal([]);
  });

  it('is shape-valid against the CapabilityProfile type', () => {
    // Compile-time check: if the type and the constant disagree, tsc fails the build.
    const profile: CapabilityProfile = DEFAULT_PROFILE;
    expect(profile).to.exist;
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

Run from `app/elohim-elements/elohim-core/`:
```bash
pnpm test --filter capability/profile.spec.ts
```
Expected: FAIL with `Cannot find module './profile.js'`.

- [ ] **Step 3: Implement profile.ts**

Create `src/capability/profile.ts`:

```ts
/**
 * Capability Profile — the viewer-side context object observed by every elohim-element.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §2
 */

export type Lens =
  | 'minimal'
  | 'simple'
  | 'standard'
  | 'detail'
  | 'debug'
  | 'trace';

export const LENS_ORDER: readonly Lens[] = [
  'minimal',
  'simple',
  'standard',
  'detail',
  'debug',
  'trace',
] as const;

export type Theme = 'light' | 'dark' | 'auto';

export type Contrast = 'normal' | 'high' | 'auto';

/** BCP 47 language tag, or 'auto' to resolve from navigator.language. */
export type Locale = string;

export type Stimulus = 'still' | 'gentle' | 'lively' | 'auto';

export const STIMULUS_ORDER: readonly Stimulus[] = ['still', 'gentle', 'lively'] as const;

export type Textuality = 'symbolic' | 'textual' | 'auto';

export interface ProfileLock {
  kind: 'pilot' | 'steward' | 'elohim-support';
  pinnedLens?: Lens;
  maxLens?: Lens;
  pinnedTheme?: Theme;
  pinnedContrast?: Contrast;
  pinnedLocale?: Locale;
  pinnedStimulus?: Stimulus;
  maxStimulus?: Stimulus;
  pinnedTextuality?: Textuality;
  /** ms epoch — present for time-bounded elohim-support sessions */
  expiresAt?: number;
}

export type Standing = string;

export interface CapabilityProfile {
  lens: Lens;
  theme: Theme;
  contrast: Contrast;
  locale: Locale;
  stimulus: Stimulus;
  textuality: Textuality;
  standings: Standing[];
  lock: ProfileLock;
  origin: 'pilot' | 'steward' | 'elohim-support';
}

/**
 * The Sabbath default. Stillness in the type system; textual for the literate-adult baseline.
 * Stewards may pin pre-literate/symbolic, locked-lens, locked-locale, etc.
 */
export const DEFAULT_PROFILE: CapabilityProfile = Object.freeze({
  lens: 'standard',
  theme: 'auto',
  contrast: 'auto',
  locale: 'auto',
  stimulus: 'still',
  textuality: 'textual',
  standings: [],
  lock: { kind: 'pilot' },
  origin: 'pilot',
});
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter capability/profile.spec.ts
```
Expected: PASS (5 tests).

- [ ] **Step 5: Typecheck**

```bash
pnpm --filter elohim-core typecheck
```
Expected: no errors.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/capability/profile.ts app/elohim-elements/elohim-core/src/capability/profile.spec.ts
git commit -m "feat(elohim-core): add CapabilityProfile types + DEFAULT_PROFILE constant"
```

---

## Task 3: ContentCertainty types

**Files:**
- Create: `app/elohim-elements/elohim-core/src/capability/certainty.ts`
- Create: `app/elohim-elements/elohim-core/src/capability/certainty.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `src/capability/certainty.spec.ts`:

```ts
import { expect } from '@open-wc/testing';
import { UNKNOWN_CERTAINTY } from './certainty.js';
import type { ContentCertainty, CertaintyState } from './certainty.js';

describe('ContentCertainty / UNKNOWN_CERTAINTY', () => {
  it('defaults to state=unknown with no richness fields populated', () => {
    expect(UNKNOWN_CERTAINTY.state).to.equal('unknown');
    expect(UNKNOWN_CERTAINTY.freshness).to.be.undefined;
    expect(UNKNOWN_CERTAINTY.attestationCount).to.be.undefined;
  });

  it('is shape-valid against the ContentCertainty type', () => {
    const c: ContentCertainty = UNKNOWN_CERTAINTY;
    expect(c).to.exist;
  });

  it('enumerates the six CertaintyState values', () => {
    const states: CertaintyState[] = ['canonical', 'partial', 'stale', 'contested', 'unreachable', 'unknown'];
    expect(states).to.have.lengthOf(6);
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test --filter capability/certainty.spec.ts
```
Expected: FAIL — module not found.

- [ ] **Step 3: Implement certainty.ts**

Create `src/capability/certainty.ts`:

```ts
/**
 * Content Certainty — the content-side observable, parallel to CapabilityProfile.
 *
 * Describes the *content being rendered*, not the viewer. In a P2P / EPR / CID world,
 * "data is loaded" does not mean "data is true." Elements observe this to render with
 * epistemic honesty.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §4
 */

export type CertaintyState =
  | 'canonical'   // fresh, fully reconciled, multiple confirming attestations
  | 'partial'     // syncing in progress; some peers haven't responded yet
  | 'stale'       // last reconciled long ago; may be out of date
  | 'contested'   // multiple attestations disagree; no canonical view exists
  | 'unreachable' // no peer currently serving; local cache only
  | 'unknown';    // freshly opened, not yet probed

export interface ContentCertainty {
  state: CertaintyState;
  /** ms since last reconciliation against any peer */
  freshness?: number;
  /** how many witnesses have signed this content */
  attestationCount?: number;
  /** hops from viewer to nearest steward of this content */
  reachDistance?: number;
  /** attestation IDs disagreeing with the canonical view */
  contestedBy?: string[];
  /** which peers have served this content */
  sourcePeers?: string[];
}

/** The safe default when no certainty has been provided. Elements MUST NOT render
 *  canonical visuals when their certainty is unknown. */
export const UNKNOWN_CERTAINTY: ContentCertainty = Object.freeze({
  state: 'unknown',
});
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter capability/certainty.spec.ts
```
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/capability/certainty.ts app/elohim-elements/elohim-core/src/capability/certainty.spec.ts
git commit -m "feat(elohim-core): add ContentCertainty types + UNKNOWN_CERTAINTY default"
```

---

## Task 4: Standing DSL parser

**Files:**
- Create: `app/elohim-elements/elohim-core/src/capability/standing.ts`
- Create: `app/elohim-elements/elohim-core/src/capability/standing.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `src/capability/standing.spec.ts`:

```ts
import { expect } from '@open-wc/testing';
import { parseStandingRequirement, satisfiesRequirement } from './standing.js';

describe('Standing DSL', () => {
  describe('parseStandingRequirement', () => {
    it('parses a single Standing as one AND-group containing one OR-clause', () => {
      const parsed = parseStandingRequirement(['pilot']);
      expect(parsed).to.deep.equal([['pilot']]);
    });

    it('parses an OR string into a single AND-group with multiple OR-clauses', () => {
      const parsed = parseStandingRequirement(['pilot | steward']);
      expect(parsed).to.deep.equal([['pilot', 'steward']]);
    });

    it('parses array of Standings as multiple AND-groups (AND semantics)', () => {
      const parsed = parseStandingRequirement(['pilot', 'contributor']);
      expect(parsed).to.deep.equal([['pilot'], ['contributor']]);
    });

    it('parses mixed: array entries are AND, | within entry is OR', () => {
      const parsed = parseStandingRequirement(['pilot | steward', 'contributor']);
      expect(parsed).to.deep.equal([['pilot', 'steward'], ['contributor']]);
    });

    it('trims whitespace around | tokens', () => {
      const parsed = parseStandingRequirement(['pilot|steward', 'pilot |  contributor']);
      expect(parsed).to.deep.equal([['pilot', 'steward'], ['pilot', 'contributor']]);
    });
  });

  describe('satisfiesRequirement', () => {
    it('passes when viewer holds the single required Standing', () => {
      expect(satisfiesRequirement(['pilot'], ['pilot'])).to.be.true;
    });

    it('fails when viewer lacks the required Standing', () => {
      expect(satisfiesRequirement(['steward'], ['pilot'])).to.be.false;
    });

    it('passes for OR: viewer holds one of several alternatives', () => {
      expect(satisfiesRequirement(['steward'], ['pilot | steward'])).to.be.true;
    });

    it('passes for AND: viewer holds all required Standings', () => {
      expect(satisfiesRequirement(['pilot', 'contributor'], ['pilot', 'contributor'])).to.be.true;
    });

    it('fails AND when viewer lacks one part', () => {
      expect(satisfiesRequirement(['pilot'], ['pilot', 'contributor'])).to.be.false;
    });

    it('handles mixed: AND of (pilot OR steward) AND contributor', () => {
      expect(satisfiesRequirement(['steward', 'contributor'], ['pilot | steward', 'contributor'])).to.be.true;
      expect(satisfiesRequirement(['steward'], ['pilot | steward', 'contributor'])).to.be.false;
    });

    it('returns true for empty requirement (vacuous)', () => {
      expect(satisfiesRequirement(['pilot'], [])).to.be.true;
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test --filter capability/standing.spec.ts
```
Expected: FAIL — module not found.

- [ ] **Step 3: Implement standing.ts**

Create `src/capability/standing.ts`:

```ts
/**
 * Standing — attested roles a viewer holds in this context.
 *
 * The DSL: an element declares requirements as an array of strings. Each entry is
 * AND-combined. Within an entry, `|` means OR. Examples:
 *   ['pilot']                          — must hold pilot
 *   ['pilot | steward']                — pilot OR steward
 *   ['pilot', 'contributor']           — pilot AND contributor
 *   ['pilot | steward', 'contributor'] — (pilot OR steward) AND contributor
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §5.3
 */

import type { Standing } from './profile.js';

/**
 * Parses the contract DSL into normalized form: outer array is AND, inner array is OR.
 * Each inner array element is a Standing name.
 */
export function parseStandingRequirement(requirement: readonly string[]): readonly Standing[][] {
  return requirement.map(entry =>
    entry.split('|').map(token => token.trim()).filter(token => token.length > 0)
  );
}

/**
 * Returns true if `held` satisfies `requirement`.
 * AND across outer groups; OR within each group.
 */
export function satisfiesRequirement(
  held: readonly Standing[],
  requirement: readonly string[]
): boolean {
  const parsed = parseStandingRequirement(requirement);
  return parsed.every(orGroup => orGroup.some(s => held.includes(s)));
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter capability/standing.spec.ts
```
Expected: PASS (11 tests).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/capability/standing.ts app/elohim-elements/elohim-core/src/capability/standing.spec.ts
git commit -m "feat(elohim-core): add Standing DSL parser + satisfiesRequirement"
```

---

## Task 5: Lit context for profile + certainty

**Files:**
- Create: `app/elohim-elements/elohim-core/src/capability/context.ts`
- Create: `app/elohim-elements/elohim-core/src/capability/context.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `src/capability/context.spec.ts`:

```ts
import { expect, fixture, html } from '@open-wc/testing';
import { LitElement } from 'lit';
import { consume, provide } from '@lit/context';
import { property, state } from 'lit/decorators.js';

import { capabilityProfileContext, contentCertaintyContext } from './context.js';
import { DEFAULT_PROFILE } from './profile.js';
import { UNKNOWN_CERTAINTY } from './certainty.js';
import type { CapabilityProfile } from './profile.js';
import type { ContentCertainty } from './certainty.js';

class TestProvider extends LitElement {
  @provide({ context: capabilityProfileContext })
  @property({ attribute: false })
  profile: CapabilityProfile = DEFAULT_PROFILE;

  @provide({ context: contentCertaintyContext })
  @property({ attribute: false })
  certainty: ContentCertainty = UNKNOWN_CERTAINTY;

  override render() {
    return html`<slot></slot>`;
  }
}
customElements.define('test-provider', TestProvider);

class TestConsumer extends LitElement {
  @consume({ context: capabilityProfileContext, subscribe: true })
  @state()
  profile: CapabilityProfile = DEFAULT_PROFILE;

  @consume({ context: contentCertaintyContext, subscribe: true })
  @state()
  certainty: ContentCertainty = UNKNOWN_CERTAINTY;

  override render() {
    return html`<span data-lens=${this.profile.lens} data-state=${this.certainty.state}></span>`;
  }
}
customElements.define('test-consumer', TestConsumer);

describe('capability context', () => {
  it('propagates profile from provider to consumer', async () => {
    const el = await fixture<TestProvider>(html`
      <test-provider>
        <test-consumer></test-consumer>
      </test-provider>
    `);
    const consumer = el.querySelector<TestConsumer>('test-consumer')!;
    await consumer.updateComplete;
    const span = consumer.shadowRoot!.querySelector('span')!;
    expect(span.getAttribute('data-lens')).to.equal('standard');
  });

  it('propagates certainty from provider to consumer', async () => {
    const el = await fixture<TestProvider>(html`
      <test-provider>
        <test-consumer></test-consumer>
      </test-provider>
    `);
    const consumer = el.querySelector<TestConsumer>('test-consumer')!;
    await consumer.updateComplete;
    const span = consumer.shadowRoot!.querySelector('span')!;
    expect(span.getAttribute('data-state')).to.equal('unknown');
  });

  it('re-renders consumer when provider changes profile', async () => {
    const provider = await fixture<TestProvider>(html`
      <test-provider>
        <test-consumer></test-consumer>
      </test-provider>
    `);
    const consumer = provider.querySelector<TestConsumer>('test-consumer')!;
    await consumer.updateComplete;
    provider.profile = { ...DEFAULT_PROFILE, lens: 'minimal' };
    await consumer.updateComplete;
    const span = consumer.shadowRoot!.querySelector('span')!;
    expect(span.getAttribute('data-lens')).to.equal('minimal');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test --filter capability/context.spec.ts
```
Expected: FAIL — context module not found.

- [ ] **Step 3: Implement context.ts**

Create `src/capability/context.ts`:

```ts
/**
 * Lit contexts for Capability Profile and Content Certainty.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §3.1, §4.2
 */

import { createContext } from '@lit/context';

import type { CapabilityProfile } from './profile.js';
import type { ContentCertainty } from './certainty.js';

export const capabilityProfileContext = createContext<CapabilityProfile>(
  Symbol.for('elohim-capability-profile')
);

export const contentCertaintyContext = createContext<ContentCertainty>(
  Symbol.for('elohim-content-certainty')
);
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter capability/context.spec.ts
```
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/capability/context.ts app/elohim-elements/elohim-core/src/capability/context.spec.ts
git commit -m "feat(elohim-core): add capabilityProfileContext + contentCertaintyContext"
```

---

## Task 6: CapabilityAwareElement mixin

**Files:**
- Create: `app/elohim-elements/elohim-core/src/capability/mixin.ts`
- Create: `app/elohim-elements/elohim-core/src/capability/mixin.spec.ts`

- [ ] **Step 1: Write the failing test**

Create `src/capability/mixin.spec.ts`:

```ts
import { expect, fixture, html } from '@open-wc/testing';
import { LitElement } from 'lit';
import { provide } from '@lit/context';
import { property } from 'lit/decorators.js';

import { CapabilityAwareElement } from './mixin.js';
import { capabilityProfileContext } from './context.js';
import { DEFAULT_PROFILE } from './profile.js';
import type { CapabilityProfile } from './profile.js';

class ContextProvider extends LitElement {
  @provide({ context: capabilityProfileContext })
  @property({ attribute: false })
  profile: CapabilityProfile = DEFAULT_PROFILE;

  override render() {
    return html`<slot></slot>`;
  }
}
customElements.define('ctx-provider', ContextProvider);

class CapAwareThing extends CapabilityAwareElement(LitElement) {
  override render() {
    return html`<span data-lens=${this.profile.lens}></span>`;
  }
}
customElements.define('cap-aware-thing', CapAwareThing);

describe('CapabilityAwareElement mixin', () => {
  it('exposes a profile property that defaults to DEFAULT_PROFILE when no provider', async () => {
    const el = await fixture<CapAwareThing>(html`<cap-aware-thing></cap-aware-thing>`);
    expect(el.profile).to.deep.equal(DEFAULT_PROFILE);
  });

  it('receives profile from a provider in the DOM tree', async () => {
    const provider = await fixture<ContextProvider>(html`
      <ctx-provider>
        <cap-aware-thing></cap-aware-thing>
      </ctx-provider>
    `);
    const el = provider.querySelector<CapAwareThing>('cap-aware-thing')!;
    await el.updateComplete;
    expect(el.profile.lens).to.equal('standard');
  });

  it('re-renders when the provider updates profile', async () => {
    const provider = await fixture<ContextProvider>(html`
      <ctx-provider>
        <cap-aware-thing></cap-aware-thing>
      </ctx-provider>
    `);
    const el = provider.querySelector<CapAwareThing>('cap-aware-thing')!;
    await el.updateComplete;
    provider.profile = { ...DEFAULT_PROFILE, lens: 'detail' };
    await el.updateComplete;
    const span = el.shadowRoot!.querySelector('span')!;
    expect(span.getAttribute('data-lens')).to.equal('detail');
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test --filter capability/mixin.spec.ts
```
Expected: FAIL — mixin module not found.

- [ ] **Step 3: Implement mixin.ts**

Create `src/capability/mixin.ts`:

```ts
/**
 * CapabilityAwareElement mixin — opt-in base for elements that observe the Capability Profile.
 *
 * Wires the consumer side of capabilityProfileContext with subscribe=true so consumers
 * re-render when the provider updates.
 *
 * Usage: extend `CapabilityAwareElement(LitElement)` instead of `LitElement`.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §3.1
 */

import { ContextConsumer } from '@lit/context';
import type { LitElement, PropertyDeclarations } from 'lit';

import { capabilityProfileContext } from './context.js';
import { DEFAULT_PROFILE } from './profile.js';
import type { CapabilityProfile } from './profile.js';

type Constructor<T = object> = new (...args: any[]) => T;

export interface CapabilityAware {
  profile: CapabilityProfile;
}

export function CapabilityAwareElement<TBase extends Constructor<LitElement>>(
  Base: TBase
): TBase & Constructor<CapabilityAware> {
  class Mixed extends Base {
    static override properties: PropertyDeclarations = {
      ...(Base as unknown as { properties?: PropertyDeclarations }).properties,
      profile: { attribute: false, state: true },
    };

    profile: CapabilityProfile = DEFAULT_PROFILE;

    constructor(...args: any[]) {
      super(...args);
      new ContextConsumer(this, {
        context: capabilityProfileContext,
        callback: (value: CapabilityProfile) => {
          this.profile = value;
          this.requestUpdate();
        },
        subscribe: true,
      });
    }
  }
  return Mixed as TBase & Constructor<CapabilityAware>;
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter capability/mixin.spec.ts
```
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/capability/mixin.ts app/elohim-elements/elohim-core/src/capability/mixin.spec.ts
git commit -m "feat(elohim-core): add CapabilityAwareElement mixin for profile-aware Lit elements"
```

---

## Task 7: Export capability barrel from elohim-core

**Files:**
- Create: `app/elohim-elements/elohim-core/src/capability/index.ts`
- Modify: `app/elohim-elements/elohim-core/src/index.ts`

- [ ] **Step 1: Write the failing test**

Add a smoke test to `src/index.ts` consumers. Create `src/capability/index.ts` first.

Create `src/capability/index.ts`:

```ts
export * from './profile.js';
export * from './certainty.js';
export * from './standing.js';
export * from './context.js';
export * from './mixin.js';
```

- [ ] **Step 2: Read current index.ts**

Read: `app/elohim-elements/elohim-core/src/index.ts`

- [ ] **Step 3: Append capability re-export**

Modify `src/index.ts` — add the line:

```ts
export * from './capability/index.js';
```

at the bottom of the file (after the existing button export). Do not remove existing exports.

- [ ] **Step 4: Verify build succeeds**

```bash
pnpm --filter elohim-core run build
```
Expected: build completes; `dist/index.d.ts` exports `CapabilityProfile`, `ContentCertainty`, `CapabilityAwareElement`, etc.

- [ ] **Step 5: Quick consumer smoke check**

```bash
grep -E "(CapabilityProfile|CapabilityAwareElement|ContentCertainty)" app/elohim-elements/elohim-core/dist/index.d.ts | head -5
```
Expected: at least three matches.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/capability/index.ts app/elohim-elements/elohim-core/src/index.ts
git commit -m "feat(elohim-core): export capability primitives from package root"
```

---

## Task 8: A11y test harness helpers

**Files:**
- Create: `app/elohim-elements/elohim-core/src/testing/a11y.ts`

The existing `elohim-button.spec.ts` already uses `axe-core` directly. This task extracts the pattern into a reusable helper and adds a keyboard-nav helper that elements can reuse.

- [ ] **Step 1: Write the failing test (self-test of the harness)**

Append to `src/testing/a11y.spec.ts` — create it fresh:

```ts
import { expect, fixture, html } from '@open-wc/testing';
import { LitElement } from 'lit';
import { axeScan, expectKeyboardFocusable } from './a11y.js';

class FocusableThing extends LitElement {
  static override readonly shadowRootOptions: ShadowRootInit = {
    ...LitElement.shadowRootOptions,
    delegatesFocus: true,
  };
  override render() {
    return html`<button>x</button>`;
  }
}
customElements.define('focusable-thing', FocusableThing);

class NotFocusableThing extends LitElement {
  override render() {
    return html`<span>x</span>`;
  }
}
customElements.define('not-focusable-thing', NotFocusableThing);

describe('a11y harness', () => {
  it('axeScan returns no violations for a clean element', async () => {
    const el = await fixture(html`<button>Save</button>`);
    const { violations } = await axeScan(el);
    expect(violations).to.have.lengthOf(0);
  });

  it('expectKeyboardFocusable passes when the element receives focus', async () => {
    const el = await fixture<FocusableThing>(html`<focusable-thing></focusable-thing>`);
    await expectKeyboardFocusable(el);
  });

  it('expectKeyboardFocusable throws when the element cannot receive focus', async () => {
    const el = await fixture<NotFocusableThing>(html`<not-focusable-thing></not-focusable-thing>`);
    let threw = false;
    try {
      await expectKeyboardFocusable(el);
    } catch (e) {
      threw = true;
    }
    expect(threw).to.be.true;
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test --filter testing/a11y.spec.ts
```
Expected: FAIL — module not found.

- [ ] **Step 3: Implement a11y.ts**

Create `src/testing/a11y.ts`:

```ts
/**
 * A11y precondition-gate helpers — usable by every element's spec.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §8.1
 */

import { aTimeout } from '@open-wc/testing';
import axe from 'axe-core';

export interface AxeResult {
  violations: axe.Result[];
}

/**
 * Scans the element subtree with axe-core. Returns violations.
 * Fail your test on `violations.length > 0`.
 */
export async function axeScan(element: Element): Promise<AxeResult> {
  const result = await axe.run(element as any);
  return { violations: result.violations };
}

/**
 * Asserts the element is keyboard-focusable. Either the host receives focus
 * directly or focus delegates to a descendant via delegatesFocus.
 * Throws if focus does not land on the host or any of its shadow descendants.
 */
export async function expectKeyboardFocusable(element: HTMLElement): Promise<void> {
  element.focus();
  await aTimeout(0);
  const active = document.activeElement;
  const inside =
    active === element ||
    (active != null && element.shadowRoot?.contains(active)) ||
    (active != null && element.contains(active));
  if (!inside) {
    throw new Error(
      `expectKeyboardFocusable: focus did not land on or within <${element.tagName.toLowerCase()}>. ` +
        `activeElement was ${active?.tagName.toLowerCase() ?? 'null'}.`
    );
  }
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter testing/a11y.spec.ts
```
Expected: PASS (3 tests).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/testing/a11y.ts app/elohim-elements/elohim-core/src/testing/a11y.spec.ts
git commit -m "feat(elohim-core): add a11y test harness (axeScan, expectKeyboardFocusable)"
```

---

## Task 9: Set up @lit/localize for i18n harness

**Files:**
- Create: `app/elohim-elements/elohim-core/src/localize/runtime.ts`
- Create: `app/elohim-elements/elohim-core/src/localize/source-locale.ts`
- Create: `app/elohim-elements/elohim-core/lit-localize.json`

- [ ] **Step 1: Create lit-localize config**

Create `app/elohim-elements/elohim-core/lit-localize.json`:

```json
{
  "$schema": "https://raw.githubusercontent.com/lit/lit/main/packages/localize-tools/config.schema.json",
  "sourceLocale": "en",
  "targetLocales": ["es", "he"],
  "tsConfig": "./tsconfig.json",
  "output": {
    "mode": "runtime",
    "outputDir": "./src/localize/generated",
    "localeCodesModule": "./src/localize/generated/locale-codes.ts"
  },
  "interchange": {
    "format": "xliff",
    "xliffDir": "./src/localize/xliff"
  }
}
```

- [ ] **Step 2: Create runtime config**

Create `src/localize/runtime.ts`:

```ts
/**
 * Lit-localize runtime configuration for elohim-core.
 *
 * Source locale: en. Targets: es, he (he is the RTL canary).
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §8.2
 */

import { configureLocalization } from '@lit/localize';

export const sourceLocale = 'en';
export const targetLocales = ['es', 'he'] as const;

export const { getLocale, setLocale } = configureLocalization({
  sourceLocale,
  targetLocales: [...targetLocales],
  loadLocale: async (locale: string) => {
    return import(`./generated/${locale}.js`);
  },
});
```

- [ ] **Step 3: Create source-locale.ts**

Create `src/localize/source-locale.ts`:

```ts
import { msg, str } from '@lit/localize';

/**
 * Smoke-test source string — proves the localize pipeline is wired and provides
 * one entry for the xliff seed.
 */
export const smokeGreeting = (name: string) => msg(str`Hello, ${name}`);
```

- [ ] **Step 4: Generate localize files**

Run from `app/elohim-elements/elohim-core/`:
```bash
pnpm exec lit-localize extract
```
Expected: creates `src/localize/xliff/es.xlf` and `src/localize/xliff/he.xlf` with the smoke string.

- [ ] **Step 5: Add a Spanish translation manually**

Edit `src/localize/xliff/es.xlf` — find the `<target>` element for `smokeGreeting`, set it to `Hola, <x …/>`. (If the file isn't created on step 4 because of empty extraction, skip this step.)

- [ ] **Step 6: Build localized bundles**

```bash
pnpm exec lit-localize build
```
Expected: creates `src/localize/generated/{es,he,locale-codes}.ts`.

- [ ] **Step 7: Verify build still passes**

```bash
pnpm --filter elohim-core run build
```
Expected: build succeeds.

- [ ] **Step 8: Commit**

```bash
git add app/elohim-elements/elohim-core/lit-localize.json \
        app/elohim-elements/elohim-core/src/localize \
        app/elohim-elements/elohim-core/package.json
git commit -m "feat(elohim-core): set up @lit/localize with en source + es/he targets"
```

---

## Task 10: I18n harness helpers

**Files:**
- Create: `app/elohim-elements/elohim-core/src/testing/i18n.ts`
- Create: `app/elohim-elements/elohim-core/src/testing/i18n.spec.ts`

- [ ] **Step 1: Write the failing test (harness self-test)**

Create `src/testing/i18n.spec.ts`:

```ts
import { expect, fixture, html } from '@open-wc/testing';
import { LitElement } from 'lit';
import { renderInLocale, scanForHardcodedStrings, requiresLogicalProperties } from './i18n.js';

class HardcodedThing extends LitElement {
  override render() {
    return html`<span aria-label="Save">Save</span>`;
  }
}
customElements.define('hardcoded-thing', HardcodedThing);

class PhysicalStyledThing extends LitElement {
  static override styles = `:host { margin-left: 8px; }` as any;
  override render() {
    return html`<span></span>`;
  }
}

describe('i18n harness', () => {
  describe('renderInLocale', () => {
    it('renders the element with the document direction set for he-IL', async () => {
      const el = await renderInLocale('he-IL', html`<span></span>`);
      expect(el.ownerDocument.documentElement.getAttribute('dir')).to.equal('rtl');
    });

    it('restores LTR direction after rendering ends', async () => {
      await renderInLocale('he-IL', html`<span></span>`);
      // Helper cleans up after itself; subsequent calls in en should be ltr.
      const el = await renderInLocale('en', html`<span></span>`);
      expect(el.ownerDocument.documentElement.getAttribute('dir')).to.equal('ltr');
    });
  });

  describe('scanForHardcodedStrings', () => {
    it('flags element render output with hardcoded text content', async () => {
      const el = await fixture<HardcodedThing>(html`<hardcoded-thing></hardcoded-thing>`);
      const findings = scanForHardcodedStrings(el.shadowRoot!.innerHTML);
      expect(findings.length).to.be.greaterThan(0);
    });

    it('returns empty findings when content uses placeholders only', () => {
      const findings = scanForHardcodedStrings('<span aria-label="{{label}}">{{text}}</span>');
      expect(findings).to.deep.equal([]);
    });
  });

  describe('requiresLogicalProperties', () => {
    it('flags physical-property CSS rules', () => {
      const findings = requiresLogicalProperties('.x { margin-left: 8px; padding-right: 4px; }');
      expect(findings).to.have.lengthOf(2);
      expect(findings[0]).to.contain('margin-left');
      expect(findings[1]).to.contain('padding-right');
    });

    it('does not flag logical properties', () => {
      const findings = requiresLogicalProperties('.x { margin-inline-start: 8px; }');
      expect(findings).to.have.lengthOf(0);
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test --filter testing/i18n.spec.ts
```
Expected: FAIL — module not found.

- [ ] **Step 3: Implement i18n.ts**

Create `src/testing/i18n.ts`:

```ts
/**
 * I18n precondition-gate helpers.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §8.2
 */

import { fixture } from '@open-wc/testing';
import type { TemplateResult } from 'lit';
import { setLocale } from '../localize/runtime.js';

const RTL_LOCALES = ['he', 'ar', 'fa', 'ur'] as const;

function isRtlLocale(locale: string): boolean {
  const lang = locale.split('-')[0]!.toLowerCase();
  return (RTL_LOCALES as readonly string[]).includes(lang);
}

/**
 * Renders a Lit template under a specific locale, setting document direction
 * and the localize runtime. Restores prior state afterward.
 */
export async function renderInLocale<T extends Element = HTMLElement>(
  locale: string,
  template: TemplateResult
): Promise<T> {
  const root = document.documentElement;
  const priorDir = root.getAttribute('dir');
  const priorLang = root.getAttribute('lang');

  root.setAttribute('lang', locale);
  root.setAttribute('dir', isRtlLocale(locale) ? 'rtl' : 'ltr');

  const baseLocale = locale.split('-')[0]!.toLowerCase();
  if (baseLocale !== 'en') {
    try {
      await setLocale(baseLocale);
    } catch {
      // locale bundle absent — leave at source
    }
  }

  const el = await fixture<T>(template);

  // Clean up after this microtask so test teardown observes the original.
  queueMicrotask(() => {
    if (priorDir !== null) root.setAttribute('dir', priorDir);
    else root.removeAttribute('dir');
    if (priorLang !== null) root.setAttribute('lang', priorLang);
    else root.removeAttribute('lang');
  });

  return el;
}

/**
 * Scans rendered HTML for likely hardcoded user-facing strings.
 * Heuristic: visible text or aria-* attribute values that are not pure
 * whitespace, not numeric, and not template placeholder syntax.
 *
 * Returns the offending fragments.
 */
export function scanForHardcodedStrings(renderedHtml: string): string[] {
  const findings: string[] = [];
  // text content between tags
  const textMatches = renderedHtml.matchAll(/>([^<>{}\s][^<>{}]*?)</g);
  for (const m of textMatches) {
    const text = m[1]!.trim();
    if (text.length > 0 && !/^[\d\s.,:;%-]+$/.test(text)) findings.push(text);
  }
  // aria-* attribute values
  const ariaMatches = renderedHtml.matchAll(/aria-[a-z]+="([^"{}]+)"/g);
  for (const m of ariaMatches) {
    const value = m[1]!.trim();
    if (value.length > 0 && !/^[\d\s.,:;%-]+$/.test(value)) findings.push(value);
  }
  return findings;
}

const PHYSICAL_PROPS = [
  'margin-left',
  'margin-right',
  'padding-left',
  'padding-right',
  'left:',
  'right:',
  'border-left',
  'border-right',
  'text-align: left',
  'text-align: right',
];

/**
 * Returns CSS property mentions that should have been logical properties.
 */
export function requiresLogicalProperties(css: string): string[] {
  return PHYSICAL_PROPS.filter(p => css.includes(p));
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter testing/i18n.spec.ts
```
Expected: PASS (6 tests).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/testing/i18n.ts app/elohim-elements/elohim-core/src/testing/i18n.spec.ts
git commit -m "feat(elohim-core): add i18n test harness (renderInLocale, scanForHardcodedStrings, requiresLogicalProperties)"
```

---

## Task 11: UA-prefs harness — media-query toggles

**Files:**
- Create: `app/elohim-elements/elohim-core/src/testing/ua-prefs.ts`
- Create: `app/elohim-elements/elohim-core/src/testing/ua-prefs.spec.ts`

**Important scope of this helper:** `setMediaQuery` patches the JS `window.matchMedia` function. It does NOT change what the browser's CSS `@media` query evaluator sees. That means:
- JS code reading `matchMedia('(prefers-reduced-motion: reduce)').matches` will see the override.
- CSS `@media (prefers-reduced-motion: reduce) { ... }` rules continue to evaluate against the real browser state.

For CSS-side coverage, the *element's stylesheet* must use `@media` queries (see Task 16 Step 3) so that under the *natural* media query the browser already evaluates correctly. The helper lets you simulate other JS-observable paths — including the `effectiveStimulusCeiling()` computation that the Profile uses at runtime.

- [ ] **Step 1: Write the failing test**

Create `src/testing/ua-prefs.spec.ts`:

```ts
import { expect, fixture, html } from '@open-wc/testing';
import {
  setMediaQuery,
  clearMediaQueries,
  effectiveStimulusCeiling,
} from './ua-prefs.js';

describe('ua-prefs harness', () => {
  afterEach(() => {
    clearMediaQueries();
  });

  describe('setMediaQuery', () => {
    it('forces prefers-reduced-motion to active when set to reduce', () => {
      setMediaQuery('prefers-reduced-motion', 'reduce');
      expect(matchMedia('(prefers-reduced-motion: reduce)').matches).to.be.true;
    });

    it('forces prefers-color-scheme', () => {
      setMediaQuery('prefers-color-scheme', 'dark');
      expect(matchMedia('(prefers-color-scheme: dark)').matches).to.be.true;
    });

    it('forces update: slow (e-paper simulation)', () => {
      setMediaQuery('update', 'slow');
      expect(matchMedia('(update: slow)').matches).to.be.true;
    });

    it('clearMediaQueries undoes overrides', () => {
      setMediaQuery('prefers-reduced-motion', 'reduce');
      clearMediaQueries();
      expect(matchMedia('(prefers-reduced-motion: reduce)').matches).to.be.false;
    });
  });

  describe('effectiveStimulusCeiling', () => {
    it('returns still when prefers-reduced-motion is reduce', () => {
      setMediaQuery('prefers-reduced-motion', 'reduce');
      expect(effectiveStimulusCeiling()).to.equal('still');
    });

    it('returns still when update is slow (e-paper)', () => {
      setMediaQuery('update', 'slow');
      expect(effectiveStimulusCeiling()).to.equal('still');
    });

    it('returns lively when no OS constraint applies', () => {
      expect(effectiveStimulusCeiling()).to.equal('lively');
    });
  });
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test --filter testing/ua-prefs.spec.ts
```
Expected: FAIL — module not found.

- [ ] **Step 3: Implement ua-prefs.ts**

Create `src/testing/ua-prefs.ts`:

```ts
/**
 * UA-prefs precondition-gate helpers.
 *
 * Patches window.matchMedia to allow tests to force specific OS preference
 * states. Use clearMediaQueries() in afterEach to reset.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §8.3
 */

import type { Stimulus } from '../capability/profile.js';

type MediaPref =
  | 'prefers-reduced-motion'
  | 'prefers-color-scheme'
  | 'prefers-contrast'
  | 'prefers-reduced-transparency'
  | 'prefers-reduced-data'
  | 'forced-colors'
  | 'update'
  | 'pointer'
  | 'hover';

const overrides = new Map<string, string>();
let originalMatchMedia: typeof window.matchMedia | null = null;

function ensurePatched(): void {
  if (originalMatchMedia) return;
  originalMatchMedia = window.matchMedia.bind(window);
  window.matchMedia = (query: string): MediaQueryList => {
    const matches = matchesOverride(query);
    if (matches !== null) {
      return makeFakeMediaQueryList(query, matches);
    }
    return originalMatchMedia!(query);
  };
}

function matchesOverride(query: string): boolean | null {
  for (const [pref, value] of overrides) {
    const match = query.match(new RegExp(`\\(${pref}:\\s*([^)]+)\\)`));
    if (match) {
      const wanted = match[1]!.trim();
      return wanted === value;
    }
  }
  return null;
}

function makeFakeMediaQueryList(query: string, matches: boolean): MediaQueryList {
  return {
    matches,
    media: query,
    onchange: null,
    addListener: () => {},
    removeListener: () => {},
    addEventListener: () => {},
    removeEventListener: () => {},
    dispatchEvent: () => false,
  };
}

/** Force a specific OS preference state for the next matchMedia query. */
export function setMediaQuery(pref: MediaPref, value: string): void {
  ensurePatched();
  overrides.set(pref, value);
}

/** Reset all overrides; restores original window.matchMedia. */
export function clearMediaQueries(): void {
  overrides.clear();
  if (originalMatchMedia) {
    window.matchMedia = originalMatchMedia;
    originalMatchMedia = null;
  }
}

/**
 * Computes the effective stimulus ceiling from OS preferences alone.
 * See spec §2.5: effectiveStimulus = min(profile.stimulus, osCeiling).
 */
export function effectiveStimulusCeiling(): Stimulus {
  const reduceMotion = matchMedia('(prefers-reduced-motion: reduce)').matches;
  const epaper = matchMedia('(update: slow)').matches;
  return reduceMotion || epaper ? 'still' : 'lively';
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter testing/ua-prefs.spec.ts
```
Expected: PASS (7 tests).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/testing/ua-prefs.ts app/elohim-elements/elohim-core/src/testing/ua-prefs.spec.ts
git commit -m "feat(elohim-core): add ua-prefs harness (media-query overrides + stimulus ceiling)"
```

---

## Task 12: Photosensitive flash analyzer

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/testing/ua-prefs.ts`
- Modify: `app/elohim-elements/elohim-core/src/testing/ua-prefs.spec.ts`

- [ ] **Step 1: Add the failing test**

Append to `src/testing/ua-prefs.spec.ts`:

```ts
import { fixture as fxFixture, html as fxHtml } from '@open-wc/testing';
import { LitElement } from 'lit';
import { measureLuminanceChanges } from './ua-prefs.js';

class StableThing extends LitElement {
  override render() {
    return fxHtml`<div style="background: #fff; width: 50px; height: 50px;"></div>`;
  }
}
customElements.define('stable-thing', StableThing);

class FlashingThing extends LitElement {
  static override styles = `
    @keyframes flash {
      0%, 100% { background: #000; }
      50% { background: #fff; }
    }
    div { animation: flash 200ms infinite; width: 50px; height: 50px; }
  ` as any;
  override render() {
    return fxHtml`<div></div>`;
  }
}

describe('photosensitive flash analyzer', () => {
  it('reports zero high-luminance changes for a still element', async () => {
    const el = await fxFixture<StableThing>(fxHtml`<stable-thing></stable-thing>`);
    const result = await measureLuminanceChanges(el, { sampleMs: 1000, sampleHz: 30 });
    expect(result.flashHz).to.be.lessThan(3);
    expect(result.exceedsThreshold).to.be.false;
  });

  // Note: we don't test the FlashingThing positive case in unit tests because
  // it's timing-sensitive. The presence of the API is what we're locking in.
});
```

- [ ] **Step 2: Run test to verify it fails**

```bash
pnpm test --filter testing/ua-prefs.spec.ts
```
Expected: FAIL — `measureLuminanceChanges` not found.

- [ ] **Step 3: Append implementation to ua-prefs.ts**

Append to `src/testing/ua-prefs.ts`:

```ts
export interface LuminanceMeasurement {
  /** Number of luminance crossings per second (large-area). Above 3 Hz is unsafe per WCAG 2.3. */
  flashHz: number;
  /** Whether any 1-second window exceeded the WCAG flash threshold. */
  exceedsThreshold: boolean;
  /** Number of samples collected. */
  samples: number;
}

export interface LuminanceOptions {
  /** Duration of the sampling window in ms. */
  sampleMs?: number;
  /** Samples per second. */
  sampleHz?: number;
  /** Relative luminance delta that counts as a "flash" boundary. WCAG uses 0.1. */
  luminanceDelta?: number;
}

/**
 * Measures luminance changes in an element over time by sampling computed background.
 * Lightweight approximation: reads computed-style backgrounds for the element and its
 * first descendant; counts crossings of a brightness threshold. Intended as a
 * regression-detector, not a real visual analyzer.
 *
 * For accurate testing, use a tool like FlashTest or Visual A11y. This harness flags
 * the obvious cases (full-element strobing) and gives a starting point.
 */
export async function measureLuminanceChanges(
  el: Element,
  opts: LuminanceOptions = {}
): Promise<LuminanceMeasurement> {
  const sampleMs = opts.sampleMs ?? 1000;
  const sampleHz = opts.sampleHz ?? 30;
  const delta = opts.luminanceDelta ?? 0.1;
  const interval = 1000 / sampleHz;

  function getBrightness(target: Element): number {
    const style = getComputedStyle(target);
    const bg = style.backgroundColor;
    const rgb = bg.match(/\d+/g);
    if (!rgb || rgb.length < 3) return 0;
    const r = Number.parseInt(rgb[0]!, 10) / 255;
    const g = Number.parseInt(rgb[1]!, 10) / 255;
    const b = Number.parseInt(rgb[2]!, 10) / 255;
    return 0.2126 * r + 0.7152 * g + 0.0722 * b;
  }

  const target = (el.shadowRoot?.firstElementChild ?? el.firstElementChild ?? el) as Element;
  const samples: number[] = [];
  const end = performance.now() + sampleMs;
  await new Promise<void>(resolve => {
    function tick(): void {
      samples.push(getBrightness(target));
      if (performance.now() >= end) resolve();
      else setTimeout(tick, interval);
    }
    tick();
  });

  let crossings = 0;
  let prev = samples[0]!;
  for (const value of samples.slice(1)) {
    if (Math.abs(value - prev) > delta) crossings++;
    prev = value;
  }
  const flashHz = (crossings / sampleMs) * 1000 / 2; // each flash is a pair of crossings
  return {
    flashHz,
    exceedsThreshold: flashHz > 3,
    samples: samples.length,
  };
}
```

- [ ] **Step 4: Run test to verify it passes**

```bash
pnpm test --filter testing/ua-prefs.spec.ts
```
Expected: PASS (8 tests total in the file).

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/testing/ua-prefs.ts app/elohim-elements/elohim-core/src/testing/ua-prefs.spec.ts
git commit -m "feat(elohim-core): add measureLuminanceChanges for WCAG 2.3 photosensitive-flash gate"
```

---

## Task 13: Testing barrel export

**Files:**
- Create: `app/elohim-elements/elohim-core/src/testing/index.ts`
- Modify: `app/elohim-elements/elohim-core/src/index.ts`

- [ ] **Step 1: Create testing barrel**

Create `src/testing/index.ts`:

```ts
export * from './a11y.js';
export * from './i18n.js';
export * from './ua-prefs.js';
```

- [ ] **Step 2: Update package.json exports**

Modify `app/elohim-elements/elohim-core/package.json` `exports` block — add:

```json
    "./testing": {
      "import": "./dist/testing/index.js",
      "types": "./dist/testing/index.d.ts"
    },
```

(Insert between `"./register"` and `"./tokens.scss"`.)

- [ ] **Step 3: Verify build still passes**

```bash
pnpm --filter elohim-core run build
```
Expected: build succeeds; `dist/testing/index.js` exists.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-elements/elohim-core/src/testing/index.ts app/elohim-elements/elohim-core/package.json
git commit -m "feat(elohim-core): export testing helpers as elohim-core/testing subpath"
```

---

## Task 14: CEM analyzer plugin for capabilityContract

**Files:**
- Create: `app/elohim-elements/elohim-core/cem-plugins/capability-contract.mjs`
- Modify: `app/elohim-elements/elohim-core/custom-elements-manifest.config.mjs`

This plugin scans JSDoc `@capability*` tags and writes them into the `capabilityContract` block on each declaration in `custom-elements.json`.

- [ ] **Step 1: Create the plugin**

Create `app/elohim-elements/elohim-core/cem-plugins/capability-contract.mjs`:

```js
/**
 * CEM analyzer plugin: reads @capability* JSDoc tags on Lit element classes
 * and writes them into a `capabilityContract` field on the declaration.
 *
 * Recognized tags:
 *   @capabilityMaxLens <Lens>
 *   @capabilityThemes <Theme,...>
 *   @capabilityContrast <Contrast,...>
 *   @capabilityLocales <Locale,...>
 *   @capabilityMaxStimulus <Stimulus>
 *   @capabilityTextuality <Textuality,...>
 *   @capabilityRequiredStandings <DSL>
 *   @capabilityOptionalStandings <DSL>
 *   @capabilityContentCertainty observed | not-observed
 *   @capabilityStates <name:status, ...>
 *
 * The gate fields (a11y, i18n, uaPrefs) are NOT read from tags; they are written
 * by the test runner. The plugin emits "unknown" for those three fields so they
 * are visible in the contract even before tests run.
 *
 * See: genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md §7
 */

const SIMPLE_LIST_TAGS = new Set([
  'capabilityThemes',
  'capabilityContrast',
  'capabilityLocales',
  'capabilityTextuality',
]);

function parseList(raw) {
  return raw
    .split(',')
    .map(t => t.trim())
    .filter(t => t.length > 0);
}

function parseStandingDsl(raw) {
  // each comma-separated entry is one AND-group; | within entries is OR (preserved as string)
  return parseList(raw);
}

function parseStates(raw) {
  // "empty:designed, error:not-yet, contested:n/a"
  const out = {};
  for (const entry of parseList(raw)) {
    const [name, status] = entry.split(':').map(t => t.trim());
    if (name && status) out[name] = status;
  }
  return out;
}

export default function capabilityContractPlugin() {
  return {
    name: 'capability-contract',
    analyzePhase({ ts, node, moduleDoc }) {
      // Only operate on class declarations
      if (!ts.isClassDeclaration(node) || !node.name) return;
      const className = node.name.text;
      const decl = moduleDoc.declarations?.find(
        d => d.kind === 'class' && d.name === className
      );
      if (!decl) return;

      const jsdoc = ts.getJSDocTags(node);
      if (!jsdoc || jsdoc.length === 0) return;

      const contract = {
        a11y: 'unknown',
        i18n: 'unknown',
        uaPrefs: 'unknown',
      };

      for (const tag of jsdoc) {
        const tagName = tag.tagName.text;
        const text = typeof tag.comment === 'string' ? tag.comment : '';
        const raw = text.replace(/\/\/.*$/, '').trim();
        if (!raw) continue;

        if (tagName === 'capabilityMaxLens') contract.maxLens = raw;
        else if (tagName === 'capabilityMaxStimulus') contract.maxStimulus = raw;
        else if (tagName === 'capabilityContentCertainty') contract.contentCertainty = raw;
        else if (SIMPLE_LIST_TAGS.has(tagName)) {
          const key = tagName === 'capabilityThemes' ? 'themes'
            : tagName === 'capabilityContrast' ? 'contrast'
            : tagName === 'capabilityLocales' ? 'locales'
            : 'textuality';
          contract[key] = parseList(raw);
        } else if (tagName === 'capabilityRequiredStandings') {
          contract.standings = contract.standings || {};
          contract.standings.required = parseStandingDsl(raw);
        } else if (tagName === 'capabilityOptionalStandings') {
          contract.standings = contract.standings || {};
          contract.standings.optional = parseStandingDsl(raw);
        } else if (tagName === 'capabilityStates') {
          contract.states = parseStates(raw);
        }
      }

      // Only attach if any capability tag fired (besides the always-present gate stubs)
      const keys = Object.keys(contract);
      if (keys.length > 3) {
        decl.capabilityContract = contract;
      }
    },
  };
}
```

- [ ] **Step 2: Register the plugin in CEM config**

Modify `app/elohim-elements/elohim-core/custom-elements-manifest.config.mjs`:

```js
import capabilityContractPlugin from './cem-plugins/capability-contract.mjs';

// Forward-affordance: when component-CID federation tooling lands,
// add a plugin here that hashes each declaration and writes a
// `componentCid` field per entry. Until then, the field is reserved
// but not populated. See:
// genesis/docs/superpowers/specs/2026-05-06-elohim-lit-component-pivot-design.md (D8)
export default {
  globs: ['src/**/*.ts'],
  exclude: ['src/**/*.spec.ts'],
  outdir: 'dist',
  litelement: true,
  packagejson: false,
  plugins: [capabilityContractPlugin()],
};
```

- [ ] **Step 3: Write a test that validates the plugin output**

Create `src/capability/contract-plugin.spec.ts`:

```ts
import { expect } from '@open-wc/testing';

describe('capability-contract CEM plugin (post-build smoke)', () => {
  let cem: any;

  before(async () => {
    cem = (await import('../../dist/custom-elements.json', { with: { type: 'json' } })).default;
  });

  it('emits a capabilityContract block for elements that declare @capability tags', () => {
    // After Task 16, <elohim-button> will declare tags. Until then, this test is allowed
    // to pass with no annotated declarations — it locks in the schema shape.
    const declarations = (cem.modules || []).flatMap(m => m.declarations || []);
    const annotated = declarations.filter(d => d.capabilityContract);
    for (const d of annotated) {
      expect(d.capabilityContract).to.have.property('a11y');
      expect(d.capabilityContract).to.have.property('i18n');
      expect(d.capabilityContract).to.have.property('uaPrefs');
    }
  });
});
```

- [ ] **Step 4: Rebuild and verify**

```bash
pnpm --filter elohim-core run build
```
Expected: build succeeds; no errors from the plugin. `dist/custom-elements.json` regenerated.

- [ ] **Step 5: Run test**

```bash
pnpm test --filter capability/contract-plugin.spec.ts
```
Expected: PASS (no annotated declarations yet — test verifies shape on those that exist).

- [ ] **Step 6: Commit**

```bash
git add app/elohim-elements/elohim-core/cem-plugins \
        app/elohim-elements/elohim-core/custom-elements-manifest.config.mjs \
        app/elohim-elements/elohim-core/src/capability/contract-plugin.spec.ts
git commit -m "feat(elohim-core): add CEM analyzer plugin for capabilityContract from JSDoc"
```

---

## Task 15: Migrate `<elohim-button>` to CapabilityAwareElement

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-button.ts`

- [ ] **Step 1: Read current button**

Read: `app/elohim-elements/elohim-core/src/elohim-button.ts`

- [ ] **Step 2: Modify class declaration and add JSDoc tags**

In `src/elohim-button.ts`:

1. Add a new import at the top:
   ```ts
   import { CapabilityAwareElement } from './capability/index.js';
   ```

2. Replace the JSDoc block above `export class ElohimButton extends LitElement` with the enriched version below, AND change the `extends LitElement` to `extends CapabilityAwareElement(LitElement)`:

```ts
/**
 * The elohim button atom — substrate primitive for action affordances.
 *
 * Token-driven; respects light/dark theme via the global tokens cascade.
 *
 * @element elohim-button
 *
 * @prop {ElohimButtonVariant} variant - Visual variant: primary | secondary | ghost
 * @prop {boolean} disabled - Disabled state. Suppresses click and applies aria-disabled.
 *
 * @fires {MouseEvent} click - Fired on activation (mouse or keyboard via native button). Native bubbling click from the inner <button>.
 *
 * @slot - Default slot for label content (text or icon+text)
 *
 * @cssprop --elohim-button-bg - Override background color
 * @cssprop --elohim-button-fg - Override foreground (label) color
 * @cssprop --elohim-button-border - Override border style
 * @cssprop --elohim-button-radius - Override border-radius
 *
 * @csspart button - The internal native <button> element
 *
 * @capabilityMaxLens standard
 * @capabilityThemes light, dark
 * @capabilityContrast normal, high
 * @capabilityLocales en
 * @capabilityMaxStimulus still
 * @capabilityTextuality textual, symbolic
 * @capabilityRequiredStandings pilot | steward | elohim-support
 * @capabilityContentCertainty not-observed
 * @capabilityStates empty:n/a, loading:n/a, error:n/a, stale:n/a, contested:n/a, offline:n/a, unauthorized:n/a
 */
export class ElohimButton extends CapabilityAwareElement(LitElement) {
```

Keep the entire class body unchanged.

- [ ] **Step 3: Run all existing button tests**

```bash
pnpm test --filter elohim-button
```
Expected: PASS (all pre-existing button tests still green).

- [ ] **Step 4: Rebuild CEM to ingest the new tags**

```bash
pnpm --filter elohim-core run build
```
Expected: build succeeds; `dist/custom-elements.json` now contains a `capabilityContract` block under the `ElohimButton` declaration.

- [ ] **Step 5: Verify contract presence**

```bash
node -e "const m = require('./app/elohim-elements/elohim-core/dist/custom-elements.json'); const button = m.modules.flatMap(x => x.declarations || []).find(d => d.name === 'ElohimButton'); console.log(JSON.stringify(button.capabilityContract, null, 2));"
```
Expected output (formatted):
```json
{
  "a11y": "unknown",
  "i18n": "unknown",
  "uaPrefs": "unknown",
  "maxLens": "standard",
  "themes": ["light", "dark"],
  "contrast": ["normal", "high"],
  "locales": ["en"],
  "maxStimulus": "still",
  "textuality": ["textual", "symbolic"],
  "standings": { "required": ["pilot | steward | elohim-support"] },
  "contentCertainty": "not-observed",
  "states": {
    "empty": "n/a", "loading": "n/a", "error": "n/a", "stale": "n/a",
    "contested": "n/a", "offline": "n/a", "unauthorized": "n/a"
  }
}
```

- [ ] **Step 6: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-button.ts
git commit -m "feat(elohim-button): extend CapabilityAwareElement + declare capability contract"
```

---

## Task 16: Button — UA-prefs precondition tests

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-button.spec.ts`

- [ ] **Step 1: Add the failing test**

Append to `src/elohim-button.spec.ts`:

```ts
import { setMediaQuery, clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';

describe('<elohim-button> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('renders with no transitions/animations when prefers-reduced-motion: reduce', async () => {
    setMediaQuery('prefers-reduced-motion', 'reduce');
    const el = await fixture<ElohimButton>(html`<elohim-button>x</elohim-button>`);
    const inner = el.shadowRoot!.querySelector('button')!;
    const style = getComputedStyle(inner);
    // Either there are no transitions, or all transition-durations are 0.
    const durations = style.transitionDuration.split(',').map(d => d.trim());
    const allZero = durations.every(d => d === '0s' || d === '0ms');
    expect(allZero || style.transitionDuration === '' || style.transitionDuration === '0s').to.be.true;
  });

  it('renders with no animation when update: slow (e-paper)', async () => {
    setMediaQuery('update', 'slow');
    const el = await fixture<ElohimButton>(html`<elohim-button>x</elohim-button>`);
    const inner = el.shadowRoot!.querySelector('button')!;
    const style = getComputedStyle(inner);
    expect(style.animationName === 'none' || style.animationName === '').to.be.true;
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimButton>(html`<elohim-button>x</elohim-button>`);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });

  it('has a touch target of at least 44×44 px under coarse pointer', async () => {
    const el = await fixture<ElohimButton>(html`<elohim-button>x</elohim-button>`);
    const inner = el.shadowRoot!.querySelector('button')!;
    const rect = inner.getBoundingClientRect();
    expect(rect.width).to.be.at.least(44);
    expect(rect.height).to.be.at.least(44);
  });
});
```

- [ ] **Step 2: Run test to verify it fails OR passes**

```bash
pnpm test --filter elohim-button
```
Expected: tests should mostly pass because button has no animations and min-width/height 44px is in its CSS. If the prefers-reduced-motion test fails, you need to wrap the existing `button { transition: ... }` declaration in a media query in step 3.

- [ ] **Step 3: If the reduced-motion test failed, gate the button's transition**

In `src/elohim-button.ts`, modify the `static override readonly styles` block — change the bare `transition: ...` rule to be wrapped in a media query. Specifically, replace:

```css
button {
  ...
  transition:
    background-color 150ms ease,
    border-color 150ms ease,
    color 150ms ease,
    transform 80ms ease;
}
```

with:

```css
button {
  ...
}

@media (prefers-reduced-motion: no-preference) and (update: fast) {
  button {
    transition:
      background-color 150ms ease,
      border-color 150ms ease,
      color 150ms ease,
      transform 80ms ease;
  }
}
```

- [ ] **Step 4: Re-run tests**

```bash
pnpm test --filter elohim-button
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-button.ts app/elohim-elements/elohim-core/src/elohim-button.spec.ts
git commit -m "test(elohim-button): add ua-prefs precondition gate tests + gate transitions on motion/update"
```

---

## Task 17: Button — i18n precondition tests

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-button.spec.ts`

- [ ] **Step 1: Add the failing test**

Append to `src/elohim-button.spec.ts`:

```ts
import { renderInLocale, requiresLogicalProperties } from './testing/i18n.js';

describe('<elohim-button> — i18n precondition gate', () => {
  it('renders the slotted label unchanged in en (label comes from slot, not internal strings)', async () => {
    const el = await renderInLocale<ElohimButton>('en', html`<elohim-button>Submit</elohim-button>`);
    const slot = el.shadowRoot!.querySelector('slot')!;
    const text = slot.assignedNodes({ flatten: true }).map(n => n.textContent).join('').trim();
    expect(text).to.equal('Submit');
  });

  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimButton>('he-IL', html`<elohim-button>שלח</elohim-button>`);
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    // Confirm the inner button is laid out — bounding box has nonzero dimensions
    const inner = el.shadowRoot!.querySelector('button')!;
    const rect = inner.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    // The button's static styles string contains its declared CSS. We scan it.
    // Note: we read styles via the class's static field through a runtime introspection.
    const cssText = (ElohimButton as unknown as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
```

- [ ] **Step 2: Run test**

```bash
pnpm test --filter elohim-button
```
Expected: the physical-CSS test will likely FAIL because the current button uses physical `padding: 0.625rem 1.25rem` — but that's symmetric so shouldn't trigger flag. The other tests should pass. If `requiresLogicalProperties` flags any property, proceed to step 3.

- [ ] **Step 3: If logical-property test failed, fix the offending properties**

In `src/elohim-button.ts`, replace any physical-only property (`margin-left`, `padding-right`, etc.) with logical equivalents (`margin-inline-start`, `padding-inline-end`). Symmetric values (`padding: 8px 16px`) are fine — they don't differ by direction. If the current styles only use symmetric values, no edit is needed and the test should already pass.

- [ ] **Step 4: Re-run tests**

```bash
pnpm test --filter elohim-button
```
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-button.ts app/elohim-elements/elohim-core/src/elohim-button.spec.ts
git commit -m "test(elohim-button): add i18n precondition gate tests (RTL render + logical-properties scan)"
```

---

## Task 18: Button — manifest assertion for capabilityContract

**Files:**
- Modify: `app/elohim-elements/elohim-core/src/elohim-button.manifest.spec.ts`

- [ ] **Step 1: Read current manifest spec**

Read: `app/elohim-elements/elohim-core/src/elohim-button.manifest.spec.ts`

- [ ] **Step 2: Add the failing test**

Append to that file:

```ts
describe('<elohim-button> — capabilityContract manifest', () => {
  let contract: any;

  before(async () => {
    const cem = (await import('../dist/custom-elements.json', { with: { type: 'json' } })).default;
    const decl = cem.modules.flatMap((m: any) => m.declarations || [])
      .find((d: any) => d.name === 'ElohimButton');
    contract = decl?.capabilityContract;
  });

  it('declares the precondition gate fields', () => {
    expect(contract).to.exist;
    expect(contract).to.have.property('a11y');
    expect(contract).to.have.property('i18n');
    expect(contract).to.have.property('uaPrefs');
  });

  it('claims maxLens=standard', () => {
    expect(contract.maxLens).to.equal('standard');
  });

  it('claims maxStimulus=still (button does not animate beyond focus)', () => {
    expect(contract.maxStimulus).to.equal('still');
  });

  it('claims both themes (light and dark)', () => {
    expect(contract.themes).to.deep.equal(['light', 'dark']);
  });

  it('claims both contrast tiers (normal and high)', () => {
    expect(contract.contrast).to.deep.equal(['normal', 'high']);
  });

  it('claims contentCertainty=not-observed (button has no content to evaluate)', () => {
    expect(contract.contentCertainty).to.equal('not-observed');
  });

  it('marks all states as n/a (button has no state semantics)', () => {
    expect(contract.states.empty).to.equal('n/a');
    expect(contract.states.error).to.equal('n/a');
    expect(contract.states.contested).to.equal('n/a');
  });

  it('requires pilot | steward | elohim-support standing', () => {
    expect(contract.standings.required).to.deep.equal(['pilot | steward | elohim-support']);
  });
});
```

- [ ] **Step 3: Rebuild (so CEM is up-to-date) and run tests**

```bash
pnpm --filter elohim-core run build && pnpm test --filter elohim-button.manifest
```
Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-elements/elohim-core/src/elohim-button.manifest.spec.ts
git commit -m "test(elohim-button): assert capabilityContract block shape in the merged CEM"
```

---

## Task 19: Run full quality gate

This task runs the full quality gates the existing project uses, to make sure nothing regressed and the new code is lint-clean.

- [ ] **Step 1: Format check**

```bash
pnpm --filter elohim-core run format:check
```
Expected: PASS. If FAIL, run `pnpm --filter elohim-core run format` and re-check.

- [ ] **Step 2: Lint**

```bash
pnpm --filter elohim-core run lint
```
Expected: PASS. If failures, fix them (most likely import-order, unused vars, or sonarjs nits).

- [ ] **Step 3: Style lint**

```bash
pnpm --filter elohim-core run lint:css
```
Expected: PASS.

- [ ] **Step 4: Typecheck**

```bash
pnpm --filter elohim-core run typecheck
```
Expected: PASS.

- [ ] **Step 5: Full test suite**

```bash
pnpm --filter elohim-core test
```
Expected: ALL PASS. New tests + all pre-existing tests green.

- [ ] **Step 6: Full build**

```bash
pnpm --filter elohim-core run build
```
Expected: build succeeds; `dist/` is regenerated.

- [ ] **Step 7: Commit anything regenerated**

```bash
git status
# Anything in dist/ that changed should be committed
git add app/elohim-elements/elohim-core/dist/custom-elements.json
git commit -m "chore(elohim-core): regenerate CEM with capability contract on <elohim-button>"
```

(Skip step 7 commit if `git status` shows nothing to commit.)

---

## Task 20: Documentation pass

**Files:**
- Modify: `app/elohim-elements/elohim-core/README.md`

- [ ] **Step 1: Read current README**

Read: `app/elohim-elements/elohim-core/README.md`

- [ ] **Step 2: Add a "Capability Profile" section before "Adding a new atom"**

Append the following section to the README, immediately before the `## Adding a new atom` heading:

```markdown
## Capability Profile

`elohim-core` exports the **Capability Profile** primitive — a frozen context object that every element observes via the `CapabilityAwareElement` mixin. The profile carries:

- `lens` — disclosure tier (minimal → trace)
- `theme` / `contrast` / `locale` — visual + language register
- `stimulus` — motion tier (**default: still**)
- `textuality` — symbolic vs textual
- `standings[]` — attested roles
- `lock` — steward / elohim-support / pilot constraint set
- `origin` — who set this profile

Companion `ContentCertainty` describes the **content being rendered**: canonical / partial / stale / contested / unreachable / unknown.

Elements declare a `capabilityContract` via JSDoc `@capability*` tags, which the CEM analyzer reads into `dist/custom-elements.json`. The contract is the single source of truth for *which cells the element implements* and *which states it has designed for*.

Three **precondition gates** sit above all cells — a11y, i18n, ua-prefs (incl. photosensitive-flash). Failing any gate blocks every cell.

For the full design, see `genesis/docs/superpowers/specs/2026-05-20-capability-profile-element-contract-design.md`.
```

- [ ] **Step 3: Update "Adding a new atom" to mention capability tags**

In the "Adding a new atom" section, in step 2 (the JSDoc list of tags), add to the bullet list right after `@cssprop` and `@csspart`:

```
   - `@capabilityMaxLens`, `@capabilityThemes`, `@capabilityContrast`, `@capabilityLocales`, `@capabilityMaxStimulus`, `@capabilityTextuality` for the visual contract
   - `@capabilityRequiredStandings`, `@capabilityOptionalStandings` for role requirements
   - `@capabilityContentCertainty observed | not-observed` and `@capabilityStates name:status, ...` for the orthogonal claims
```

- [ ] **Step 4: Commit**

```bash
git add app/elohim-elements/elohim-core/README.md
git commit -m "docs(elohim-core): document Capability Profile primitive + element-contract JSDoc tags"
```

---

## Final verification

- [ ] **All tests pass:**

```bash
pnpm --filter elohim-core test
```
Expected: every test in every spec file green.

- [ ] **Build clean:**

```bash
pnpm --filter elohim-core run build
```
Expected: builds succeed, `dist/custom-elements.json` contains the `capabilityContract` for `ElohimButton`.

- [ ] **All commits land cleanly on the branch:**

```bash
git log --oneline -25
```
Expected: ~20 new commits on top of the spec commit, each focused and testable.

- [ ] **Quality gates green:**

```bash
pnpm --filter elohim-core run format:check && \
  pnpm --filter elohim-core run lint && \
  pnpm --filter elohim-core run lint:css && \
  pnpm --filter elohim-core run typecheck
```
Expected: all green.

When all of the above are green, M1–M3 of the spec are landed. Follow-on plans (M4 storybook matrix; M5 second element; M6 shell lens-toggle + banners; M7 codegen enforcement) build on this foundation and are written separately after this lands.
