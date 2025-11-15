import { Component, OnInit, OnDestroy } from '@angular/core';
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
    switch (this.currentTheme) {
      case 'light':
        return '☀️';
      case 'dark':
        return '🌙';
      case 'device':
        return '💻';
      default:
        return '💻';
    }
  }

  getTooltip(): string {
    switch (this.currentTheme) {
      case 'light':
        return 'Light mode - Click to switch to dark';
      case 'dark':
        return 'Dark mode - Click to switch to device default';
      case 'device':
        return 'Device default - Click to switch to light';
      default:
        return 'Toggle theme';
    }
  }
}
