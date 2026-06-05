import { expect, fixture, html } from '@open-wc/testing';

import {
  baseHrefOwnsPath,
  installEprLinkInterceptor,
  recordCrossBundleHandoff,
} from './epr-link-interceptor.js';

const NAV_STACK_KEY = 'elohim.session-nav-stack.v1';

describe('epr-link-interceptor', () => {
  let uninstall: (() => void) | null = null;
  let assigned: string[] = [];

  // Bubble-phase guard: registered in capture=false so it fires AFTER the
  // capture-phase interceptor. If the interceptor called stopImmediatePropagation
  // in capture phase, this guard never runs. Tracks whether propagation reached
  // the bubble phase (pass-through observable path).
  let bubbleReached = false;
  let guard: (e: Event) => void;

  function install(opts: Parameters<typeof installEprLinkInterceptor>[0] = {}): void {
    uninstall = installEprLinkInterceptor({
      ...opts,
      // test seam: capture instead of really navigating
      assign: (href: string) => assigned.push(href),
    });
  }

  beforeEach(() => {
    bubbleReached = false;
    guard = (e: Event) => {
      e.preventDefault(); // prevent any real navigation in test runner
      bubbleReached = true;
    };
    document.addEventListener('click', guard, false);
  });

  afterEach(() => {
    document.removeEventListener('click', guard, false);
    uninstall?.();
    uninstall = null;
    assigned = [];
    bubbleReached = false;
    sessionStorage.removeItem(NAV_STACK_KEY);
  });

  async function clickAnchor(href: string, mod: Partial<MouseEventInit> = {}): Promise<MouseEvent> {
    const a = await fixture<HTMLAnchorElement>(html`<a href=${href}>link</a>`);
    const ev = new MouseEvent('click', { bubbles: true, cancelable: true, composed: true, ...mod });
    a.dispatchEvent(ev);
    return ev;
  }

  it('intercepts a cross-bundle anchor: preventDefault + assign + handoff record', async () => {
    install({ ownsPath: () => false });
    const ev = await clickAnchor('/lamad');
    // Intercepted: preventDefault called, assign called, bubble guard NOT reached (stopImmediatePropagation)
    expect(ev.defaultPrevented).to.be.true;
    expect(assigned).to.deep.equal(['/lamad']);
    expect(bubbleReached).to.be.false;
    const stack = JSON.parse(sessionStorage.getItem(NAV_STACK_KEY) ?? '[]') as unknown[];
    expect(stack).to.have.length(1);
  });

  it('passes through same-bundle anchors untouched', async () => {
    install({ ownsPath: () => true });
    await clickAnchor('/community');
    // Pass-through: assign NOT called, bubble guard DID reach
    expect(assigned).to.be.empty;
    expect(bubbleReached).to.be.true;
  });

  it('passes through modified clicks, _blank targets, downloads, hash links, bypass-marked', async () => {
    install({ ownsPath: () => false });

    // ctrl-click
    await clickAnchor('/lamad', { ctrlKey: true });
    expect(assigned).to.be.empty;
    expect(bubbleReached).to.be.true;

    // reset bubble tracker between sub-assertions
    bubbleReached = false;

    // meta-click
    await clickAnchor('/lamad', { metaKey: true });
    expect(assigned).to.be.empty;
    expect(bubbleReached).to.be.true;

    bubbleReached = false;

    // _blank target
    const blank = await fixture<HTMLAnchorElement>(html`<a href="/lamad" target="_blank">x</a>`);
    const evBlank = new MouseEvent('click', { bubbles: true, cancelable: true });
    blank.dispatchEvent(evBlank);
    expect(assigned).to.be.empty;
    expect(bubbleReached).to.be.true;

    bubbleReached = false;

    // download attribute
    const dl = await fixture<HTMLAnchorElement>(html`<a href="/lamad" download>x</a>`);
    const evDl = new MouseEvent('click', { bubbles: true, cancelable: true });
    dl.dispatchEvent(evDl);
    expect(assigned).to.be.empty;
    expect(bubbleReached).to.be.true;

    bubbleReached = false;

    // hash-only href
    await clickAnchor('#frag');
    expect(assigned).to.be.empty;
    expect(bubbleReached).to.be.true;

    bubbleReached = false;

    // data-epr-bypass attribute
    const bypass = await fixture<HTMLAnchorElement>(html`<a href="/lamad" data-epr-bypass>x</a>`);
    const evBy = new MouseEvent('click', { bubbles: true, cancelable: true });
    bypass.dispatchEvent(evBy);
    expect(assigned).to.be.empty;
    expect(bubbleReached).to.be.true;
  });

  it('calls beforeCrossBundle instead of the default handoff when provided', async () => {
    let called: string | null = null;
    install({ ownsPath: () => false, beforeCrossBundle: (href) => (called = href) });
    await clickAnchor('/lamad?x=1');
    expect(called).to.equal('/lamad?x=1');
    expect(sessionStorage.getItem(NAV_STACK_KEY)).to.be.null;
    // Still intercepted — assign called, bubble guard NOT reached
    expect(assigned).to.deep.equal(['/lamad?x=1']);
    expect(bubbleReached).to.be.false;
  });

  it('explicit install replaces a default install; default never replaces', () => {
    const u1 = installEprLinkInterceptor({ assign: () => undefined });
    const u2 = installEprLinkInterceptor({ assign: () => undefined }); // default vs existing → no-op handle
    const u3 = installEprLinkInterceptor({ explicit: true, ownsPath: () => true, assign: () => undefined });
    u2(); // must be safe and must NOT remove the active explicit install
    u3();
    u1();
    expect(window.__elohimEprLinkInterceptor).to.be.undefined;
  });

  it('recordCrossBundleHandoff caps the stack at 32 entries', () => {
    for (let i = 0; i < 40; i++) recordCrossBundleHandoff(`cid-${i}`);
    const stack = JSON.parse(sessionStorage.getItem(NAV_STACK_KEY) ?? '[]') as unknown[];
    expect(stack).to.have.length(32);
  });

  it('baseHrefOwnsPath owns everything under a "/" base', () => {
    // wtr serves with base "/", so the heuristic owns all paths here
    expect(baseHrefOwnsPath('/anything')).to.be.true;
  });
});
