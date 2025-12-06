import { Component, OnInit } from '@angular/core';
import { RouterOutlet, Router, NavigationEnd } from '@angular/router';
import { CommonModule } from '@angular/common';
import { filter } from 'rxjs/operators';
import { ThemeToggleComponent } from './components/theme-toggle/theme-toggle.component';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, ThemeToggleComponent, CommonModule],
  templateUrl: './app.component.html',
  styleUrl: './app.component.css'
})
export class AppComponent implements OnInit {
  title = 'elohim-app';
  /** Show floating theme toggle only on root landing page */
  showFloatingToggle = false;

  constructor(private readonly router: Router) {}

  ngOnInit(): void {
    // Track route changes to show floating toggle only on root landing page
    this.router.events
      .pipe(filter(event => event instanceof NavigationEnd))
      .subscribe((event: NavigationEnd) => {
        this.showFloatingToggle = this.isRootLandingPage(event.url);
      });

    // Check initial route
    this.showFloatingToggle = this.isRootLandingPage(this.router.url);
  }

  /** Check if URL is the root landing page (/ or empty) */
  private isRootLandingPage(url: string): boolean {
    // Strip query params and fragments
    const path = url.split('?')[0].split('#')[0];
    return path === '/' || path === '';
  }
}
