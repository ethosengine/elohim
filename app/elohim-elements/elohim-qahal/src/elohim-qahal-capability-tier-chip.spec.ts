import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalCapabilityTierChip as ElohimQahalCapabilityTierChipClass } from './elohim-qahal-capability-tier-chip.js';
import type { ElohimQahalCapabilityTierChip } from './elohim-qahal-capability-tier-chip.js';

describe('<elohim-qahal-capability-tier-chip>', () => {
  it('renders the tier label', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="steward"></elohim-qahal-capability-tier-chip>
    `);
    expect(el.shadowRoot!.textContent).to.include('steward');
  });

  it('supports protected-tier values (child, idd_member, elder_under_guardianship, legal_steward_protected)', async () => {
    for (const t of [
      'child',
      'idd_member',
      'elder_under_guardianship',
      'legal_steward_protected',
    ]) {
      const el = await fixture<ElohimQahalCapabilityTierChip>(html`
        <elohim-qahal-capability-tier-chip tier=${t}></elohim-qahal-capability-tier-chip>
      `);
      expect(el.tier).to.equal(t);
    }
  });

  it('marks protected tiers visually distinctly via attribute', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="child"></elohim-qahal-capability-tier-chip>
    `);
    expect(el.getAttribute('protected')).to.not.be.null;
  });

  it('normalises underscores to spaces in the label', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="idd_member"></elohim-qahal-capability-tier-chip>
    `);
    expect(el.shadowRoot!.textContent).to.include('idd member');
  });

  it('does not set protected attribute for standard tiers', async () => {
    for (const t of ['visitor', 'engaged', 'contributor', 'steward', 'elohim-support']) {
      const el = await fixture<ElohimQahalCapabilityTierChip>(html`
        <elohim-qahal-capability-tier-chip tier=${t}></elohim-qahal-capability-tier-chip>
      `);
      expect(el.getAttribute('protected'), `${t} should not have protected attr`).to.be.null;
    }
  });

  it('renders an unknown tier value without the protected attribute', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="unknown_tier"></elohim-qahal-capability-tier-chip>
    `);
    expect(el.getAttribute('protected')).to.be.null;
    expect(el.shadowRoot!.textContent).to.include('unknown tier');
  });

  it('removes protected attribute when tier changes from protected to standard', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="child"></elohim-qahal-capability-tier-chip>
    `);
    expect(el.getAttribute('protected')).to.not.be.null;
    el.tier = 'steward';
    await elementUpdated(el);
    expect(el.getAttribute('protected')).to.be.null;
  });

  it('retains protected attribute when switching between two protected tiers', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="child"></elohim-qahal-capability-tier-chip>
    `);
    expect(el.getAttribute('protected')).to.not.be.null;
    el.tier = 'idd_member';
    await elementUpdated(el);
    expect(el.getAttribute('protected')).to.not.be.null;
  });

  it('renders with no attributes as visitor tier without protected attr', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip></elohim-qahal-capability-tier-chip>
    `);
    expect(el.tier).to.equal('visitor');
    expect(el.getAttribute('protected')).to.be.null;
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="steward"></elohim-qahal-capability-tier-chip>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-capability-tier-chip> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (chip is still; no motion to gate)', () => {
    // The chip declares maxStimulus=still so it must not have any CSS transitions.
    const cssText = (
      ElohimQahalCapabilityTierChipClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalCapabilityTierChip>(html`
      <elohim-qahal-capability-tier-chip tier="contributor"></elohim-qahal-capability-tier-chip>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-capability-tier-chip> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalCapabilityTierChip>(
      'he-IL',
      html`
        <elohim-qahal-capability-tier-chip tier="engaged"></elohim-qahal-capability-tier-chip>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    // Chip is textual — bounding box has nonzero dimensions
    const chip = el.shadowRoot!.querySelector('.chip')!;
    const rect = chip.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
    expect(rect.height).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalCapabilityTierChipClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
