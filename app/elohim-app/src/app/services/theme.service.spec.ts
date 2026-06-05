import { TestBed } from '@angular/core/testing';
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
    window.dispatchEvent(
      new StorageEvent('storage', { key: 'elohim-theme', newValue: 'dark' }),
    );
    window.removeEventListener('elohim-theme-changed', onEvent);
    expect(service.getCurrentTheme()).toBe('dark');
    expect(events).toBe(0); // adoptExternal never re-dispatches
  });
});
