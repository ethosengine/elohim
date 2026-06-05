import { Injectable, Renderer2, RendererFactory2, inject } from '@angular/core';

// @coverage: 96.3% (2026-02-24)

import { BehaviorSubject, Observable } from 'rxjs';

export type Theme = 'light' | 'dark' | 'device';

const THEME_STORAGE_KEY = 'elohim-theme';
const THEME_CHANGE_EVENT = 'elohim-theme-changed';

@Injectable({
  providedIn: 'root',
})
export class ThemeService {
  private readonly renderer: Renderer2;
  private readonly currentTheme$ = new BehaviorSubject<Theme>('device');

  private readonly rendererFactory = inject(RendererFactory2);

  constructor() {
    this.renderer = this.rendererFactory.createRenderer(null, null);
    this.loadTheme();

    // Cross-island/tab sync: Lit's ThemeStore (elohim-core) speaks the same
    // contract; adopt its changes without re-dispatching (no loops).
    globalThis.addEventListener('storage', (e: StorageEvent) => {
      if (e.key === THEME_STORAGE_KEY && this.isValidTheme(e.newValue)) {
        this.adoptExternal(e.newValue);
      }
    });
    globalThis.addEventListener(THEME_CHANGE_EVENT, (e: Event) => {
      const theme = (e as CustomEvent<{ theme?: unknown }>).detail?.theme;
      if (this.isValidTheme(theme)) this.adoptExternal(theme);
    });
  }

  /**
   * Get the current theme as an observable
   */
  getTheme(): Observable<Theme> {
    return this.currentTheme$.asObservable();
  }

  /**
   * Get the current theme value
   */
  getCurrentTheme(): Theme {
    return this.currentTheme$.value;
  }

  /**
   * Cycle to the next theme: device -> light -> dark -> device
   */
  cycleTheme(): void {
    const themes: Theme[] = ['device', 'light', 'dark'];
    const currentIndex = themes.indexOf(this.currentTheme$.value);
    const nextIndex = (currentIndex + 1) % themes.length;
    this.setTheme(themes[nextIndex]);
  }

  /**
   * Set a specific theme
   */
  setTheme(theme: Theme): void {
    this.currentTheme$.next(theme);
    this.applyTheme(theme);
    this.saveTheme(theme);
    globalThis.dispatchEvent(new CustomEvent(THEME_CHANGE_EVENT, { detail: { theme } }));
  }

  /**
   * Apply the theme to the document.
   * html is the AUTHORITY (tokens.scss :root[data-theme] + color-scheme key
   * off it — chrome var-chains are declared on :root and substitute there);
   * body keeps the attribute for legacy body[data-theme] descendant selectors.
   * Twin contract with elohim-core's ThemeStore — change BOTH or NEITHER.
   */
  private applyTheme(theme: Theme): void {
    for (const target of [document.documentElement, document.body]) {
      this.renderer.removeClass(target, 'theme-light');
      this.renderer.removeClass(target, 'theme-dark');
      this.renderer.removeClass(target, 'theme-device');
      this.renderer.addClass(target, `theme-${theme}`);
      this.renderer.setAttribute(target, 'data-theme', theme);
    }
  }

  /**
   * Save theme preference to localStorage
   *
   * SECURITY NOTE: localStorage usage is safe here.
   * - Only stores non-sensitive user UI preference (theme selection)
   * - No personal identifiable information (PII) is stored
   * - No authentication tokens or credentials are stored
   * - Data is client-side only and used for UI personalization
   * - Limited to predefined theme values (light, dark, device)
   */
  private saveTheme(theme: Theme): void {
    try {
      localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // localStorage write failure is non-critical
    }
  }

  /**
   * Load theme preference from localStorage
   *
   * SECURITY NOTE: localStorage usage is safe here.
   * - Only reads non-sensitive user UI preference (theme selection)
   * - Validates input against allowed values before use
   * - Falls back to safe default if validation fails
   * - No risk of code injection as value is type-checked
   */
  private loadTheme(): void {
    try {
      const savedTheme = localStorage.getItem(THEME_STORAGE_KEY);
      if (this.isValidTheme(savedTheme)) {
        this.setTheme(savedTheme);
      } else {
        this.setTheme('device');
      }
    } catch {
      // localStorage read failure - fallback to default theme
      this.setTheme('device');
    }
  }

  private isValidTheme(v: unknown): v is Theme {
    return v === 'light' || v === 'dark' || v === 'device';
  }

  /** External change (Lit island / other tab): apply + emit, never re-persist/dispatch. */
  private adoptExternal(theme: Theme): void {
    if (theme === this.currentTheme$.value) return;
    this.currentTheme$.next(theme);
    this.applyTheme(theme);
  }
}
