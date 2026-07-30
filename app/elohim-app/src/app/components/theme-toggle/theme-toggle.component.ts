import { CommonModule } from '@angular/common';
import {
  Component,
  Input,
  OnDestroy,
  OnInit,
  inject,
} from '@angular/core';

// @coverage: 84.2% (2026-02-24)

import { Subscription } from 'rxjs';

import { ThemeService, Theme } from '../../services/theme.service';

@Component({
  selector: 'app-theme-toggle',
  imports: [CommonModule],
  templateUrl: './theme-toggle.component.html',
  styleUrl: './theme-toggle.component.css',
})
export class ThemeToggleComponent implements OnInit, OnDestroy {
  @Input() inline = false;
  currentTheme: Theme = 'device';
  private themeSubscription?: Subscription;

  private readonly themeService = inject(ThemeService);

  ngOnInit(): void {
    this.themeSubscription = this.themeService.getTheme().subscribe(theme => {
      this.currentTheme = theme;
    });
  }

  ngOnDestroy(): void {
    this.themeSubscription?.unsubscribe();
  }

  toggleTheme(): void {
    this.themeService.cycleTheme();
  }

  getIcon(): string {
    // Show sun/moon based on actual effective theme
    const effectiveTheme = this.getEffectiveTheme();
    return effectiveTheme === 'light' ? '☀️' : '🌙';
  }

  isAutoMode(): boolean {
    return this.currentTheme === 'device';
  }

  private getEffectiveTheme(): 'light' | 'dark' {
    if (this.currentTheme === 'device') {
      // System preference (matchMedia) is a browser-only signal — absent in the
      // V8 SSR runtime, where this getter is reached synchronously via the
      // template's {{ getIcon() }}. Default to dark server-side; the client
      // re-evaluates on hydration.
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: inside typeof globalThis.matchMedia guard
      if (typeof globalThis.matchMedia !== 'function') return 'dark';
      // eslint-disable-next-line no-restricted-syntax -- SSR-safe: guarded by the typeof globalThis.matchMedia check above
      return globalThis.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
    }
    return this.currentTheme;
  }

  getTooltip(): string {
    switch (this.currentTheme) {
      case 'light':
        return 'Light mode - Click to switch to dark';
      case 'dark':
        return 'Dark mode - Click to switch to auto';
      case 'device':
        return 'Auto mode - Click to switch to light';
      default:
        return 'Toggle theme';
    }
  }
}
