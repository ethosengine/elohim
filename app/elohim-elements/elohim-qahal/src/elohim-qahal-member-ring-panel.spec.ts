import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';
import {
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
} from 'elohim-core/testing';

import './register.js';
import { ElohimQahalMemberRingPanel as ElohimQahalMemberRingPanelClass } from './elohim-qahal-member-ring-panel.js';
import type { ElohimQahalMemberRingPanel, MemberTier } from './elohim-qahal-member-ring-panel.js';

const makeMember = (id: string) => ({ id, name: `Member ${id}` });

const SAMPLE_TIERS: MemberTier[] = [
  {
    id: 'stewards',
    label: 'Stewards',
    count: 3,
    members: [makeMember('alice'), makeMember('bob'), makeMember('carol')],
  },
  {
    id: 'contributors',
    label: 'Contributors',
    count: 5,
    members: [
      makeMember('dave'),
      makeMember('eve'),
      makeMember('frank'),
      makeMember('grace'),
      makeMember('hank'),
    ],
  },
  {
    id: 'contributor-presences',
    label: 'Contributor Presences',
    count: 2,
    members: [makeMember('ivan'), makeMember('judy')],
  },
  {
    id: 'compute-hosting-stewards',
    label: 'Compute-Hosting Stewards',
    count: 1,
    members: [makeMember('kevin')],
  },
];

describe('<elohim-qahal-member-ring-panel>', () => {
  it('renders "Network reach" label', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="150"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Network reach');
  });

  it('renders reach count prominently', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="150"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const count = el.shadowRoot!.querySelector('.reach-count');
    expect(count).to.exist;
    expect(count!.textContent!.trim()).to.equal('150');
  });

  it('renders a section for each tier', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="11"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const sections = el.shadowRoot!.querySelectorAll('.tier-section');
    expect(sections).to.have.lengthOf(4);
  });

  it('renders tier label and count for each tier', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="11"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include('Stewards');
    expect(text).to.include('Contributors');
  });

  it('renders imagodei badges for each visible member', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="11"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const badges = el.shadowRoot!.querySelectorAll('elohim-qahal-imagodei-badge');
    // 3 + 5 + 2 + 1 = 11 total members, all under MAX_BADGES=8 per tier
    expect(badges.length).to.equal(11);
  });

  it('shows "+ N more" when tier has more than 8 members', async () => {
    const bigTier: MemberTier = {
      id: 'big',
      label: 'Big Tier',
      count: 12,
      members: Array.from({ length: 12 }, (_, i) => ({ id: `m${i}`, name: `Member ${i}` })),
    };
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="12"
        .tiers=${[bigTier]}
      ></elohim-qahal-member-ring-panel>
    `);
    const overflow = el.shadowRoot!.querySelector('.tier-overflow');
    expect(overflow).to.exist;
    expect(overflow!.textContent).to.include('+ 4 more');
    // Only 8 badges shown
    const badges = el.shadowRoot!.querySelectorAll('elohim-qahal-imagodei-badge');
    expect(badges.length).to.equal(8);
  });

  it('renders contributor-presences tier note', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="11"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include(
      'non-protocol participants whose recognition accrues to the Qahal commons'
    );
  });

  it('renders compute-hosting-stewards tier note', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="11"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const text = el.shadowRoot!.textContent ?? '';
    expect(text).to.include(
      'lending stewarded compute allocation for resilience, edge distribution, discovery'
    );
  });

  it('passes axe accessibility audit', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="11"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-qahal-member-ring-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions (panel is still; no motion to gate)', () => {
    const cssText = (
      ElohimQahalMemberRingPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    expect(cssText).to.not.contain('transition:');
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimQahalMemberRingPanel>(html`
      <elohim-qahal-member-ring-panel
        reach="11"
        .tiers=${SAMPLE_TIERS}
      ></elohim-qahal-member-ring-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

describe('<elohim-qahal-member-ring-panel> — i18n precondition gate', () => {
  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimQahalMemberRingPanel>(
      'he-IL',
      html`
        <elohim-qahal-member-ring-panel
          reach="11"
          .tiers=${SAMPLE_TIERS}
        ></elohim-qahal-member-ring-panel>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const div = el.shadowRoot!.querySelector('div')!;
    const rect = div.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (
      ElohimQahalMemberRingPanelClass as unknown as {
        styles: { cssText: string };
      }
    ).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});
