import { Injectable, Renderer2, RendererFactory2 } from '@angular/core';

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

  getTheme(): Observable<Theme> {
    return this.currentTheme$.asObservable();
  }

  getCurrentTheme(): Theme {
    return this.currentTheme$.value;
  }

  cycleTheme(): void {
    const themes: Theme[] = ['device', 'light', 'dark'];
    const currentIndex = themes.indexOf(this.currentTheme$.value);
    const nextIndex = (currentIndex + 1) % themes.length;
    this.setTheme(themes[nextIndex]);
  }

  setTheme(theme: Theme): void {
    this.currentTheme$.next(theme);
    this.applyTheme(theme);
    this.saveTheme(theme);
  }

  private applyTheme(theme: Theme): void {
    const body = document.body;

    this.renderer.removeClass(body, 'theme-light');
    this.renderer.removeClass(body, 'theme-dark');
    this.renderer.removeClass(body, 'theme-device');

    this.renderer.addClass(body, `theme-${theme}`);
    this.renderer.setAttribute(body, 'data-theme', theme);
  }

  private saveTheme(theme: Theme): void {
    try {
      localStorage.setItem('doorway-theme', theme);
    } catch {
      // localStorage write failure is non-critical
    }
  }

  private loadTheme(): void {
    try {
      const savedTheme = localStorage.getItem('doorway-theme') as Theme;
      if (savedTheme && ['light', 'dark', 'device'].includes(savedTheme)) {
        this.setTheme(savedTheme);
      } else {
        this.setTheme('device');
      }
    } catch {
      this.setTheme('device');
    }
  }
}
