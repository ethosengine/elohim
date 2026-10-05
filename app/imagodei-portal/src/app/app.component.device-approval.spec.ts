/** @vitest-environment jsdom */

import '@analogjs/vitest-angular/setup-zone';
import '@angular/compiler';

import { getTestBed, TestBed } from '@angular/core/testing';
import { BrowserTestingModule, platformBrowserTesting } from '@angular/platform-browser/testing';
import { afterEach, beforeAll, beforeEach, describe, expect, it } from 'vitest';

import { AppComponent } from './app.component.js';
import { DEVICE_APPROVAL_PORT } from './device-approval/device-approval-port.js';

const settle = () => new Promise<void>(resolve => setTimeout(resolve, 0));

/** The header the native portal shows over its device approval page. */
describe('AppComponent — the native device approval page’s header', () => {
  beforeAll(() => {
    getTestBed().initTestEnvironment(BrowserTestingModule, platformBrowserTesting());
  });

  beforeEach(() => {
    window.history.replaceState(null, '', '/auth/portal/consent/device?request=e30');
    globalThis.fetch = async () => new Response('not found', { status: 404 });
  });

  afterEach(() => TestBed.resetTestingModule());

  async function render(): Promise<HTMLElement> {
    TestBed.configureTestingModule({
      imports: [AppComponent],
      providers: [
        {
          provide: DEVICE_APPROVAL_PORT,
          useValue: {
            client: {
              view: async () => ({ ok: false, status: 404, body: null }),
              agree: async () => ({ ok: false, status: 404, body: null }),
            },
            signIn: () => undefined,
            handBack: () => undefined,
          },
        },
      ],
    });
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    await fixture.componentInstance.ngOnInit();
    await settle();
    fixture.detectChanges();
    return fixture.nativeElement as HTMLElement;
  }

  it('says, in this host’s words, that the person’s own device holds their key', async () => {
    const root = await render();
    const chip = root.querySelector('[data-testid="device-consent-trust"]') as
      | (HTMLElement & { strings?: { ownNodeLabel?: string } })
      | null;
    expect(chip?.getAttribute('slot')).toBe('header');
    expect(chip?.getAttribute('trust-mode')).toBe('peer-conductor');
    expect(chip?.strings?.ownNodeLabel).toBe('Your own device holds your key');
  });

  it('replaces the default header, so no "hosted" chip and no witness line appear', async () => {
    const root = await render();
    const header = root.querySelectorAll('[slot="header"]');
    expect(header).toHaveLength(1);
    expect(root.querySelector('elohim-imagodei-attestor-row')).toBeNull();
    expect(root.textContent).not.toMatch(/hosted via|no witnesses/i);
  });
});
