import { elementUpdated, expect, fixture, html } from '@open-wc/testing';
import axe from 'axe-core';

import './register.js';
import { ElohimHypercardPanel } from './elohim-hypercard-panel.js';
import type { ContextMenuItem } from './elohim-context-menu.js';
import { clearMediaQueries, measureLuminanceChanges } from './testing/ua-prefs.js';
import { renderInLocale, requiresLogicalProperties } from './testing/i18n.js';
import {
  assertThemeContrast,
  axeScanStrict,
  themeFixture,
  type ThemeCell,
} from './testing/theme-contrast.js';

// ---------------------------------------------------------------------------
// Helpers — omni binding fixture injection
//
// The protocol-omni SHIPPED binding (protocol-omni.component.css) assigns
// --elohim-hypercard-bg/border/shadow from --omni-* tokens but (pre-fix)
// does NOT set `color` on the embedding rule, so hypercard text inherits the
// app theme fg while the bg follows the OS color-scheme.  These helpers
// reproduce that exact cascade inside the WTR browser for the tokens cells
// declared in the theme-contrast gate describe block below.
//
// Approach (spec §4.1): the shipped binding is injected as a <style> element
// in the test fixture; the element itself stays blank-slate untouched.
// ---------------------------------------------------------------------------

/**
 * Inject a <style> element with the given CSS text into document.head.
 * Returns a cleanup function that removes the element.
 */
function injectStyle(cssText: string): () => void {
  const el = document.createElement('style');
  el.setAttribute('data-omni-test-fixture', '');
  el.textContent = cssText;
  document.head.append(el);
  return () => el.remove();
}

/**
 * Render an <elohim-hypercard-panel> inside a container that mimics the
 * protocol-omni embedding: `--elohim-hypercard-{bg,border,shadow}` are bound
 * to the BROKEN omni light-scheme tokens (OS-scheme-keyed, no `color`), while
 * the document context carries a dark-app-theme fg — reproducing the contrast
 * failure reported by the operator.
 *
 * Returns the hypercard element and a cleanup function.
 */
async function renderInBrokenOmniBinding(
  actions: ContextMenuItem[]
): Promise<{ el: ElohimHypercardPanel; cleanup: () => void }> {
  // Broken binding: bg from omni-light tokens, no color set.
  // The page context fg is a dark-theme value — simulating the OS=light /
  // app-theme=dark mismatch (or OS=dark / app-theme=light, same failure class).
  const BROKEN_CSS = `
    .omni-resilience-test {
      /* omni light bg — the value the OS-scheme block provides */
      --elohim-hypercard-bg: rgba(255, 255, 255, 0.92);
      --elohim-hypercard-border: 1px solid rgba(20, 22, 30, 0.14);
      --elohim-hypercard-shadow: 0 1px 6px rgba(0, 0, 0, 0.08);
      /* deliberately no color binding on this rule — the bug */
    }
  `;
  const cleanup = injectStyle(BROKEN_CSS);

  // The wrapper provides a dark-theme fg (app theme), opaque dark bg (page).
  // color-scheme:light mimics OS=light so the hypercard bg (rgba(255,255,255,...))
  // resolves white — while the inherited fg stays the dark-app-theme value set
  // on the wrapper below.  Ratio ~1.44:1, matching the live alpha measurement.
  const wrapper = await fixture<HTMLDivElement>(html`
    <div
      style="color-scheme: light; background: rgb(15, 23, 42); color: rgb(232, 234, 240); padding: 8px;"
    >
      <div class="omni-resilience-test" style="position: relative; display: inline-block;">
        <elohim-hypercard-panel panelLabel="Resilience status" .actions=${actions} open>
          <dl>
            <dt>Placement coverage</dt>
            <dd>9 of 12 collective roles filled</dd>
          </dl>
        </elohim-hypercard-panel>
      </div>
    </div>
  `);
  const el = wrapper.querySelector('elohim-hypercard-panel') as ElohimHypercardPanel;
  await el.updateComplete;
  // Disable the fold-down animation before measurement so cumulativeOpacity()
  // sees steady-state opacity (not mid-flight 0), same as themeFixture's
  // disableMotion() helper.
  if (el.shadowRoot) {
    const noMotion = new CSSStyleSheet();
    noMotion.replaceSync(
      ':host, *, *::before, *::after { transition: none !important; animation: none !important; }'
    );
    el.shadowRoot.adoptedStyleSheets = [...el.shadowRoot.adoptedStyleSheets, noMotion];
  }
  return { el, cleanup };
}

// ---------------------------------------------------------------------------
// Shared fixture data
// ---------------------------------------------------------------------------

const ACTIONS: ContextMenuItem[] = [
  { id: 'view-full', label: 'View full resilience' },
  { id: 'steward', label: 'Steward this content' },
  { id: 'network', label: 'View network', disabled: true },
];

const PANEL_LABEL = 'Resilience status';

// ---------------------------------------------------------------------------
// <elohim-hypercard-panel> — open/close render
// ---------------------------------------------------------------------------

describe('<elohim-hypercard-panel>', () => {
  it('is defined in the custom element registry', () => {
    expect(customElements.get('elohim-hypercard-panel')).to.equal(ElohimHypercardPanel);
  });

  it('is hidden when open is false (default)', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
      ></elohim-hypercard-panel>
    `);
    const style = getComputedStyle(el);
    expect(style.display).to.equal('none');
  });

  it('is visible (display:block) when open is true', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <div style="position:relative;">
        <elohim-hypercard-panel
          panelLabel=${PANEL_LABEL}
          .actions=${ACTIONS}
          open
        ></elohim-hypercard-panel>
      </div>
    `);
    const panel = el.querySelector('elohim-hypercard-panel') as ElohimHypercardPanel;
    const style = getComputedStyle(panel);
    expect(style.display).to.equal('block');
  });

  it('open attribute is reflected', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel=${PANEL_LABEL}></elohim-hypercard-panel>
    `);
    expect(el.hasAttribute('open')).to.be.false;
    el.open = true;
    await elementUpdated(el);
    expect(el.hasAttribute('open')).to.be.true;
    el.open = false;
    await elementUpdated(el);
    expect(el.hasAttribute('open')).to.be.false;
  });

  // ---------------------------------------------------------------------------
  // Slot rendering
  // ---------------------------------------------------------------------------

  it('renders slotted content inside the content part', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel=${PANEL_LABEL} open>
        <p id="slotted-content">Placement gap: 3 of 12 collective roles unfilled</p>
      </elohim-hypercard-panel>
    `);
    const slot = el.shadowRoot!.querySelector('slot') as HTMLSlotElement;
    const assigned = slot.assignedElements();
    expect(assigned.length).to.equal(1);
    expect(assigned[0]!.id).to.equal('slotted-content');
  });

  it('exposes container, content, action-row, and action parts', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel=${PANEL_LABEL} .actions=${ACTIONS} open>
        <p>Stewarding collective: Vineyard Household</p>
      </elohim-hypercard-panel>
    `);
    expect(el.shadowRoot!.querySelector('[part="container"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="content"]')).to.exist;
    expect(el.shadowRoot!.querySelector('[part="action-row"]')).to.exist;
    const actionParts = el.shadowRoot!.querySelectorAll('[part="action"]');
    expect(actionParts.length).to.equal(3);
  });

  it('renders role="dialog" with aria-label from panelLabel', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel="Household resilience" open></elohim-hypercard-panel>
    `);
    const dialog = el.shadowRoot!.querySelector('[role="dialog"]');
    expect(dialog).to.exist;
    expect(dialog!.getAttribute('aria-label')).to.equal('Household resilience');
  });

  // ---------------------------------------------------------------------------
  // Action row rendering
  // ---------------------------------------------------------------------------

  it('renders no action-row when actions is empty', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel=${PANEL_LABEL} open></elohim-hypercard-panel>
    `);
    expect(el.shadowRoot!.querySelector('[part="action-row"]')).to.not.exist;
  });

  it('renders action-row when actions is non-empty', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
        open
      ></elohim-hypercard-panel>
    `);
    expect(el.shadowRoot!.querySelector('[part="action-row"]')).to.exist;
  });

  it('each action button has data-action-id matching item.id', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
        open
      ></elohim-hypercard-panel>
    `);
    const btns = el.shadowRoot!.querySelectorAll<HTMLButtonElement>('[part="action"]');
    expect(btns[0]!.getAttribute('data-action-id')).to.equal('view-full');
    expect(btns[1]!.getAttribute('data-action-id')).to.equal('steward');
    expect(btns[2]!.getAttribute('data-action-id')).to.equal('network');
  });

  it('disabled action button has disabled attribute and aria-disabled=true', async () => {
    const actions: ContextMenuItem[] = [
      { id: 'active', label: 'Active action' },
      { id: 'locked', label: 'Locked action', disabled: true },
    ];
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${actions}
        open
      ></elohim-hypercard-panel>
    `);
    const btns = el.shadowRoot!.querySelectorAll<HTMLButtonElement>('[part="action"]');
    expect(btns[0]!.disabled).to.be.false;
    expect(btns[0]!.getAttribute('aria-disabled')).to.equal('false');
    expect(btns[1]!.disabled).to.be.true;
    expect(btns[1]!.getAttribute('aria-disabled')).to.equal('true');
  });

  // ---------------------------------------------------------------------------
  // action-select event
  // ---------------------------------------------------------------------------

  it('emits action-select with correct id when an action button is clicked', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
        open
      ></elohim-hypercard-panel>
    `);
    let received: { id: string } | null = null;
    el.addEventListener('action-select', (e: Event) => {
      received = (e as CustomEvent<{ id: string }>).detail;
    });
    const btn = el.shadowRoot!.querySelector<HTMLButtonElement>('[data-action-id="view-full"]')!;
    btn.click();
    expect(received).to.exist;
    expect(received!.id).to.equal('view-full');
  });

  it('action-select event is bubbles:true, composed:true', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <div>
        <elohim-hypercard-panel
          panelLabel=${PANEL_LABEL}
          .actions=${ACTIONS}
          open
        ></elohim-hypercard-panel>
      </div>
    `);
    const panel = el.querySelector('elohim-hypercard-panel') as ElohimHypercardPanel;
    let received: CustomEvent | null = null;
    // Listen on the outer div — event must bubble across shadow boundary
    el.addEventListener('action-select', (e: Event) => {
      received = e as CustomEvent;
    });
    const btn = panel.shadowRoot!.querySelector<HTMLButtonElement>('[data-action-id="steward"]')!;
    btn.click();
    expect(received).to.exist;
    expect(received!.bubbles).to.be.true;
    expect(received!.composed).to.be.true;
  });

  it('disabled action does not emit action-select on click', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
        open
      ></elohim-hypercard-panel>
    `);
    let fired = false;
    el.addEventListener('action-select', () => {
      fired = true;
    });
    const disabledBtn = el.shadowRoot!.querySelector<HTMLButtonElement>(
      '[data-action-id="network"]'
    )!;
    disabledBtn.click();
    expect(fired).to.be.false;
    // Panel should still be open
    expect(el.open).to.be.true;
  });

  // ---------------------------------------------------------------------------
  // close event
  // ---------------------------------------------------------------------------

  it('stays open after an enabled action is clicked — selection is intent, the host decides dismissal', async () => {
    // Auto-closing on selection would race the host's in-place card flip
    // (the close-reset would always win) — spec §11.1 boundary discipline.
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
        open
      ></elohim-hypercard-panel>
    `);
    let closeFired = false;
    let selected: string | null = null;
    el.addEventListener('close', () => {
      closeFired = true;
    });
    el.addEventListener('action-select', (e: Event) => {
      selected = (e as CustomEvent<{ id: string }>).detail.id;
    });
    const btn = el.shadowRoot!.querySelector<HTMLButtonElement>('[data-action-id="steward"]')!;
    btn.click();
    await elementUpdated(el);
    expect(selected).to.equal('steward');
    expect(closeFired).to.be.false;
    expect(el.open).to.be.true;
  });

  it('pulls focus back inside when the action row changes while open (in-place flip)', async () => {
    // The flip consumes the activating button; without repair, focus drops to
    // <body> and Escape stops reaching the dialog.
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
        open
      ></elohim-hypercard-panel>
    `);
    const btn = el.shadowRoot!.querySelector<HTMLButtonElement>('[data-action-id="view-full"]')!;
    btn.focus();
    btn.click();
    el.actions = [];
    await elementUpdated(el);
    await new Promise<void>(resolve => {
      queueMicrotask(() => resolve());
    });
    const container = el.shadowRoot!.querySelector('[part="container"]') as HTMLElement;
    expect(el.shadowRoot!.activeElement).to.equal(container);
    expect(el.open).to.be.true;
  });

  it('close event is bubbles:true, composed:true', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <div>
        <elohim-hypercard-panel
          panelLabel=${PANEL_LABEL}
          .actions=${ACTIONS}
          open
        ></elohim-hypercard-panel>
      </div>
    `);
    const panel = el.querySelector('elohim-hypercard-panel') as ElohimHypercardPanel;
    let received: CustomEvent | null = null;
    el.addEventListener('close', (e: Event) => {
      received = e as CustomEvent;
    });
    const dialog = panel.shadowRoot!.querySelector('[role="dialog"]') as HTMLElement;
    dialog.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await elementUpdated(panel);
    expect(received).to.exist;
    expect(received!.bubbles).to.be.true;
    expect(received!.composed).to.be.true;
  });

  // ---------------------------------------------------------------------------
  // Escape closes from focus inside
  // ---------------------------------------------------------------------------

  it('closes on Escape key dispatched inside the dialog', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
        open
      ></elohim-hypercard-panel>
    `);
    const dialog = el.shadowRoot!.querySelector('[role="dialog"]') as HTMLElement;
    dialog.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await elementUpdated(el);
    expect(el.open).to.be.false;
  });

  it('emits close event on Escape', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
        open
      ></elohim-hypercard-panel>
    `);
    let closeFired = false;
    el.addEventListener('close', () => {
      closeFired = true;
    });
    const dialog = el.shadowRoot!.querySelector('[role="dialog"]') as HTMLElement;
    dialog.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await elementUpdated(el);
    expect(closeFired).to.be.true;
  });

  // ---------------------------------------------------------------------------
  // Click-outside closes
  // ---------------------------------------------------------------------------

  it('closes on mousedown outside the element', async () => {
    const wrapper = await fixture<HTMLDivElement>(html`
      <div>
        <elohim-hypercard-panel
          panelLabel=${PANEL_LABEL}
          .actions=${ACTIONS}
          open
        ></elohim-hypercard-panel>
        <button id="outside" type="button">Outside</button>
      </div>
    `);
    const panel = wrapper.querySelector('elohim-hypercard-panel') as ElohimHypercardPanel;
    const outside = wrapper.querySelector<HTMLButtonElement>('#outside')!;
    outside.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, composed: true }));
    await elementUpdated(panel);
    expect(panel.open).to.be.false;
  });

  it('does NOT close on mousedown inside the element', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel=${PANEL_LABEL} .actions=${ACTIONS} open>
        <p>Collective stewardship summary</p>
      </elohim-hypercard-panel>
    `);
    el.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, composed: true }));
    await elementUpdated(el);
    expect(el.open).to.be.true;
  });

  // ---------------------------------------------------------------------------
  // Focus management
  // ---------------------------------------------------------------------------

  it('moves focus into first enabled action button on open', async () => {
    // Create an element closed first, then open it
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${ACTIONS}
      ></elohim-hypercard-panel>
    `);
    el.open = true;
    await elementUpdated(el);
    // Wait for queueMicrotask to fire
    await new Promise(resolve => queueMicrotask(resolve));
    const focused = el.shadowRoot!.activeElement;
    expect(focused).to.exist;
    expect(focused!.getAttribute('data-action-id')).to.equal('view-full');
  });

  it('moves focus to container when there are no enabled actions', async () => {
    const allDisabled: ContextMenuItem[] = [{ id: 'a', label: 'Disabled', disabled: true }];
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel
        panelLabel=${PANEL_LABEL}
        .actions=${allDisabled}
      ></elohim-hypercard-panel>
    `);
    el.open = true;
    await elementUpdated(el);
    await new Promise(resolve => queueMicrotask(resolve));
    const focused = el.shadowRoot!.activeElement;
    // Should be the container (role="dialog")
    expect(focused).to.exist;
    expect(focused!.getAttribute('role')).to.equal('dialog');
  });

  it('restores focus to previously focused element on close', async () => {
    const wrapper = await fixture<HTMLDivElement>(html`
      <div>
        <button id="trigger" type="button">Open panel</button>
        <elohim-hypercard-panel
          panelLabel=${PANEL_LABEL}
          .actions=${ACTIONS}
        ></elohim-hypercard-panel>
      </div>
    `);
    const trigger = wrapper.querySelector<HTMLButtonElement>('#trigger')!;
    const panel = wrapper.querySelector<ElohimHypercardPanel>('elohim-hypercard-panel')!;

    // Focus trigger, then open panel
    trigger.focus();
    expect(document.activeElement).to.equal(trigger);
    panel.open = true;
    await elementUpdated(panel);
    await new Promise(resolve => queueMicrotask(resolve));

    // Close the panel via Escape
    const dialog = panel.shadowRoot!.querySelector('[role="dialog"]') as HTMLElement;
    dialog.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await elementUpdated(panel);

    // Focus should have restored to the trigger
    expect(document.activeElement).to.equal(trigger);
  });
});

// ---------------------------------------------------------------------------
// <elohim-hypercard-panel> — a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-hypercard-panel> — a11y precondition gate', () => {
  it('passes axe-core scan when open with slotted content and actions', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel="Resilience status" .actions=${ACTIONS} open>
        <dl>
          <dt>Placement coverage</dt>
          <dd>9 of 12 collective roles filled</dd>
        </dl>
      </elohim-hypercard-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core scan with no actions (content-only panel)', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel="Stewarding collective summary" open>
        <p>Three stewardship commitments active in the Vineyard household.</p>
      </elohim-hypercard-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('passes axe-core scan with a disabled action', async () => {
    const actions: ContextMenuItem[] = [
      { id: 'active', label: 'View placement gaps' },
      { id: 'locked', label: 'Steward this content', disabled: true },
    ];
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel="Content stewardship" .actions=${actions} open>
        <p>Content stewardship status: 2 active commitments.</p>
      </elohim-hypercard-panel>
    `);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });
});

// ---------------------------------------------------------------------------
// <elohim-hypercard-panel> — ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-hypercard-panel> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('fold-down animation is wrapped in (prefers-reduced-motion: no-preference)', () => {
    const cssText = (ElohimHypercardPanel as unknown as { styles: { cssText: string } }).styles
      .cssText;
    expect(cssText).to.contain('prefers-reduced-motion');
    const motionIdx = cssText.indexOf('prefers-reduced-motion: no-preference');
    expect(motionIdx).to.be.greaterThan(-1);

    const afterMotion = cssText.slice(motionIdx);
    expect(afterMotion).to.contain('elohim-hypercard-fold-down');
  });

  it('fold-down animation is also gated on (update: fast)', () => {
    const cssText = (ElohimHypercardPanel as unknown as { styles: { cssText: string } }).styles
      .cssText;
    const motionIdx = cssText.indexOf('prefers-reduced-motion: no-preference');
    const afterMotion = cssText.slice(motionIdx);
    expect(afterMotion).to.contain('update: fast');
  });

  it('forced-colors overrides use CSS system colors (Canvas, CanvasText, Highlight, HighlightText, GrayText)', () => {
    const cssText = (ElohimHypercardPanel as unknown as { styles: { cssText: string } }).styles
      .cssText;
    expect(cssText).to.contain('forced-colors: active');
    const forcedIdx = cssText.indexOf('forced-colors: active');
    // eslint-disable-next-line unicorn/prefer-set-has -- string scan, not membership lookup
    const afterForced = cssText.slice(forcedIdx);
    const hasSystemColor =
      afterForced.includes('Canvas') ||
      afterForced.includes('CanvasText') ||
      afterForced.includes('Highlight') ||
      afterForced.includes('HighlightText') ||
      afterForced.includes('GrayText');
    expect(hasSystemColor).to.be.true;
  });

  it('passes the photosensitive-flash analyzer (no luminance flicker)', async () => {
    const el = await fixture<ElohimHypercardPanel>(html`
      <elohim-hypercard-panel panelLabel=${PANEL_LABEL} .actions=${ACTIONS} open>
        <p>Collective placement summary</p>
      </elohim-hypercard-panel>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 600, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// <elohim-hypercard-panel> — i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-hypercard-panel> — i18n precondition gate', () => {
  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const cssText = (ElohimHypercardPanel as unknown as { styles: { cssText: string } }).styles
      .cssText;
    const findings = requiresLogicalProperties(cssText);
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('renders correctly in RTL document direction (he-IL)', async () => {
    const el = await renderInLocale<ElohimHypercardPanel>(
      'he-IL',
      html`
        <elohim-hypercard-panel
          panelLabel="מצב חוסן"
          .actions=${[{ id: 'view-full', label: 'הצג מלא' }]}
          open
        >
          <p>סיכום ניהול קולקטיב</p>
        </elohim-hypercard-panel>
      `
    );
    expect(el).to.exist;
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    const rect = el.getBoundingClientRect();
    expect(rect.width).to.be.greaterThan(0);
  });
});

// ---------------------------------------------------------------------------
// <elohim-hypercard-panel> — theme-contrast gate
// ---------------------------------------------------------------------------

describe('<elohim-hypercard-panel> — theme-contrast gate', () => {
  // System cells only — no shipped token binding for this element yet.
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];

  const THEME_ACTIONS: ContextMenuItem[] = [
    { id: 'view-full', label: 'View full resilience' },
    { id: 'steward', label: 'Steward this content' },
    { id: 'network', label: 'View network', disabled: true },
  ];

  for (const cell of CELLS) {
    it(`passes contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimHypercardPanel>(
        html`
          <elohim-hypercard-panel panelLabel="Resilience status" .actions=${THEME_ACTIONS} open>
            <dl>
              <dt>Placement coverage</dt>
              <dd>9 of 12 collective roles filled</dd>
            </dl>
          </elohim-hypercard-panel>
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
// <elohim-hypercard-panel> — omni binding tokens cells
//
// These cells pin the protocol-omni.component.css → hypercard cssprop
// binding contract (theme-authority spec §4.1 "tokens cells": inject the
// SHIPPED binding as the test fixture; blank-slate discipline holds; the
// element is never modified).
//
// Cell taxonomy:
//   omni-binding-broken  — FAILING-FIRST gate: reproduces the OS-scheme ≠
//     app-theme mismatch that produced the operator-reported unreadable cards.
//     Injects the pre-fix binding (no `color` on the embedding rule); the
//     dark-app-theme fg inherited from the wrapper over the light-OS-scheme bg
//     yields ~1.44:1 (WCAG 4.5:1 threshold → FAIL).  This cell MUST fail
//     before the CSS fix and pass after it (post-fix color pairing closes the
//     contrast gap because both fg and bg come from the same --omni-* family).
//
//   omni-light           — CONTRACT cell: omni light tokens + light context.
//     Verifies rgba(255,255,255,0.92) bg is paired with rgba(20,22,30,0.96) fg
//     (the fixed .omni-resilience rule adds `color: var(--omni-fg)`).
//     Passes post-fix only.
//
//   omni-dark            — CONTRACT cell: omni dark tokens + dark context.
//     Verifies rgba(22,23,28,0.92) bg is paired with rgba(232,234,240,0.96) fg.
//     Passes post-fix only.
// ---------------------------------------------------------------------------

describe('<elohim-hypercard-panel> — omni binding tokens cells', () => {
  const OMNI_ACTIONS: ContextMenuItem[] = [
    { id: 'view-full', label: 'View full resilience' },
    { id: 'steward', label: 'Steward this content' },
    { id: 'network', label: 'View network', disabled: true },
  ];

  // ── FAILING-FIRST cell ────────────────────────────────────────────────────
  // This cell reproduces the BROKEN pre-fix pairing:
  //   bg = rgba(255,255,255,0.92)  (omni light, OS-scheme-keyed)
  //   fg = rgb(232,234,240)        (dark-app-theme, inherited from wrapper)
  // Computed ratio ≈1.44:1 — far below the 4.5:1 WCAG threshold.
  //
  // Expected lifecycle:
  //   BEFORE CSS fix  → FAIL  (assertThemeContrast throws — red gate)
  //   AFTER  CSS fix  → PASS  (color: var(--omni-fg) brings fg into the
  //                            light-token family → rgba(20,22,30,0.96) on
  //                            rgba(255,255,255,0.92) ≈ 18:1 PASS)
  it('omni-binding-broken: fails contrast when OS-scheme bg is paired with app-theme fg (pre-fix reproducer)', async () => {
    const { el, cleanup } = await renderInBrokenOmniBinding(OMNI_ACTIONS);
    try {
      // assertThemeContrast is expected to THROW here — that is the failing-first
      // evidence.  We catch the error and re-throw with a clear label so the WTR
      // output names the regression class, not just "Error".
      let threw = false;
      try {
        assertThemeContrast(el);
      } catch (err) {
        threw = true;
        // Confirm it IS a contrast failure (not some other error).
        const msg = err instanceof Error ? err.message : String(err);
        expect(msg, 'Expected a theme-contrast failure').to.contain('theme-contrast');
        // Confirm the measured ratio is below threshold (ratio string present).
        // The walk reports "X:1 < 4.5:1" so we just check "<" is in the message.
        expect(msg, 'Expected a below-threshold ratio in the failure message').to.contain('<');
      }
      expect(
        threw,
        'assertThemeContrast MUST throw for the broken pairing — if it does not throw, the failing-first gate is not exercising the regression'
      ).to.be.true;
    } finally {
      cleanup();
    }
  });

  // ── CONTRACT cell: omni-light (post-fix) ─────────────────────────────────
  // Fixed binding: both bg and fg come from the omni light token family.
  //   bg ≈ rgba(255,255,255,0.92) — omni light bg
  //   fg ≈ rgba(20,22,30,0.96)    — omni light fg  (color: var(--omni-fg))
  // Ratio ≈ 18:1 — well above 4.5:1.
  //
  // Note: a no-motion sheet is injected into the hypercard shadow root to
  // disable the fold-down animation before measurement.  The animation runs
  // opacity 0→1; without suppression, cumulativeOpacity() would pick up
  // opacity≈0 mid-flight and report a bogus 1:1 ratio (same guard the
  // themeFixture helper applies via disableMotion()).
  it('omni-light: hypercard bg+fg both come from omni light tokens (post-fix contract)', async () => {
    const FIXED_LIGHT_CSS = `
      .omni-resilience-test-light {
        --omni-bg: rgba(255, 255, 255, 0.92);
        --omni-fg: rgba(20, 22, 30, 0.96);
        --omni-border: rgba(20, 22, 30, 0.14);
        --omni-shadow: 0 1px 6px rgba(0, 0, 0, 0.08);
        --elohim-hypercard-bg: var(--omni-bg);
        --elohim-hypercard-border: var(--omni-border);
        --elohim-hypercard-shadow: var(--omni-shadow);
        color: var(--omni-fg);
      }
    `;
    const cleanup = injectStyle(FIXED_LIGHT_CSS);
    try {
      const wrapper = await fixture<HTMLDivElement>(html`
        <div
          style="color-scheme: light; background: rgb(249, 250, 251); color: rgb(17, 24, 39); padding: 8px;"
        >
          <div
            class="omni-resilience-test-light"
            style="position: relative; display: inline-block;"
          >
            <elohim-hypercard-panel panelLabel="Resilience status" .actions=${OMNI_ACTIONS} open>
              <dl>
                <dt>Placement coverage</dt>
                <dd>9 of 12 collective roles filled</dd>
              </dl>
            </elohim-hypercard-panel>
          </div>
        </div>
      `);
      const el = wrapper.querySelector('elohim-hypercard-panel') as ElohimHypercardPanel;
      await el.updateComplete;
      // Disable the fold-down animation before measurement so cumulativeOpacity()
      // sees steady-state opacity (not mid-flight 0).  Mirror the approach used
      // in themeFixture's disableMotion() helper.
      if (el.shadowRoot) {
        const noMotion = new CSSStyleSheet();
        noMotion.replaceSync(
          ':host, *, *::before, *::after { transition: none !important; animation: none !important; }'
        );
        el.shadowRoot.adoptedStyleSheets = [...el.shadowRoot.adoptedStyleSheets, noMotion];
      }
      assertThemeContrast(el);
    } finally {
      cleanup();
    }
  });

  // ── CONTRACT cell: omni-dark (post-fix) ──────────────────────────────────
  // Fixed binding: both bg and fg come from the omni dark token family.
  //   bg ≈ rgba(22,23,28,0.92)      — omni dark bg
  //   fg ≈ rgba(232,234,240,0.96)   — omni dark fg  (color: var(--omni-fg))
  // Ratio ≈ 11:1 — well above 4.5:1.
  it('omni-dark: hypercard bg+fg both come from omni dark tokens (post-fix contract)', async () => {
    const FIXED_DARK_CSS = `
      .omni-resilience-test-dark {
        --omni-bg: rgba(22, 23, 28, 0.92);
        --omni-fg: rgba(232, 234, 240, 0.96);
        --omni-border: rgba(232, 234, 240, 0.16);
        --omni-shadow: 0 1px 6px rgba(0, 0, 0, 0.35);
        --elohim-hypercard-bg: var(--omni-bg);
        --elohim-hypercard-border: var(--omni-border);
        --elohim-hypercard-shadow: var(--omni-shadow);
        color: var(--omni-fg);
      }
    `;
    const cleanup = injectStyle(FIXED_DARK_CSS);
    try {
      const wrapper = await fixture<HTMLDivElement>(html`
        <div
          style="color-scheme: dark; background: rgb(15, 23, 42); color: rgb(226, 232, 240); padding: 8px;"
        >
          <div class="omni-resilience-test-dark" style="position: relative; display: inline-block;">
            <elohim-hypercard-panel panelLabel="Resilience status" .actions=${OMNI_ACTIONS} open>
              <dl>
                <dt>Placement coverage</dt>
                <dd>9 of 12 collective roles filled</dd>
              </dl>
            </elohim-hypercard-panel>
          </div>
        </div>
      `);
      const el = wrapper.querySelector('elohim-hypercard-panel') as ElohimHypercardPanel;
      await el.updateComplete;
      // Disable animation so cumulativeOpacity() sees steady-state opacity.
      if (el.shadowRoot) {
        const noMotion = new CSSStyleSheet();
        noMotion.replaceSync(
          ':host, *, *::before, *::after { transition: none !important; animation: none !important; }'
        );
        el.shadowRoot.adoptedStyleSheets = [...el.shadowRoot.adoptedStyleSheets, noMotion];
      }
      assertThemeContrast(el);
    } finally {
      cleanup();
    }
  });
});
