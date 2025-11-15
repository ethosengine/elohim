import { Injectable, Renderer2, RendererFactory2 } from '@angular/core';
import { BehaviorSubject, Observable } from 'rxjs';

export type Theme = 'light' | 'dark' | 'device';

@Injectable({
  providedIn: 'root'
})
export class ThemeService {
  private renderer: Renderer2;
  private currentTheme$ = new BehaviorSubject<Theme>('device');

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
   */
  private saveTheme(theme: Theme): void {
    try {
      localStorage.setItem('elohim-theme', theme);
    } catch (e) {
      console.warn('Failed to save theme preference:', e);
    }
  }

  /**
   * Load theme preference from localStorage
   */
  private loadTheme(): void {
    try {
      const savedTheme = localStorage.getItem('elohim-theme') as Theme;
      if (savedTheme && ['light', 'dark', 'device'].includes(savedTheme)) {
        this.setTheme(savedTheme);
      } else {
        this.setTheme('device');
      }
    } catch (e) {
      console.warn('Failed to load theme preference:', e);
      this.setTheme('device');
    }
  }
}
