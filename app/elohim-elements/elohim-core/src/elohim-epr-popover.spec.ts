import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import type { ElohimEprPopover, EprHead } from './elohim-epr-popover.js';
import { ElohimEprPopover as PopoverClass } from './elohim-epr-popover.js';
import { requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';
import {
  assertInlineWithinViewport,
  resetViewport,
  setViewportArchetype,
} from './testing/viewport.js';

const FIXTURE_HEAD: EprHead = {
  version: 1,
  id: 'epr-test-1',
  content: 'content-cid-abc',
  lamad: {
    title: 'Introduction to Shefa',
    contentType: 'lesson',
    contentFormat: 'sophia-quiz-json',
    description: 'A foundational lesson on economic stewardship.',
    tags: ['shefa', 'stewardship', 'foundations'],
  },
  shefa: {
    stewards: ['agent-alice', 'agent-bob'],
  },
  qahal: {
    reach: 'household',
    layer: 'constitutional',
  },
  relationships: [],
};

describe('<elohim-epr-popover>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-epr-popover')).to.equal(PopoverClass);
  });

  it('renders nothing when not visible', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD}></elohim-epr-popover>
    `);
    const popover = el.shadowRoot?.querySelector('[data-testid="epr-popover"]');
    expect(popover).to.not.exist;
  });

  it('renders popover content when visible=true and head is set', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const popover = el.shadowRoot?.querySelector('[data-testid="epr-popover"]');
    expect(popover).to.exist;
  });

  it('shows the content type badge', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const badge = el.shadowRoot?.querySelector('[data-testid="epr-popover-type"]');
    expect(badge?.textContent?.trim()).to.contain('lesson');
  });

  it('shows the content format', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const format = el.shadowRoot?.querySelector('[data-testid="epr-popover-format"]');
    expect(format?.textContent).to.contain('sophia-quiz-json');
  });

  it('shows the title', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const title = el.shadowRoot?.querySelector('[data-testid="epr-popover-title"]');
    expect(title?.textContent?.trim()).to.equal('Introduction to Shefa');
  });

  it('shows the description', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const desc = el.shadowRoot?.querySelector('[data-testid="epr-popover-desc"]');
    expect(desc?.textContent).to.contain('foundational lesson');
  });

  it('renders tags', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const tags = el.shadowRoot?.querySelector('[data-testid="epr-popover-tags"]');
    expect(tags).to.exist;
    expect(tags?.textContent).to.contain('shefa');
    expect(tags?.textContent).to.contain('stewardship');
  });

  it('shows shefa steward count', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const shefa = el.shadowRoot?.querySelector('[data-testid="epr-popover-shefa"]');
    expect(shefa).to.exist;
    expect(shefa?.textContent).to.contain('2');
  });

  it('shows qahal reach badge', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const qahal = el.shadowRoot?.querySelector('[data-testid="epr-popover-qahal"]');
    expect(qahal).to.exist;
    expect(qahal?.textContent).to.contain('household');
    expect(qahal?.textContent).to.contain('constitutional');
  });

  it('hides shefa section when no stewards', async () => {
    const headNoShefa: EprHead = {
      ...FIXTURE_HEAD,
      shefa: {},
    };
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${headNoShefa} .visible=${true}></elohim-epr-popover>
    `);
    const shefa = el.shadowRoot?.querySelector('[data-testid="epr-popover-shefa"]');
    expect(shefa).to.not.exist;
  });

  it('renders "Open resource" button when route is set', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover
        .head=${FIXTURE_HEAD}
        .visible=${true}
        route="/lamad/intro-shefa"
      ></elohim-epr-popover>
    `);
    const link = el.shadowRoot?.querySelector('[data-testid="epr-popover-link"]');
    expect(link).to.exist;
    expect(link?.textContent).to.contain('Open resource');
  });

  it('hides "Open resource" button when route is null', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const link = el.shadowRoot?.querySelector('[data-testid="epr-popover-link"]');
    expect(link).to.not.exist;
  });

  it('renders plain anchor with href when route=null and href is supplied', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover
        .head=${FIXTURE_HEAD}
        .visible=${true}
        href="/epr/foo"
      ></elohim-epr-popover>
    `);
    const link = el.shadowRoot?.querySelector<HTMLAnchorElement>(
      '[data-testid="epr-popover-link"]'
    );
    expect(link).to.exist;
    expect(link?.tagName.toLowerCase()).to.equal('a');
    expect(link?.getAttribute('href')).to.equal('/epr/foo');
  });

  it('dispatches epr-popover-navigate on link click', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover
        .head=${FIXTURE_HEAD}
        .visible=${true}
        route="/lamad/intro-shefa"
      ></elohim-epr-popover>
    `);

    let navigatedRoute: string | null = null;
    el.addEventListener('epr-popover-navigate', (e: Event) => {
      navigatedRoute = (e as CustomEvent<{ route: string }>).detail.route;
    });

    const link = el.shadowRoot?.querySelector<HTMLButtonElement>(
      '[data-testid="epr-popover-link"]'
    );
    link?.click();

    expect(navigatedRoute).to.equal('/lamad/intro-shefa');
  });

  it('dispatches epr-popover-dismiss on mouseleave', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);

    let dismissed = false;
    el.addEventListener('epr-popover-dismiss', () => {
      dismissed = true;
    });

    const popover = el.shadowRoot?.querySelector<HTMLDivElement>('[data-testid="epr-popover"]');
    popover?.dispatchEvent(new MouseEvent('mouseleave', { bubbles: true }));

    expect(dismissed).to.be.true;
  });

  it('dispatches epr-popover-enter on mouseenter', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);

    let entered = false;
    el.addEventListener('epr-popover-enter', () => {
      entered = true;
    });

    const popover = el.shadowRoot?.querySelector<HTMLDivElement>('[data-testid="epr-popover"]');
    popover?.dispatchEvent(new MouseEvent('mouseenter', { bubbles: true }));

    expect(entered).to.be.true;
  });

  it('applies position as inline style', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover
        .head=${FIXTURE_HEAD}
        .visible=${true}
        .position=${{ top: 100, left: 200 }}
      ></elohim-epr-popover>
    `);
    await elementUpdated(el);
    expect(el.style.top).to.equal('100px');
    expect(el.style.left).to.equal('200px');
  });

  it('reflects visible attribute on host', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} visible></elohim-epr-popover>
    `);
    expect(el.hasAttribute('visible')).to.be.true;
    el.visible = false;
    await elementUpdated(el);
    expect(el.hasAttribute('visible')).to.be.false;
  });

  it('has role=tooltip on the popover container', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const popover = el.shadowRoot?.querySelector('[data-testid="epr-popover"]');
    expect(popover?.getAttribute('role')).to.equal('tooltip');
  });

  it('passes axe-core a11y scan when visible', async () => {
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover .head=${FIXTURE_HEAD} .visible=${true}></elohim-epr-popover>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-epr-popover> — i18n gate', () => {
  it('uses only logical CSS properties', () => {
    const cssText = (PopoverClass as { styles: { cssText: string } }).styles.cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });
});

describe('<elohim-epr-popover> — ua-prefs gate', () => {
  it('no animations declared without media guard (still mode default)', () => {
    const cssText = (PopoverClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.not.contain('@keyframes');
  });

  it('forced-colors overrides use CSS system colors', () => {
    const cssText = (PopoverClass as { styles: { cssText: string } }).styles.cssText;
    expect(cssText).to.contain('forced-colors: active');
    const idx = cssText.indexOf('forced-colors: active');
    // eslint-disable-next-line unicorn/prefer-set-has -- string scan, not membership lookup
    const after = cssText.slice(idx);
    const hasSystemColor =
      after.includes('Canvas') ||
      after.includes('CanvasText') ||
      after.includes('ButtonFace') ||
      after.includes('ButtonText') ||
      after.includes('LinkText');
    expect(hasSystemColor).to.be.true;
  });
});

describe('<elohim-epr-popover> — theme-contrast gate', () => {
  // tokens cells: no shipped binding for this element yet — system cells only
  // (theme-authority spec §4.1). The blank-slate contract per scheme.
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];

  for (const cell of CELLS) {
    it(`passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimEprPopover>(
        html`
          <elohim-epr-popover
            .head=${FIXTURE_HEAD}
            .visible=${true}
            route="/lamad/intro-shefa"
          ></elohim-epr-popover>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
      await axeScanStrict(el);
    });
  }
});

// ---------------------------------------------------------------------------
// <elohim-epr-popover> — viewport precondition gate
//
// The popover applies consumer-supplied {top,left} fixed coords. Any naive
// anchor-rect caller (rect.bottom/rect.left — markdown-renderer is the live
// example) reproduces the 2026-06-12 fold-down overflow class near viewport
// edges, so the element owns the final clamp.
// ---------------------------------------------------------------------------

describe('<elohim-epr-popover> — viewport precondition gate', () => {
  afterEach(() => resetViewport());

  it('clamps an off-screen consumer position back inside the phone viewport', async () => {
    await setViewportArchetype('phone');
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover
        .head=${FIXTURE_HEAD}
        .visible=${true}
        .position=${{ top: 100, left: 360 }}
      ></elohim-epr-popover>
    `);
    await elementUpdated(el);
    assertInlineWithinViewport(el);
  });

  it('leaves honest coordinates untouched when the popover already fits', async () => {
    await setViewportArchetype('phone');
    const el = await fixture<ElohimEprPopover>(html`
      <elohim-epr-popover
        .head=${FIXTURE_HEAD}
        .visible=${true}
        .position=${{ top: 40, left: 20 }}
      ></elohim-epr-popover>
    `);
    await elementUpdated(el);
    expect(el.style.top).to.equal('40px');
    expect(el.style.left).to.equal('20px');
  });
});
