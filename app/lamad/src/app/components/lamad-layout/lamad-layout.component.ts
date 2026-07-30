import { CommonModule } from '@angular/common';
import {
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  OnInit,
  OnDestroy,
} from '@angular/core';
import { FormsModule } from '@angular/forms';
import { RouterOutlet, Router, NavigationEnd } from '@angular/router';

// @coverage: 52.9% (2026-02-24)

import { filter, takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import 'elohim-core/register';
import type { ElohimContextAppConfig } from 'elohim-core';

import { DataLoaderService } from '../../services/data-loader.service';

import { RendererInitializerService } from '../../renderers/renderer-initializer.service';

import { SyncProgressComponent } from '../sync-progress/sync-progress.component';

@Component({
  selector: 'app-lamad-layout',
  standalone: true,
  imports: [CommonModule, RouterOutlet, FormsModule, SyncProgressComponent],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  templateUrl: './lamad-layout.component.html',
  styleUrls: ['./lamad-layout.component.css'],
})
export class LamadLayoutComponent implements OnInit, OnDestroy {
  isReady = false;
  isHomePage = false;

  /** Host-supplied navigator config (§12.3: routing lives in the host, not the primitive). */
  readonly navigatorApps: ElohimContextAppConfig[] = [
    {
      id: 'lamad',
      name: 'Lamad',
      icon: '📚',
      route: '/lamad', // route-literal-ok: navigator app-config route declaration (host routing config, not a minted content link)
      tagline: 'Learning & Content',
      available: true,
    },
    {
      id: 'community',
      name: 'Qahal',
      icon: '👥',
      route: '/community',
      tagline: 'Community & Governance',
      available: true,
    },
    {
      id: 'shefa',
      name: 'Shefa',
      icon: '✨',
      route: '/shefa',
      tagline: 'Economics of Flourishing',
      available: true,
    },
    {
      id: 'avodah',
      name: 'Avodah',
      icon: '🔨',
      route: '/avodah',
      tagline: 'Work & Stewardship',
      available: true,
    },
    {
      id: 'map',
      name: 'Map',
      icon: '🌍',
      route: '/map',
      tagline: 'Living Places',
      available: true,
    },
  ];

  readonly navigatorIdentityRoutes = {
    profile: '/identity/profile',
    login: '/identity/login',
    register: '/identity/register',
  } as const;

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
    // router.url is already base-stripped (bundle served with <base href="/lamad/">), // route-literal-ok: doc comment describing base-href stripping
    // so the home route is the bare '/'. Tolerate a trailing query string and the
    // empty string the Router reports before the first navigation commits.
    this.isHomePage =
      this.router.url === '/' || this.router.url === '' || this.router.url.startsWith('/?');
  }

  /**
   * Navigator routes are protocol-absolute. Lamad-owned ones go through this
   * bundle's router (base-href '/lamad/' is stripped); everything else is a // route-literal-ok: doc comment describing base-href stripping
   * cross-bundle handoff to the doorway-projected address.
   */
  onNavigatorNavigate(event: Event): void {
    const route = (event as CustomEvent<{ route?: string }>).detail?.route;
    if (!route) return;
    // lint-route-literals requires its annotation on the same line as the literal,
    // and the line is over the print width — keep prettier off it.
    // prettier-ignore
    if (route === '/lamad' || route.startsWith('/lamad/')) { // route-literal-ok: own-bundle prefix detection (base-href handoff), not a minted link
      void this.router.navigateByUrl(route.slice('/lamad'.length) || '/'); // route-literal-ok: base-href strip length, not a minted link
    } else {
      globalThis.location.assign(route);
    }
  }
}
