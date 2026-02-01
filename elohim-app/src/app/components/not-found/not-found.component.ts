import { CommonModule } from '@angular/common';
import { Component, OnInit, inject } from '@angular/core';
import { Router, RouterModule } from '@angular/router';

// @coverage: 100.0% (2026-01-31)

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
})
export class NotFoundComponent implements OnInit {
  private readonly seoService = inject(SeoService);
  private readonly router = inject(Router);

  /** The attempted URL path */
  attemptedUrl = '';

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
    void this.router.navigate(['/lamad']);
  }

  /**
   * Go back to previous page
   */
  goBack(): void {
    window.history.back();
  }
}
