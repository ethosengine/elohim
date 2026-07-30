import { DOCUMENT } from '@angular/common';
import { PLATFORM_ID } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { vi } from 'vitest';

import { ThemeService } from './theme.service';

describe('ThemeService', () => {
  let service: ThemeService;

  beforeEach(() => {
    localStorage.clear();
    TestBed.configureTestingModule({});
    service = TestBed.inject(ThemeService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should default to device theme', () => {
    expect(service.getCurrentTheme()).toBe('device');
  });

  it('should cycle through themes', () => {
    service.setTheme('device');
    expect(service.getCurrentTheme()).toBe('device');

    service.cycleTheme();
    expect(service.getCurrentTheme()).toBe('light');

    service.cycleTheme();
    expect(service.getCurrentTheme()).toBe('dark');

    service.cycleTheme();
    expect(service.getCurrentTheme()).toBe('device');
  });

  it('should save theme to localStorage', () => {
    service.setTheme('dark');
    expect(localStorage.getItem('elohim-theme')).toBe('dark');
  });

  it('should apply theme class to body', () => {
    service.setTheme('light');
    expect(document.body.classList.contains('theme-light')).toBe(true);
    expect(document.body.getAttribute('data-theme')).toBe('light');
  });

  it('dual-writes data-theme to documentElement (authority) and body (compat)', () => {
    service.setTheme('dark');
    expect(document.documentElement.getAttribute('data-theme')).toBe('dark');
    expect(document.documentElement.classList.contains('theme-dark')).toBe(true);
    expect(document.body.getAttribute('data-theme')).toBe('dark');
    service.setTheme('light');
    expect(document.documentElement.getAttribute('data-theme')).toBe('light');
    expect(document.documentElement.classList.contains('theme-dark')).toBe(false);
  });

  it('adopts an elohim-theme-changed event from a Lit island without re-dispatching', () => {
    let events = 0;
    const onEvent = (): void => {
      events += 1;
    };
    window.addEventListener('elohim-theme-changed', onEvent);
    window.dispatchEvent(new CustomEvent('elohim-theme-changed', { detail: { theme: 'dark' } }));
    window.removeEventListener('elohim-theme-changed', onEvent);
    expect(service.getCurrentTheme()).toBe('dark');
    expect(document.body.getAttribute('data-theme')).toBe('dark');
    expect(events).toBe(1); // only the one we dispatched
  });

  it('dispatches elohim-theme-changed when Angular sets the theme (Lit follows)', () => {
    let detail: { theme?: string } | null = null;
    const onEvent = (e: Event): void => {
      detail = (e as CustomEvent<{ theme: string }>).detail;
    };
    window.addEventListener('elohim-theme-changed', onEvent);
    service.setTheme('light');
    window.removeEventListener('elohim-theme-changed', onEvent);
    expect(detail).toEqual({ theme: 'light' });
  });

  it('adopts a cross-tab storage event without dispatching elohim-theme-changed', () => {
    let events = 0;
    const onEvent = (): void => {
      events += 1;
    };
    window.addEventListener('elohim-theme-changed', onEvent);
    window.dispatchEvent(new StorageEvent('storage', { key: 'elohim-theme', newValue: 'dark' }));
    window.removeEventListener('elohim-theme-changed', onEvent);
    expect(service.getCurrentTheme()).toBe('dark');
    expect(events).toBe(0); // adoptExternal never re-dispatches
  });
});

// The V8 SSR runtime (deno_core) has no global `document`, `localStorage`, or
// `globalThis.addEventListener`. This service is `providedIn: 'root'` and is
// instantiated during SSR bootstrap, so any unguarded browser-global access in
// its constructor path throws `ReferenceError: document is not defined`, the
// injector fails, and the render comes back as an empty shell (the 2026-07-05
// blank-landing root cause). These guard the SSR contract on the server path.
describe('ThemeService — SSR safety (server platform)', () => {
  beforeEach(() => {
    localStorage.clear();
    document.documentElement.removeAttribute('data-theme');
    document.body.removeAttribute('data-theme');
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [{ provide: PLATFORM_ID, useValue: 'server' }],
    });
  });

  it('constructs and defaults to device without throwing (regression: document is not defined)', () => {
    const service = TestBed.inject(ThemeService);
    expect(service).toBeTruthy();
    expect(service.getCurrentTheme()).toBe('device');
  });

  it('never touches localStorage on the server path', () => {
    const getSpy = vi.spyOn(Storage.prototype, 'getItem');
    const setSpy = vi.spyOn(Storage.prototype, 'setItem');
    const service = TestBed.inject(ThemeService); // constructor runs loadTheme
    service.setTheme('dark'); // would call saveTheme in the browser
    expect(service.getCurrentTheme()).toBe('dark'); // in-memory state still updates
    expect(getSpy).not.toHaveBeenCalled();
    expect(setSpy).not.toHaveBeenCalled();
    expect(localStorage.getItem('elohim-theme')).toBeNull();
    getSpy.mockRestore();
    setSpy.mockRestore();
  });

  it('does not register the theme window listeners on the server path', () => {
    const addSpy = vi.spyOn(globalThis, 'addEventListener');
    TestBed.inject(ThemeService); // constructor would register listeners in a browser
    expect(addSpy).not.toHaveBeenCalledWith('storage', expect.any(Function));
    expect(addSpy).not.toHaveBeenCalledWith('elohim-theme-changed', expect.any(Function));
    addSpy.mockRestore();
  });

  it('themes the INJECTED DOCUMENT (not the global) so the SSR HTML carries data-theme', () => {
    const service = TestBed.inject(ThemeService);
    service.setTheme('light');
    // Applied via the injected DOCUMENT token, which platform-server provides
    // during SSR — the fix for the global-`document` ReferenceError.
    expect(TestBed.inject(DOCUMENT).documentElement.getAttribute('data-theme')).toBe('light');
  });
});
