import { ChangeDetectionStrategy, Component, Input, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';

import { ThemeService, Theme } from '../../services/theme.service';

/**
 * Cycles device → light → dark. Drawn with inline SVG rather than emoji: an
 * emoji glyph depends on the reader's installed fonts and rendered as a bare
 * dot where none was available.
 */
@Component({
  selector: 'app-theme-toggle',
  standalone: true,
  templateUrl: './theme-toggle.component.html',
  changeDetection: ChangeDetectionStrategy.OnPush,
  styleUrl: './theme-toggle.component.css',
})
export class ThemeToggleComponent {
  @Input() inline = false;

  private readonly themeService = inject(ThemeService);

  /** The chosen theme, kept in step with ThemeService. */
  readonly currentTheme = toSignal(this.themeService.getTheme(), {
    initialValue: this.themeService.getCurrentTheme(),
  });

  readonly tooltip = computed(() => this.tooltipFor(this.currentTheme()));

  toggleTheme(): void {
    this.themeService.cycleTheme();
  }

  isAutoMode(): boolean {
    return this.currentTheme() === 'device';
  }

  getTooltip(): string {
    return this.tooltip();
  }

  private tooltipFor(theme: Theme): string {
    switch (theme) {
      case 'light':
        return 'Light mode. Switch to dark';
      case 'dark':
        return 'Dark mode. Switch to match your device';
      case 'device':
        return 'Auto mode, matching your device. Switch to light';
      default:
        return 'Change theme';
    }
  }
}
