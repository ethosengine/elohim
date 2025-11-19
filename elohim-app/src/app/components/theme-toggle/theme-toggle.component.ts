import { Component, OnInit, OnDestroy, Input } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ThemeService, Theme } from '../../services/theme.service';
import { Subscription } from 'rxjs';

@Component({
  selector: 'app-theme-toggle',
  imports: [CommonModule],
  templateUrl: './theme-toggle.component.html',
  styleUrl: './theme-toggle.component.css'
})
export class ThemeToggleComponent implements OnInit, OnDestroy {
  @Input() inline = false;
  currentTheme: Theme = 'device';
  private themeSubscription?: Subscription;

  constructor(private readonly themeService: ThemeService) {}

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
      // Check system preference
      return window.matchMedia('(prefers-color-scheme: light)').matches ? 'light' : 'dark';
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
