import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalImagodeiBadge as ElohimQahalImagodeiBadgeClass } from './elohim-qahal-imagodei-badge.js';
import type { ElohimQahalImagodeiBadge } from './elohim-qahal-imagodei-badge.js';

describe('<elohim-qahal-imagodei-badge>', () => {
  it('renders with display name in default slot', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="Matthew Dowell"></elohim-qahal-imagodei-badge>
    `);
    expect(el.shadowRoot).to.exist;
    expect(el.shadowRoot!.textContent).to.include('Matthew Dowell');
  });

  it('exposes name and avatar-url properties', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge
        name="Matthew Dowell"
        avatar-url="https://example.com/m.jpg"
      ></elohim-qahal-imagodei-badge>
    `);
    expect(el.name).to.equal('Matthew Dowell');
    expect(el.avatarUrl).to.equal('https://example.com/m.jpg');
  });

  it('supports a standing-tier attribute (visitor | engaged | contributor | steward)', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="X" standing-tier="steward"></elohim-qahal-imagodei-badge>
    `);
    expect(el.standingTier).to.equal('steward');
    expect(el.getAttribute('standing-tier')).to.equal('steward');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="Matthew"></elohim-qahal-imagodei-badge>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('renders fallback initials when avatar-url is absent', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="Matthew Dowell"></elohim-qahal-imagodei-badge>
    `);
    expect(el.shadowRoot!.textContent).to.include('MD');
  });
});

describe('<elohim-qahal-imagodei-badge> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (badge is still; no motion to gate)', () => {
    // The badge declares maxStimulus=still so it must not have any CSS transitions.
    // Structural assertion: the styles string must not contain 'transition:' at all.
    const cssText = (
      ElohimQahalImagodeiBadgeClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalImagodeiBadge>(html`
      <elohim-qahal-imagodei-badge name="Test"></elohim-qahal-imagodei-badge>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-imagodei-badge> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalImagodeiBadge>(
      'he-IL',
      html`
        <elohim-qahal-imagodei-badge name="מתיו"></elohim-qahal-imagodei-badge>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    // Confirm the avatar and name are laid out — bounding box has nonzero dimensions
    const avatar = el.shadowRoot!.querySelector('.avatar')!;
    const rect = avatar.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalImagodeiBadgeClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
