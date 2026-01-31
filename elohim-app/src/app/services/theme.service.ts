import { Injectable, Renderer2, RendererFactory2 } from '@angular/core';

// @coverage: 96.3% (2026-01-31)

import { BehaviorSubject, Observable } from 'rxjs';

export type Theme = 'light' | 'dark' | 'device';

@Injectable({
  providedIn: 'root',
})
export class ThemeService {
  private readonly renderer: Renderer2;
  private readonly currentTheme$ = new BehaviorSubject<Theme>('device');

  constructor(rendererFactory: RendererFactory2) {
    this.renderer = rendererFactory.createRenderer(null, null);
    this.loadTheme();
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
  }

  /**
   * Apply the theme to the document
   */
  private applyTheme(theme: Theme): void {
    const body = document.body;

    // Remove any existing theme classes
    this.renderer.removeClass(body, 'theme-light');
    this.renderer.removeClass(body, 'theme-dark');
    this.renderer.removeClass(body, 'theme-device');

    // Apply new theme class
    this.renderer.addClass(body, `theme-${theme}`);

    // Set data attribute for CSS targeting
    this.renderer.setAttribute(body, 'data-theme', theme);
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
      localStorage.setItem('elohim-theme', theme);
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
      const savedTheme = localStorage.getItem('elohim-theme') as Theme;
      if (savedTheme && ['light', 'dark', 'device'].includes(savedTheme)) {
        this.setTheme(savedTheme);
      } else {
        this.setTheme('device');
      }
    } catch {
      // localStorage read failure - fallback to default theme
      this.setTheme('device');
    }
  }
}
