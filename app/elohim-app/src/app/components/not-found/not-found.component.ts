import { CommonModule } from '@angular/common';
import {
  Component,
  CUSTOM_ELEMENTS_SCHEMA,
  OnInit,
  inject,
  ChangeDetectionStrategy,
} from '@angular/core';
import { Router, RouterModule } from '@angular/router';
import 'elohim-core/register';

// @coverage: 100.0% (2026-02-24)

import { EprNavService } from '../../elohim/services/epr-nav.service';
import { SeoService } from '../../services/seo.service';

/**
 * NotFoundComponent - 404 Page
 *
 * Displays a friendly 404 error page with:
 * - Clear messaging that the page wasn't found
 * - Helpful navigation options
 * - Consistent branding
 * - Proper SEO (noindex)
 */
@Component({
  selector: 'app-not-found',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './not-found.component.html',
  styleUrl: './not-found.component.css',
  changeDetection: ChangeDetectionStrategy.Eager,
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
})
export class NotFoundComponent implements OnInit {
  private readonly seoService = inject(SeoService);
  private readonly router = inject(Router);
  private readonly eprNav = inject(EprNavService);

  /** The attempted URL path */
  attemptedUrl = '';

  /**
   * Suggested cross-bundle links to the lamad mount. Sanctioned plain-href
   * targets (elohim-app gospel; the capture-phase interceptor handles them) —
   * fixed mount paths, not minted content links.
   */
  readonly lamadLinks = {
    home: '/lamad', // route-literal-ok: cross-bundle mount link, not a minted content link
    explore: '/lamad/explore', // route-literal-ok: cross-bundle mount link, not a minted content link
    search: '/lamad/search', // route-literal-ok: cross-bundle mount link, not a minted content link
  };

  ngOnInit(): void {
    this.attemptedUrl = this.router.url;

    // Set SEO with noindex to prevent search engines from indexing 404 pages
    this.seoService.updateSeo({
      title: 'Page Not Found',
      description: 'The page you are looking for could not be found.',
      noIndex: true,
      openGraph: {
        ogType: 'website',
      },
    });
  }

  /**
   * Navigate to home page
   */
  goHome(): void {
    void this.router.navigate(['/']);
  }

  /**
   * Navigate to Lamad learning platform
   */
  goToLamad(): void {
    this.eprNav.navigate('/lamad'); // route-literal-ok: sanctioned EprNavService cross-bundle mount nav (elohim-app gospel), not a minted content link
  }

  /**
   * Go back to previous page
   */
  goBack(): void {
    globalThis.history.back();
  }
}
