import { expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
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
