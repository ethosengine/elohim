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
  isLamadRoute = false;

  constructor(private readonly router: Router) {}

  ngOnInit(): void {
    // Track route changes to hide floating toggle on lamad routes
    this.router.events
      .pipe(filter(event => event instanceof NavigationEnd))
      .subscribe((event: NavigationEnd) => {
        this.isLamadRoute = event.url.startsWith('/lamad');
      });

    // Check initial route
    this.isLamadRoute = this.router.url.startsWith('/lamad');
  }
}
