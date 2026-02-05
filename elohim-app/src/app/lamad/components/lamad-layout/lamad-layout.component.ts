import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterOutlet, RouterLink, Router, NavigationEnd } from '@angular/router';

// @coverage: 52.9% (2026-02-05)

import { filter, takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import { ElohimNavigatorComponent } from '@app/elohim/components/elohim-navigator/elohim-navigator.component';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';

import { RendererInitializerService } from '../../renderers/renderer-initializer.service';

@Component({
  selector: 'app-lamad-layout',
  standalone: true,
  imports: [CommonModule, RouterOutlet, RouterLink, FormsModule, ElohimNavigatorComponent],
  templateUrl: './lamad-layout.component.html',
  styleUrls: ['./lamad-layout.component.css'],
})
export class LamadLayoutComponent implements OnInit, OnDestroy {
  isReady = false;
  isHomePage = false;

  private readonly destroy$ = new Subject<void>();

  constructor(
    private readonly dataLoader: DataLoaderService,
    private readonly router: Router,
    // Injecting RendererInitializerService triggers renderer registration
    private readonly _rendererInit: RendererInitializerService
  ) {}

  ngOnInit(): void {
    // Lightweight check that data layer is accessible
    // Uses checkReadiness() instead of getContentIndex() to avoid loading all content
    this.dataLoader
      .checkReadiness()
      .pipe(takeUntil(this.destroy$))
      .subscribe({
        next: () => {
          this.isReady = true;
        },
        error: () => {
          this.isReady = true; // Still mark ready to show error state
        },
      });

    // Track route for UI state
    this.router.events
      .pipe(
        filter(event => event instanceof NavigationEnd),
        takeUntil(this.destroy$)
      )
      .subscribe(() => {
        this.checkIfHomePage();
      });

    this.checkIfHomePage();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  private checkIfHomePage(): void {
    this.isHomePage = this.router.url === '/lamad' || this.router.url === '/lamad/';
  }
}
