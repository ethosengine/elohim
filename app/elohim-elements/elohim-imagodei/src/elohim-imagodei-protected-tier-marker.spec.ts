import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiProtectedTierMarker as ElohimImagodeiProtectedTierMarkerClass } from './elohim-imagodei-protected-tier-marker.js';
import type {
  ElohimImagodeiProtectedTierMarker,
  ProtectedTier,
} from './elohim-imagodei-protected-tier-marker.js';

describe('<elohim-imagodei-protected-tier-marker>', () => {
  it('renders correctly for all 4 protected tier values', async () => {
    const tiers: ProtectedTier[] = [
      'child',
      'idd_member',
      'elder_under_guardianship',
      'legal_steward_protected',
    ];
    for (const t of tiers) {
      const el = await fixture<ElohimImagodeiProtectedTierMarker>(html`
        <elohim-imagodei-protected-tier-marker tier=${t}></elohim-imagodei-protected-tier-marker>
      `);
      expect(el.tier).to.equal(t);
      expect(el.shadowRoot!.textContent).to.include(t.replace(/_/g, ' '));
    }
  });

  it('normalises underscores to spaces in the label', async () => {
    const el = await fixture<ElohimImagodeiProtectedTierMarker>(html`
      <elohim-imagodei-protected-tier-marker
        tier="elder_under_guardianship"
      ></elohim-imagodei-protected-tier-marker>
    `);
    expect(el.shadowRoot!.textContent).to.include('elder under guardianship');
  });

  it('renders the child tier label', async () => {
    const el = await fixture<ElohimImagodeiProtectedTierMarker>(html`
      <elohim-imagodei-protected-tier-marker tier="child"></elohim-imagodei-protected-tier-marker>
    `);
    expect(el.shadowRoot!.textContent).to.include('child');
  });

  it('renders the idd_member tier with underscore-to-space label', async () => {
    const el = await fixture<ElohimImagodeiProtectedTierMarker>(html`
      <elohim-imagodei-protected-tier-marker
        tier="idd_member"
      ></elohim-imagodei-protected-tier-marker>
    `);
    expect(el.shadowRoot!.textContent).to.include('idd member');
  });

  it('renders gracefully for an unknown tier value', async () => {
    const el = await fixture<ElohimImagodeiProtectedTierMarker>(html`
      <elohim-imagodei-protected-tier-marker
        tier=${'unknown_tier' as ProtectedTier}
      ></elohim-imagodei-protected-tier-marker>
    `);
    // Should render without crashing; label normalises underscores
    expect(el.shadowRoot!.textContent).to.include('unknown tier');
    // Unknown tiers should not get the shield icon
    expect(el.shadowRoot!.querySelector('.shield')).to.be.null;
  });

  it('carries a role=status on the marker span', async () => {
    const el = await fixture<ElohimImagodeiProtectedTierMarker>(html`
      <elohim-imagodei-protected-tier-marker tier="child"></elohim-imagodei-protected-tier-marker>
    `);
    const marker = el.shadowRoot!.querySelector('.marker')!;
    expect(marker.getAttribute('role')).to.equal('status');
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimImagodeiProtectedTierMarker>(html`
      <elohim-imagodei-protected-tier-marker
        tier="legal_steward_protected"
      ></elohim-imagodei-protected-tier-marker>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-imagodei-protected-tier-marker> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (marker is still; no motion to gate)', () => {
    const cssText = (
      ElohimImagodeiProtectedTierMarkerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimImagodeiProtectedTierMarker>(html`
      <elohim-imagodei-protected-tier-marker tier="child"></elohim-imagodei-protected-tier-marker>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-imagodei-protected-tier-marker> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimImagodeiProtectedTierMarker>(
      'he-IL',
      html`
        <elohim-imagodei-protected-tier-marker
          tier="idd_member"
        ></elohim-imagodei-protected-tier-marker>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const marker = el.shadowRoot!.querySelector('.marker')!;
    const rect = marker.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimImagodeiProtectedTierMarkerClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
