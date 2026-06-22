import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import {
  ElohimContributorCard as ElohimContributorCardClass,
  presenceStateBadge,
} from './elohim-imagodei-contributor-card.js';
import type { ElohimContributorCard } from './elohim-imagodei-contributor-card.js';
import type { ContributorPresenceView } from '@elohim/storage-client';

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

const UNCLAIMED: ContributorPresenceView = {
  id: 'sha256-0000000000000000000000000000000000000000000000000000000000000001',
  hAppId: 'sha256-app0000000000000000000000000000000000000000000000000000000000001',
  displayName: 'Ada Lovelace',
  presenceState: 'unclaimed',
  externalIdentifiers: null,
  establishingContentIds: [],
  affinityTotal: 14.5,
  uniqueEngagers: 3,
  citationCount: 7,
  recognitionScore: 42.0,
  recognitionByContent: null,
  lastRecognitionAt: null,
  stewardId: null,
  stewardshipStartedAt: null,
  stewardshipCommitmentId: null,
  stewardshipQualityScore: null,
  claimInitiatedAt: null,
  claimVerifiedAt: null,
  claimVerificationMethod: null,
  claimEvidence: null,
  claimedAgentId: null,
  claimRecognitionTransferredValue: null,
  claimFacilitatedBy: null,
  image: null,
  note: null,
  metadata: null,
  createdAt: '2026-01-01T00:00:00Z',
  updatedAt: '2026-01-01T00:00:00Z',
  dhtAnchorHash: null,
};

const CLAIMED: ContributorPresenceView = {
  ...UNCLAIMED,
  id: 'sha256-0000000000000000000000000000000000000000000000000000000000000002',
  displayName: 'Grace Hopper',
  presenceState: 'claimed',
  image: 'https://example.com/grace.jpg',
  recognitionScore: 128.0,
  citationCount: 22,
  affinityTotal: 63.0,
  uniqueEngagers: 11,
  claimedAgentId: 'uhCAkSomeAgentKey',
};

// ---------------------------------------------------------------------------
// Unit tests — presenceStateBadge pure function
// ---------------------------------------------------------------------------

describe('presenceStateBadge()', () => {
  it('returns "Open to steward" variant for unclaimed — framed as an opportunity', () => {
    const { label, variant } = presenceStateBadge('unclaimed');
    expect(variant).to.equal('unclaimed');
    expect(label).to.equal('Open to steward');
    // Must NOT read like an error
    expect(label.toLowerCase()).not.to.include('error');
    expect(label.toLowerCase()).not.to.include('unrecognised');
  });

  it('returns "Stewarded" variant for stewarded', () => {
    const { label, variant } = presenceStateBadge('stewarded');
    expect(variant).to.equal('stewarded');
    expect(label).to.equal('Stewarded');
  });

  it('returns "Claimed" variant for claimed', () => {
    const { label, variant } = presenceStateBadge('claimed');
    expect(variant).to.equal('claimed');
    expect(label).to.equal('Claimed');
  });

  it('returns "Claiming…" variant for claiming', () => {
    const { label, variant } = presenceStateBadge('claiming');
    expect(variant).to.equal('claiming');
    expect(label).to.include('Claiming');
  });

  it('degrades gracefully on an unknown state string', () => {
    const { label, variant } = presenceStateBadge('future-state-xyz');
    expect(variant).to.equal('unknown');
    // Sentence-case from raw string — does not crash
    expect(label).to.equal('Future-state-xyz');
  });

  it('handles empty string without throwing', () => {
    const { label, variant } = presenceStateBadge('');
    expect(variant).to.equal('unknown');
    expect(label).to.equal('Unknown');
  });
});

// ---------------------------------------------------------------------------
// Core element contract tests
// ---------------------------------------------------------------------------

describe('<elohim-contributor-card>', () => {
  it('renders an empty state message when no presence is supplied', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card></elohim-contributor-card>
    `);
    expect(el.shadowRoot!.textContent).to.include('No contributor data');
  });

  it('renders display name from the presence', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Ada Lovelace');
  });

  it('renders the state badge with the correct label', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    const badge = el.shadowRoot!.querySelector('[part="state-badge"]');
    expect(badge).to.exist;
    expect(badge!.textContent?.trim()).to.include('Open to steward');
  });

  it('uses data-variant on the badge — not color-only signalling', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    const badge = el.shadowRoot!.querySelector('[part="state-badge"]');
    expect(badge!.getAttribute('data-variant')).to.equal('unclaimed');
    // Marker prefix must also be present (CSS ::before uses data-badge-marker attr)
    expect(badge!.getAttribute('data-badge-marker')).to.not.be.empty;
  });

  it('renders recognition score and citation count', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('42');
    expect(text).to.include('7');
  });

  it('hides affinity/engager stats when compact=true', async () => {
    const full = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    expect(full.shadowRoot!.querySelector('[part="affinity-row"]')).to.exist;

    const compact = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED} compact></elohim-contributor-card>
    `);
    expect(compact.shadowRoot!.querySelector('[part="affinity-row"]')).to.be.null;
  });

  it('renders an <img> when presence.image is set', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${CLAIMED}></elohim-contributor-card>
    `);
    const img = el.shadowRoot!.querySelector('[part="avatar"] img');
    expect(img).to.exist;
    expect(img!.getAttribute('src')).to.equal('https://example.com/grace.jpg');
  });

  it('renders initials placeholder when presence.image is null', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    expect(el.shadowRoot!.querySelector('[part="avatar"] img')).to.be.null;
    const initials = el.shadowRoot!.querySelector('.avatar-initials');
    expect(initials).to.exist;
    // Ada Lovelace -> AL
    expect(initials!.textContent?.trim()).to.equal('AL');
  });

  it('emits contributor-select with { id } on click (no href — button path)', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    let detail: { id: string } | null = null;
    el.addEventListener('contributor-select', e => {
      detail = (e as CustomEvent<{ id: string }>).detail;
    });
    (el.shadowRoot!.querySelector('[part="activation"]') as HTMLElement).click();
    expect(detail).to.deep.equal({ id: UNCLAIMED.id });
  });

  it('emits contributor-select exactly ONCE per click when href is set (no double-fire)', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card
        .presence=${CLAIMED}
        href="/contributors/grace"
      ></elohim-contributor-card>
    `);
    let count = 0;
    el.addEventListener('contributor-select', () => {
      count += 1;
    });
    const anchor = el.shadowRoot!.querySelector('[part="activation"]') as HTMLAnchorElement;
    // Prevent navigation so the test page does not redirect
    anchor.addEventListener('click', e => e.preventDefault(), { once: true });
    anchor.click();
    expect(count).to.equal(1);
  });

  it('emits contributor-select on keyboard Enter (button fires natively)', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    let fired = false;
    el.addEventListener('contributor-select', () => {
      fired = true;
    });
    // <button> fires 'click' natively on Enter — simulate that
    const btn = el.shadowRoot!.querySelector('[part="activation"]') as HTMLButtonElement;
    btn.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(fired).to.be.true;
  });

  it('emits contributor-select on keyboard Space (button fires natively)', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    let fired = false;
    el.addEventListener('contributor-select', () => {
      fired = true;
    });
    // <button> fires 'click' natively on Space — simulate that
    const btn = el.shadowRoot!.querySelector('[part="activation"]') as HTMLButtonElement;
    btn.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(fired).to.be.true;
  });

  it('renders an <a> element when href is supplied', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card
        .presence=${CLAIMED}
        href="/contributors/grace"
      ></elohim-contributor-card>
    `);
    const anchor = el.shadowRoot!.querySelector('[part="activation"]');
    expect(anchor?.tagName.toLowerCase()).to.equal('a');
    expect(anchor?.getAttribute('href')).to.equal('/contributors/grace');
  });

  it('renders a <button> element when no href is supplied', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    const btn = el.shadowRoot!.querySelector('[part="activation"]');
    expect(btn?.tagName.toLowerCase()).to.equal('button');
  });

  it('passes axe accessibility audit for unclaimed state', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit for claimed state', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${CLAIMED}></elohim-contributor-card>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe accessibility audit when href is set', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card
        .presence=${CLAIMED}
        href="/contributors/grace"
      ></elohim-contributor-card>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-contributor-card> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS has no transition/animation rules (stimulus: still)', () => {
    const cssText = (ElohimContributorCardClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    expect(cssText).not.to.contain('transition:');
    expect(cssText).not.to.contain('animation:');
    expect(cssText).not.to.contain('@keyframes');
  });

  it('CSS has a forced-colors active override block', () => {
    const cssText = (ElohimContributorCardClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    expect(cssText).to.contain('forced-colors');
  });

  it('CSS has a pointer:coarse override block for touch targets', () => {
    const cssText = (ElohimContributorCardClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    expect(cssText).to.contain('pointer: coarse');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimContributorCard>(html`
      <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-contributor-card> — i18n precondition gate', () => {
  it('uses only logical CSS properties (no physical margin-left/right etc.)', () => {
    const cssText = (ElohimContributorCardClass as unknown as { styles: { cssText: string } })
      .styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimContributorCard>(
      'he-IL',
      html`
        <elohim-contributor-card .presence=${UNCLAIMED}></elohim-contributor-card>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const article = el.shadowRoot!.querySelector('article')!;
    const rect = article.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });
});
