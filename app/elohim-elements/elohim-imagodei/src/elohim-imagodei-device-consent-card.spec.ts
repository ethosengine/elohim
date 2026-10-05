import { aTimeout, expect, fixture, html, oneEvent } from '@open-wc/testing';
import axe from 'axe-core';
import {
  assertThemeContrast,
  clearMediaQueries,
  measureLuminanceChanges,
  renderInLocale,
  requiresLogicalProperties,
  themeFixture,
  type ThemeCell,
} from 'elohim-core/testing';

import './register.js';
import { ElohimImagodeiDeviceConsentCard as DeviceConsentCardClass } from './elohim-imagodei-device-consent-card.js';
import type {
  DeviceConsentApproveDetail,
  DeviceConsentRequest,
  ElohimImagodeiDeviceConsentCard,
} from './elohim-imagodei-device-consent-card.js';

const ENROLL_ONLY: DeviceConsentRequest = {
  clientId: 'elohim-cli',
  label: 'workspace',
  deviceFingerprint: 'uhCAk…8f3a',
  askedActs: ['device.enroll'],
};

const ENROLL_AND_ROOT: DeviceConsentRequest = {
  ...ENROLL_ONLY,
  deviceRootFingerprint: 'uhCAk…c41e',
  askedActs: ['device.enroll', 'device.bind-root'],
};

const cssText = (): string =>
  (DeviceConsentCardClass as unknown as { styles: { cssText: string } }).styles.cssText;

const q = <T extends Element = HTMLElement>(el: Element, sel: string): T | null =>
  el.shadowRoot!.querySelector<T>(sel);

/** Drop the Unicode isolate marks the card wraps around person-supplied names. */
const plain = (value: string): string => value.replace(/[⁦-⁩]/g, '');

const text = (el: Element): string => plain(el.shadowRoot!.textContent!).replace(/\s+/g, ' ');

const checkbox = (el: Element, act: string) =>
  q<HTMLInputElement>(el, `[part="act-row"][data-act="${act}"] input`)!;

async function toggle(el: ElohimImagodeiDeviceConsentCard, act: string, on: boolean) {
  const box = checkbox(el, act);
  box.checked = on;
  box.dispatchEvent(new Event('change'));
  await el.updateComplete;
}

async function review(request: DeviceConsentRequest) {
  return fixture<ElohimImagodeiDeviceConsentCard>(html`
    <elohim-imagodei-device-consent-card .request=${request}></elohim-imagodei-device-consent-card>
  `);
}

// ---------------------------------------------------------------------------
// Review phase — act rows, coherence, approve detail
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-device-consent-card> — review', () => {
  it('names the device and shows its short key with a compare line', async () => {
    const el = await review(ENROLL_ONLY);
    expect(q(el, '[part="heading"]')!.textContent).to.include('workspace');
    expect(q(el, '[part="fingerprint"][data-key="device"]')!.textContent).to.include('uhCAk…8f3a');
    expect(q(el, '[part="device-key"]')!.textContent).to.include('what your terminal printed');
  });

  it('renders one act row per asked act, with the story-asserted names', async () => {
    const one = await review(ENROLL_ONLY);
    const oneRows = one.shadowRoot!.querySelectorAll('[part="act-row"]');
    expect(oneRows.length).to.equal(1);
    expect(oneRows[0].textContent).to.include('enroll this device');

    const two = await review(ENROLL_AND_ROOT);
    const rows = Array.from(two.shadowRoot!.querySelectorAll('[part="act-row"]'));
    expect(rows.map(r => r.getAttribute('data-act'))).to.deep.equal([
      'device.enroll',
      'device.bind-root',
    ]);
    expect(rows[0].textContent).to.include('enroll this device');
    expect(rows[1].textContent).to.include('bind this device’s root key');
  });

  it('gives each act its plain sentence, and shows the root key on its row', async () => {
    const el = await review(ENROLL_AND_ROOT);
    const enroll = q(el, '[part="act-row"][data-act="device.enroll"]')!;
    const root = q(el, '[part="act-row"][data-act="device.bind-root"]')!;
    expect(enroll.textContent).to.include('recognize this machine as one of yours');
    expect(root.textContent).to.include('traced back to you');
    expect(root.querySelector('[part="fingerprint"][data-key="root"]')!.textContent).to.include(
      'uhCAk…c41e'
    );
  });

  it('ignores acts it does not recognise', async () => {
    const el = await review({
      ...ENROLL_ONLY,
      askedActs: ['device.enroll', 'content.publish' as 'device.enroll'],
    });
    expect(el.shadowRoot!.querySelectorAll('[part="act-row"]').length).to.equal(1);
  });

  it('starts with every asked act agreed', async () => {
    const el = await review(ENROLL_AND_ROOT);
    expect(checkbox(el, 'device.enroll').checked).to.equal(true);
    expect(checkbox(el, 'device.bind-root').checked).to.equal(true);
  });

  it('states that enrollment does not let the device change content', async () => {
    const el = await review(ENROLL_ONLY);
    expect(q(el, '[part="scope-note"]')!.textContent).to.include(
      'does not, by itself, let the device change any of your content'
    );
  });

  it('says the doorway signs for a hosted person, naming the host when given', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        signer="doorway-host"
        host-label="alpha"
      ></elohim-imagodei-device-consent-card>
    `);
    const line = plain(q(el, '[part="signer"]')!.textContent!);
    expect(line).to.include('Your doorway (alpha)');
    expect(line).to.include('sign this as you');
  });

  it('says this node signs for a person on their own node', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        signer="peer-conductor"
      ></elohim-imagodei-device-consent-card>
    `);
    expect(q(el, '[part="signer"]')!.textContent).to.include('This node holds your key');
  });

  it('shows who is signed in when personLabel is set', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        person-label="Matthew"
      ></elohim-imagodei-device-consent-card>
    `);
    expect(q(el, '[part="signed-in"]')!.textContent).to.include('Matthew');
  });

  it('coherence: leaving out enrollment leaves out and disables the root key, with a reason', async () => {
    const el = await review(ENROLL_AND_ROOT);
    await toggle(el, 'device.enroll', false);
    const root = checkbox(el, 'device.bind-root');
    expect(root.checked).to.equal(false);
    expect(root.disabled).to.equal(true);
    const reason = q(el, '[part="act-row"][data-act="device.bind-root"] [data-blocked]');
    expect(reason).to.exist;
    expect(root.getAttribute('aria-describedby')).to.include(reason!.id);
  });

  it('coherence: agreeing to enrollment again restores the root-key choice', async () => {
    const el = await review(ENROLL_AND_ROOT);
    await toggle(el, 'device.enroll', false);
    await toggle(el, 'device.enroll', true);
    expect(checkbox(el, 'device.bind-root').checked).to.equal(true);
    expect(checkbox(el, 'device.bind-root').disabled).to.equal(false);
  });

  it('coherence: a root key asked without enrollment can never be agreed', async () => {
    const el = await review({ ...ENROLL_AND_ROOT, askedActs: ['device.bind-root'] });
    expect(checkbox(el, 'device.bind-root').disabled).to.equal(true);
    expect(q<HTMLButtonElement>(el, '[part="approve"]')!.disabled).to.equal(true);
  });

  it('approve carries every act agreed by default', async () => {
    const el = await review(ENROLL_AND_ROOT);
    setTimeout(() => q(el, '[part="approve"]')!.click());
    const ev = (await oneEvent(el, 'approve')) as CustomEvent<DeviceConsentApproveDetail>;
    expect(ev.detail).to.deep.equal({
      agreedActs: ['device.enroll', 'device.bind-root'],
      declinedActs: [],
    });
    expect(ev.bubbles).to.equal(true);
    expect(ev.composed).to.equal(true);
  });

  it('approve carries the root key as declined when it was left out', async () => {
    const el = await review(ENROLL_AND_ROOT);
    await toggle(el, 'device.bind-root', false);
    setTimeout(() => q(el, '[part="approve"]')!.click());
    const ev = (await oneEvent(el, 'approve')) as CustomEvent<DeviceConsentApproveDetail>;
    expect(ev.detail).to.deep.equal({
      agreedActs: ['device.enroll'],
      declinedActs: ['device.bind-root'],
    });
  });

  it('disables approve, with a reason, when nothing is agreed', async () => {
    const el = await review(ENROLL_AND_ROOT);
    await toggle(el, 'device.enroll', false);
    const approve = q<HTMLButtonElement>(el, '[part="approve"]')!;
    expect(approve.disabled).to.equal(true);
    const hint = q(el, '#nothing-chosen')!;
    expect(approve.getAttribute('aria-describedby')).to.equal(hint.id);
    let fired = false;
    el.addEventListener('approve', () => {
      fired = true;
    });
    approve.click();
    expect(fired).to.equal(false);
  });

  it('decline fires a bubbling, composed decline event', async () => {
    const el = await review(ENROLL_ONLY);
    setTimeout(() => q(el, '[part="decline"]')!.click());
    const ev = (await oneEvent(el, 'decline')) as CustomEvent<{ reason: string }>;
    expect(ev.detail.reason).to.equal('user-rejected');
    expect(ev.bubbles).to.equal(true);
    expect(ev.composed).to.equal(true);
  });

  it('uses no protocol jargon in the visible review copy', async () => {
    const el = await review(ENROLL_AND_ROOT);
    const visible = text(el).toLowerCase();
    for (const word of ['dht', 'cap grant', 'pkce', 'consent record', 'relying party']) {
      expect(visible).to.not.include(word);
    }
  });

  it('accepts replacement strings from the host', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        .strings=${{ approve: 'Aprobar' }}
      ></elohim-imagodei-device-consent-card>
    `);
    expect(q(el, '[part="approve"]')!.textContent).to.include('Aprobar');
    expect(q(el, '[part="decline"]')!.textContent).to.include('Decline');
  });
});

// ---------------------------------------------------------------------------
// Phases
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-device-consent-card> — phases', () => {
  it('signing: actions and act controls disabled, busy announced', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_AND_ROOT}
        phase="signing"
      ></elohim-imagodei-device-consent-card>
    `);
    expect(q<HTMLButtonElement>(el, '[part="approve"]')!.disabled).to.equal(true);
    expect(q<HTMLButtonElement>(el, '[part="decline"]')!.disabled).to.equal(true);
    expect(q<HTMLFieldSetElement>(el, 'fieldset')!.disabled).to.equal(true);
    expect(q(el, '[part="card"]')!.getAttribute('aria-busy')).to.equal('true');
    const status = q(el, '[part="status"]')!;
    expect(status.getAttribute('role')).to.equal('status');
    expect(status.textContent).to.include('signing this as you');
    // The "will sign" promise is replaced by the status, not repeated beside it.
    expect(q(el, '[part="signer"]')).to.equal(null);
  });

  it('signing: approve and decline clicks are ignored', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="signing"
      ></elohim-imagodei-device-consent-card>
    `);
    let fired = 0;
    el.addEventListener('approve', () => fired++);
    el.addEventListener('decline', () => fired++);
    // Disabled buttons swallow clicks; call the handlers' DOM path anyway.
    q(el, '[part="approve"]')!.click();
    q(el, '[part="decline"]')!.click();
    expect(fired).to.equal(0);
  });

  it('code: shows the code large and selectable, a copy button, a countdown and the paste line', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="code"
        code="KQ7M-4XTD"
        .expiresAt=${Date.now() + 5 * 60_000}
      ></elohim-imagodei-device-consent-card>
    `);
    const code = q(el, '[part="code"]')!;
    expect(code.textContent).to.include('KQ7M-4XTD');
    expect(getComputedStyle(code).userSelect).to.equal('all');
    expect(getComputedStyle(code).fontFamily).to.include('monospace');
    expect(q(el, '[part="copy"]')).to.exist;
    const countdown = q(el, '[part="countdown"]')!;
    expect(countdown.getAttribute('role')).to.equal('timer');
    expect(countdown.textContent).to.match(/[45]:\d\d/);
    expect(text(el)).to.include('Paste this into the terminal that asked');
    expect(text(el)).to.include('Only that terminal can use it');
  });

  it('code: the live region speaks in whole minutes, not every second', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="code"
        code="KQ7M-4XTD"
        .expiresAt=${Date.now() + 5 * 60_000}
      ></elohim-imagodei-device-consent-card>
    `);
    await el.updateComplete;
    const live = q(el, '[aria-live="polite"]')!;
    expect(live.textContent).to.include('About 5 minutes left');
    const timer = q(el, '[part="countdown"]')!;
    expect(timer.hasAttribute('aria-live')).to.equal(false);
  });

  it('code: copy uses the clipboard when available and fires code-copied', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="code"
        code="KQ7M-4XTD"
      ></elohim-imagodei-device-consent-card>
    `);
    const original = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
    let written = '';
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: {
        writeText: (t: string) => {
          written = t;
          return Promise.resolve();
        },
      },
    });
    try {
      setTimeout(() => q(el, '[part="copy"]')!.click());
      const ev = (await oneEvent(el, 'code-copied')) as CustomEvent<{ method: string }>;
      expect(ev.detail.method).to.equal('clipboard');
      expect(written).to.equal('KQ7M-4XTD');
    } finally {
      if (original) Object.defineProperty(navigator, 'clipboard', original);
      else delete (navigator as unknown as Record<string, unknown>).clipboard;
    }
  });

  it('code: without a clipboard it selects the code instead', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="code"
        code="KQ7M-4XTD"
      ></elohim-imagodei-device-consent-card>
    `);
    const original = Object.getOwnPropertyDescriptor(navigator, 'clipboard');
    Object.defineProperty(navigator, 'clipboard', { configurable: true, value: undefined });
    let fired = false;
    el.addEventListener('code-copied', () => {
      fired = true;
    });
    try {
      q(el, '[part="copy"]')!.click();
      await aTimeout(0);
      await el.updateComplete;
      expect(fired).to.equal(false);
      expect(getSelection()!.toString()).to.include('KQ7M-4XTD');
      expect(text(el)).to.include('The code is selected');
    } finally {
      if (original) Object.defineProperty(navigator, 'clipboard', original);
      else delete (navigator as unknown as Record<string, unknown>).clipboard;
    }
  });

  it('code: fires expired once and shows the expired presentation when time runs out', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="code"
        code="KQ7M-4XTD"
        .expiresAt=${Date.now() + 150}
      ></elohim-imagodei-device-consent-card>
    `);
    let count = 0;
    el.addEventListener('expired', () => count++);
    const ev = (await oneEvent(el, 'expired')) as CustomEvent<{ expiresAt: number }>;
    expect(ev.detail.expiresAt).to.equal(el.expiresAt);
    await el.updateComplete;
    expect(q(el, '[part="code"]')).to.equal(null);
    expect(q(el, '[part="copy"]')).to.equal(null);
    expect(q(el, '[part="heading"]')!.textContent).to.include('run out');
    await aTimeout(1100);
    expect(count).to.equal(1);
  });

  it('code: an already-past expiry renders expired immediately', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="code"
        code="KQ7M-4XTD"
        .expiresAt=${Date.now() - 1000}
      ></elohim-imagodei-device-consent-card>
    `);
    await el.updateComplete;
    expect(q(el, '[part="code"]')).to.equal(null);
    expect(q(el, '[part="heading"]')!.textContent).to.include('run out');
  });

  it('handed-back: no code, says the terminal received it and the tab can close', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="handed-back"
        code="KQ7M-4XTD"
      ></elohim-imagodei-device-consent-card>
    `);
    expect(q(el, '[part="code"]')).to.equal(null);
    expect(text(el)).to.not.include('KQ7M-4XTD');
    expect(text(el)).to.include('terminal on this machine has received the code');
    expect(text(el)).to.include('close this tab');
  });

  it('declined: says nothing was approved and the device is not recognized', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="declined"
      ></elohim-imagodei-device-consent-card>
    `);
    expect(q(el, '[part="heading"]')!.textContent).to.include('Nothing was approved');
    expect(text(el)).to.include('“workspace” is not recognized as your device');
    expect(q(el, '[part="code"]')).to.equal(null);
  });

  const refusals: [string, string][] = [
    ['request_acts_incoherent', 'without also being enrolled'],
    ['act_unknown', 'doesn’t recognize'],
    ['redemption_expired', 'ran out before it was used'],
    ['consent_unavailable', 'can’t be taken right now'],
    ['something_new', 'Something stopped this request'],
  ];
  for (const [code, sentence] of refusals) {
    it(`refused (${code}): shows a human sentence and the raw code`, async () => {
      const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
        <elohim-imagodei-device-consent-card
          .request=${ENROLL_ONLY}
          phase="refused"
          refusal-code=${code}
        ></elohim-imagodei-device-consent-card>
      `);
      expect(q(el, '[part="message"]')!.textContent).to.include(sentence);
      expect(q(el, '[part="refusal-code"] code')!.textContent).to.equal(code);
    });
  }

  it('moves focus to the new heading on a phase change', async () => {
    const el = await review(ENROLL_ONLY);
    el.phase = 'declined';
    await el.updateComplete;
    const heading = q(el, '[part="heading"]')!;
    expect(el.shadowRoot!.activeElement).to.equal(heading);
  });

  it('does not steal focus on first render', async () => {
    const el = await review(ENROLL_ONLY);
    expect(el.shadowRoot!.activeElement).to.equal(null);
  });
});

// ---------------------------------------------------------------------------
// a11y precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-device-consent-card> — a11y precondition gate', () => {
  const variants: [string, () => ReturnType<typeof html>][] = [
    [
      'review with root key',
      () => html`
        <elohim-imagodei-device-consent-card
          .request=${ENROLL_AND_ROOT}
          person-label="Matthew"
        ></elohim-imagodei-device-consent-card>
      `,
    ],
    [
      'signing',
      () => html`
        <elohim-imagodei-device-consent-card
          .request=${ENROLL_AND_ROOT}
          phase="signing"
        ></elohim-imagodei-device-consent-card>
      `,
    ],
    [
      'code',
      () => html`
        <elohim-imagodei-device-consent-card
          .request=${ENROLL_ONLY}
          phase="code"
          code="KQ7M-4XTD"
          .expiresAt=${Date.now() + 5 * 60_000}
        ></elohim-imagodei-device-consent-card>
      `,
    ],
    [
      'refused',
      () => html`
        <elohim-imagodei-device-consent-card
          .request=${ENROLL_ONLY}
          phase="refused"
          refusal-code="act_unknown"
        ></elohim-imagodei-device-consent-card>
      `,
    ],
  ];
  for (const [name, tpl] of variants) {
    it(`passes axe: ${name}`, async () => {
      const el = await fixture<ElohimImagodeiDeviceConsentCard>(tpl());
      const results = await axe.run(el);
      expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
    });
  }

  it('passes axe with the root key left out (disabled control)', async () => {
    const el = await review(ENROLL_AND_ROOT);
    await toggle(el, 'device.enroll', false);
    const results = await axe.run(el);
    expect(results.violations, JSON.stringify(results.violations, null, 2)).to.have.lengthOf(0);
  });

  it('every act checkbox has an accessible label from its row', async () => {
    const el = await review(ENROLL_AND_ROOT);
    for (const act of ['device.enroll', 'device.bind-root']) {
      expect(checkbox(el, act).closest('label')).to.exist;
    }
  });

  it('controls are keyboard-focusable in reading order', async () => {
    const el = await review(ENROLL_AND_ROOT);
    const order = Array.from(el.shadowRoot!.querySelectorAll<HTMLElement>('input, button')).map(
      n => n.getAttribute('part') ?? n.getAttribute('value')
    );
    expect(order).to.deep.equal(['device.enroll', 'device.bind-root', 'decline', 'approve']);
    for (const n of el.shadowRoot!.querySelectorAll<HTMLElement>('input, button')) {
      expect(n.tabIndex).to.be.greaterThanOrEqual(0);
    }
  });
});

// ---------------------------------------------------------------------------
// theme-contrast precondition gate (system cells — blank-slate)
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-device-consent-card> — theme-contrast precondition gate', () => {
  const CELLS: ThemeCell[] = ['system-light', 'system-dark'];
  for (const cell of CELLS) {
    it(`review holds WCAG contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimImagodeiDeviceConsentCard>(
        html`
          <elohim-imagodei-device-consent-card
            .request=${ENROLL_AND_ROOT}
          ></elohim-imagodei-device-consent-card>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
    });

    it(`code holds WCAG contrast in ${cell}`, async () => {
      const { el } = await themeFixture<ElohimImagodeiDeviceConsentCard>(
        html`
          <elohim-imagodei-device-consent-card
            .request=${ENROLL_ONLY}
            phase="code"
            code="KQ7M-4XTD"
            .expiresAt=${Date.now() + 5 * 60_000}
          ></elohim-imagodei-device-consent-card>
        `,
        cell
      );
      await el.updateComplete;
      assertThemeContrast(el);
    });
  }
});

// ---------------------------------------------------------------------------
// ua-prefs precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-device-consent-card> — ua-prefs precondition gate', () => {
  afterEach(() => clearMediaQueries());

  it('CSS omits transitions and animations (still by default)', () => {
    expect(cssText()).to.not.contain('transition:');
    expect(cssText()).to.not.contain('animation:');
  });

  it('CSS has a forced-colors override block', () => {
    expect(cssText()).to.contain('forced-colors');
  });

  it('CSS enlarges touch targets under coarse pointers', () => {
    expect(cssText()).to.match(/@media \(pointer: coarse\)/);
  });

  it('passes the photosensitive-flash analyzer while the countdown runs', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${ENROLL_ONLY}
        phase="code"
        code="KQ7M-4XTD"
        .expiresAt=${Date.now() + 5 * 60_000}
      ></elohim-imagodei-device-consent-card>
    `);
    const result = await measureLuminanceChanges(el, { sampleMs: 1200, sampleHz: 30 });
    expect(result.exceedsThreshold).to.be.false;
  });
});

// ---------------------------------------------------------------------------
// i18n precondition gate
// ---------------------------------------------------------------------------

describe('<elohim-imagodei-device-consent-card> — i18n precondition gate', () => {
  it('renders in RTL document direction (he-IL) and keeps keys left-to-right', async () => {
    const el = await renderInLocale<ElohimImagodeiDeviceConsentCard>(
      'he-IL',
      html`
        <elohim-imagodei-device-consent-card
          .request=${ENROLL_AND_ROOT}
        ></elohim-imagodei-device-consent-card>
      `
    );
    expect(document.documentElement.getAttribute('dir')).to.equal('rtl');
    expect(q(el, '[part="card"]')!.getBoundingClientRect().width).to.be.greaterThan(0);
    const fp = q(el, '[part="fingerprint"]')!;
    expect(getComputedStyle(fp).direction).to.equal('ltr');
  });

  it('isolates person-supplied names so they cannot reorder the sentence', async () => {
    const el = await fixture<ElohimImagodeiDeviceConsentCard>(html`
      <elohim-imagodei-device-consent-card
        .request=${{ ...ENROLL_ONLY, label: 'עבודה' }}
        person-label="Matthew"
        host-label="alpha"
      ></elohim-imagodei-device-consent-card>
    `);
    expect(q(el, '[part="heading"]')!.textContent).to.include('⁨עבודה⁩');
    expect(q(el, '[part="signed-in"]')!.textContent).to.include('⁨Matthew⁩');
    expect(q(el, '[part="signer"]')!.textContent).to.include('⁨alpha⁩');
  });

  it('uses no physical CSS properties (only logical or non-positional)', () => {
    const findings = requiresLogicalProperties(cssText());
    expect(findings, JSON.stringify(findings, null, 2)).to.have.lengthOf(0);
  });

  it('fits a 320px-wide container without horizontal overflow', async () => {
    const wrapper = await fixture<HTMLDivElement>(html`
      <div style="inline-size: 320px;">
        <elohim-imagodei-device-consent-card
          .request=${ENROLL_AND_ROOT}
          person-label="Matthew"
        ></elohim-imagodei-device-consent-card>
      </div>
    `);
    const el = wrapper.firstElementChild as ElohimImagodeiDeviceConsentCard;
    await el.updateComplete;
    const card = q(el, '[part="card"]')!;
    expect(card.scrollWidth).to.be.at.most(320);
  });
});
