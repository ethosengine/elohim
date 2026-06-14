import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import {
  ElohimQahalMemorySafety as ElohimQahalMemorySafetyClass,
  type FeltStatusView,
} from './elohim-qahal-memory-safety.js';
import type { ElohimQahalMemorySafety } from './elohim-qahal-memory-safety.js';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const PROTECTED: FeltStatusView = {
  headline: 'Held by 3 households: the Dowells, Aunt Ruth, First Church',
  reassurance: 'protected',
  heldBy: [
    { id: 'sha256-aabbcc', kind: 'household', label: 'the Dowells' },
    { id: 'sha256-ddeeff', kind: 'household', label: 'Aunt Ruth' },
    { id: 'sha256-112233', kind: 'church', label: 'First Church' },
  ],
  floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 3 },
};

const WATCHING: FeltStatusView = {
  headline: 'These photos are safe — 2 households still hold a complete copy',
  reassurance: 'watching',
  heldBy: [
    { id: 'sha256-aabbcc', kind: 'household', label: 'the Dowells' },
    { id: 'sha256-112233', kind: 'church', label: 'First Church' },
  ],
  floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 2 },
};

const NEEDS_HELP: FeltStatusView = {
  headline: 'Only 1 household is holding these photos',
  reassurance: 'needs-help',
  heldBy: [{ id: 'sha256-aabbcc', kind: 'household', label: 'the Dowells' }],
  floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 1 },
  suggestedAction: 'Invite a household to help hold these',
};

const NOT_YET_SEEN: FeltStatusView = {
  headline: "We can't confirm these are backed up yet",
  reassurance: 'not-yet-seen',
  heldBy: [],
  floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 0 },
  suggestedAction: 'Invite a household to help hold these',
};

const VAULT_FLOOR: FeltStatusView = {
  headline: 'Held by 3 households: the Dowells, Aunt Ruth, First Church',
  reassurance: 'watching',
  heldBy: [
    { id: 'sha256-aabbcc', kind: 'household', label: 'the Dowells' },
    { id: 'sha256-ddeeff', kind: 'household', label: 'Aunt Ruth' },
    { id: 'sha256-112233', kind: 'church', label: 'First Church' },
  ],
  floor: { tier: 'vault', tierDeclared: true, wantsHouseholds: 5, hasHouseholds: 3 },
};

// ---------------------------------------------------------------------------
// Core rendering contract
// ---------------------------------------------------------------------------

describe('<elohim-qahal-memory-safety>', () => {
  it('renders the empty state when feltStatus is null', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety></elohim-qahal-memory-safety>
    `);
    const empty = el.shadowRoot!.querySelector('.empty');
    expect(empty).to.exist;
    expect(empty!.textContent!.trim()).to.include('loading');
  });

  it('renders headline from feltStatus.headline — does NOT re-translate', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety
        .feltStatus=${PROTECTED}
      ></elohim-qahal-memory-safety>
    `);
    const headline = el.shadowRoot!.querySelector('.headline');
    expect(headline).to.exist;
    expect(headline!.textContent!.trim()).to.equal(
      'Held by 3 households: the Dowells, Aunt Ruth, First Church'
    );
  });

  it('renders content-label in the heading when provided', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety
        content-label="summer-1974"
        .feltStatus=${PROTECTED}
      ></elohim-qahal-memory-safety>
    `);
    const heading = el.shadowRoot!.querySelector('.heading');
    expect(heading!.textContent).to.include('summer-1974');
  });

  it('renders holder labels (names), not id or hash values', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    const chips = el.shadowRoot!.querySelectorAll('.holder-chip');
    expect(chips).to.have.lengthOf(3);
    const texts = Array.from(chips).map(c => c.textContent!.trim());
    expect(texts).to.include('the Dowells');
    expect(texts).to.include('Aunt Ruth');
    expect(texts).to.include('First Church');
    // Must NOT render the id/hash
    for (const text of texts) {
      expect(text).to.not.match(/sha256/);
    }
  });

  it('renders "a household" when label is absent on a heldBy entry', async () => {
    const statusWithUnlabelled: FeltStatusView = {
      ...WATCHING,
      heldBy: [
        { id: 'sha256-aabbcc', kind: 'household' }, // no label
        { id: 'sha256-ddeeff', kind: 'household', label: 'Aunt Ruth' },
      ],
    };
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${statusWithUnlabelled}></elohim-qahal-memory-safety>
    `);
    const chips = el.shadowRoot!.querySelectorAll('.holder-chip');
    const texts = Array.from(chips).map(c => c.textContent!.trim());
    expect(texts).to.include('a household');
    expect(texts).to.include('Aunt Ruth');
  });

  it('renders an empty holder list (no chips) when heldBy is empty', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NOT_YET_SEEN}></elohim-qahal-memory-safety>
    `);
    const list = el.shadowRoot!.querySelector('.holder-list');
    expect(list).to.not.exist;
  });

  it('renders the floor line with M of N phrasing', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${WATCHING}></elohim-qahal-memory-safety>
    `);
    const floor = el.shadowRoot!.querySelector('.floor-line');
    expect(floor).to.exist;
    expect(floor!.textContent).to.match(/2 of the 3/);
  });

  it('renders the vault-tier floor with tier name when tierDeclared=true', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${VAULT_FLOOR}></elohim-qahal-memory-safety>
    `);
    const floor = el.shadowRoot!.querySelector('.floor-line');
    expect(floor!.textContent).to.include('vault tier');
    expect(floor!.textContent).to.match(/3 of the 5/);
  });

  it('does NOT mention the tier when tierDeclared=false (default floor)', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    const floor = el.shadowRoot!.querySelector('.floor-line');
    // tierDeclared=false → tier name NOT present (not imposing household's choice)
    expect(floor!.textContent).to.not.include('standard tier');
  });
});

// ---------------------------------------------------------------------------
// Four reassurance states
// ---------------------------------------------------------------------------

describe('<elohim-qahal-memory-safety> — reassurance states', () => {
  it('"protected" state: badge label is "Protected"', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    const badge = el.shadowRoot!.querySelector('.status-badge');
    expect(badge).to.exist;
    expect(badge!.getAttribute('data-reassurance')).to.equal('protected');
    expect(badge!.textContent).to.include('Protected');
  });

  it('"watching" state: badge label is "Watching"', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${WATCHING}></elohim-qahal-memory-safety>
    `);
    const badge = el.shadowRoot!.querySelector('.status-badge');
    expect(badge!.getAttribute('data-reassurance')).to.equal('watching');
    expect(badge!.textContent).to.include('Watching');
  });

  it('"needs-help" state: badge label is "Needs help"', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NEEDS_HELP}></elohim-qahal-memory-safety>
    `);
    const badge = el.shadowRoot!.querySelector('.status-badge');
    expect(badge!.getAttribute('data-reassurance')).to.equal('needs-help');
    expect(badge!.textContent).to.include('Needs help');
  });

  it('"not-yet-seen" state: badge label is "Not yet confirmed"', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NOT_YET_SEEN}></elohim-qahal-memory-safety>
    `);
    const badge = el.shadowRoot!.querySelector('.status-badge');
    expect(badge!.getAttribute('data-reassurance')).to.equal('not-yet-seen');
    expect(badge!.textContent).to.include('Not yet confirmed');
  });

  it('"not-yet-seen" has a DIFFERENT badge indicator from "protected"', async () => {
    const protectedEl = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    const notSeenEl = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NOT_YET_SEEN}></elohim-qahal-memory-safety>
    `);
    const protectedIndicator = protectedEl
      .shadowRoot!.querySelector('.status-badge [aria-hidden]')!
      .textContent!.trim();
    const notSeenIndicator = notSeenEl
      .shadowRoot!.querySelector('.status-badge [aria-hidden]')!
      .textContent!.trim();
    // Honesty discipline: must be visually distinct from protected
    expect(notSeenIndicator).to.not.equal(protectedIndicator);
  });

  it('"not-yet-seen" badge uses a dashed border (CSS pattern check)', () => {
    // Structural CSS assertion — not-yet-seen must have border-style: dashed
    // to ensure it is visually distinct from protected independent of color.
    const cssText = (
      ElohimQahalMemorySafetyClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.include("data-reassurance='not-yet-seen'");
    expect(cssText).to.include('border-style: dashed');
  });
});

// ---------------------------------------------------------------------------
// CTA / suggestedAction
// ---------------------------------------------------------------------------

describe('<elohim-qahal-memory-safety> — CTA', () => {
  it('renders the suggestedAction as a button when present', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NEEDS_HELP}></elohim-qahal-memory-safety>
    `);
    const cta = el.shadowRoot!.querySelector('.cta');
    expect(cta).to.exist;
    expect(cta!.tagName).to.equal('BUTTON');
    expect(cta!.textContent!.trim()).to.equal('Invite a household to help hold these');
  });

  it('does NOT render a CTA button when suggestedAction is absent', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    expect(el.shadowRoot!.querySelector('.cta')).to.not.exist;
  });

  it('dispatches memory-safety-action event when CTA is clicked', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NEEDS_HELP}></elohim-qahal-memory-safety>
    `);
    let fired: CustomEvent | null = null;
    el.addEventListener('memory-safety-action', e => {
      fired = e as CustomEvent;
    });
    const btn = el.shadowRoot!.querySelector('.cta') as HTMLButtonElement;
    btn.click();
    expect(fired).to.exist;
    expect((fired as unknown as CustomEvent<{ action: string }>).detail.action).to.equal(
      'Invite a household to help hold these'
    );
  });

  it('dispatches memory-safety-action event when CTA receives Enter keydown', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NOT_YET_SEEN}></elohim-qahal-memory-safety>
    `);
    let fired = false;
    el.addEventListener('memory-safety-action', () => {
      fired = true;
    });
    const btn = el.shadowRoot!.querySelector('.cta') as HTMLButtonElement;
    btn.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true }));
    expect(fired).to.be.true;
  });
});

// ---------------------------------------------------------------------------
// Reactivity
// ---------------------------------------------------------------------------

describe('<elohim-qahal-memory-safety> — reactivity', () => {
  it('updates reassurance badge when feltStatus changes', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    expect(
      el.shadowRoot!.querySelector('.status-badge')!.getAttribute('data-reassurance')
    ).to.equal('protected');

    el.feltStatus = NOT_YET_SEEN;
    await elementUpdated(el);

    expect(
      el.shadowRoot!.querySelector('.status-badge')!.getAttribute('data-reassurance')
    ).to.equal('not-yet-seen');
  });

  it('reveals CTA when reassurance transitions to needs-help', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    expect(el.shadowRoot!.querySelector('.cta')).to.not.exist;

    el.feltStatus = NEEDS_HELP;
    await elementUpdated(el);

    expect(el.shadowRoot!.querySelector('.cta')).to.exist;
  });
});

// ---------------------------------------------------------------------------
// a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-qahal-memory-safety> — a11y gate', () => {
  it('passes axe audit in protected state', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe audit in not-yet-seen state', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NOT_YET_SEEN}></elohim-qahal-memory-safety>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe audit in empty/null state', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety></elohim-qahal-memory-safety>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('status-badge has role=status and aria-live=polite for screen readers', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${WATCHING}></elohim-qahal-memory-safety>
    `);
    const badge = el.shadowRoot!.querySelector('.status-badge')!;
    expect(badge.getAttribute('role')).to.equal('status');
    expect(badge.getAttribute('aria-live')).to.equal('polite');
  });

  it('status-badge aria-label describes the state in words (not color-only)', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NEEDS_HELP}></elohim-qahal-memory-safety>
    `);
    const badge = el.shadowRoot!.querySelector('.status-badge')!;
    expect(badge.getAttribute('aria-label')).to.include('Needs help');
  });

  it('CTA button is keyboard-focusable and has a semantic type=button', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${NEEDS_HELP}></elohim-qahal-memory-safety>
    `);
    const btn = el.shadowRoot!.querySelector('.cta') as HTMLButtonElement;
    expect(btn.type).to.equal('button');
    // Must not be disabled
    expect(btn.disabled).to.be.false;
  });

  it('holder-list has aria-label="Held by"', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    const list = el.shadowRoot!.querySelector('.holder-list')!;
    expect(list.getAttribute('aria-label')).to.equal('Held by');
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-qahal-memory-safety> — ua-prefs gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (maxStimulus=still)', () => {
    const cssText = (
      ElohimQahalMemorySafetyClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
    expect(cssText).to.not.contain('animation:');
    expect(cssText).to.not.contain('@keyframes');
  });

  it('CSS contains forced-colors overrides using system colors', () => {
    const cssText = (
      ElohimQahalMemorySafetyClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.include('forced-colors: active');
    expect(cssText).to.include('Canvas');
    expect(cssText).to.include('CanvasText');
    expect(cssText).to.include('ButtonFace');
    expect(cssText).to.include('ButtonText');
  });

  it('passes photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalMemorySafety>(html`
      <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });

  it('CSS contains pointer:coarse enlargement for holder chips and CTA', () => {
    const cssText = (
      ElohimQahalMemorySafetyClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.include('pointer: coarse');
    // The coarse section must enlarge min-block-size
    expect(cssText).to.include('min-block-size');
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-qahal-memory-safety> — i18n gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalMemorySafety>(
      'he-IL',
      html`
        <elohim-qahal-memory-safety .feltStatus=${PROTECTED}></elohim-qahal-memory-safety>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const container = el.shadowRoot!.querySelector('.container')!;
    const rect = container.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalMemorySafetyClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
