import { CommonModule } from '@angular/common';
import { Component, OnInit, inject } from '@angular/core';
import { Router, RouterModule } from '@angular/router';

// @coverage: 100.0% (2026-02-04)

import { SeoService } from '../../../services/seo.service';

/**
 * LamadNotFoundComponent - 404 Page for Lamad routes
 *
 * Displays a learning-focused 404 error page with:
 * - Educational metaphor messaging
 * - Lamad-specific navigation options
 * - Consistent Lamad branding
 * - Proper SEO (noindex)
 */
@Component({
  selector: 'app-lamad-not-found',
  standalone: true,
  imports: [CommonModule, RouterModule],
  templateUrl: './lamad-not-found.component.html',
  styleUrl: './lamad-not-found.component.css',
})
export class LamadNotFoundComponent implements OnInit {
  private readonly seoService = inject(SeoService);
  private readonly router = inject(Router);

  /** The attempted URL path */
  attemptedUrl = '';

  /** Determine what kind of resource was likely being sought */
  resourceType: 'path' | 'resource' | 'unknown' = 'unknown';

  ngOnInit(): void {
    this.attemptedUrl = this.router.url;
    this.detectResourceType();

    // Set SEO with noindex
    this.seoService.updateSeo({
      title: 'Content Not Found - Lamad',
      description: 'The learning content you are looking for could not be found.',
      noIndex: true,
      openGraph: {
        ogType: 'website',
      },
    });
  }

  private detectResourceType(): void {
    if (this.attemptedUrl.includes('/path/')) {
      this.resourceType = 'path';
    } else if (
      this.attemptedUrl.includes('/resource/') ||
      this.attemptedUrl.includes('/content/')
    ) {
      this.resourceType = 'resource';
    }
  }

  /**
   * Get contextual message based on what was being sought
   */
  getMessage(): string {
    switch (this.resourceType) {
      case 'path':
        return "This learning path seems to have been moved or doesn't exist yet.";
      case 'resource':
        return "This content resource couldn't be found in our knowledge base.";
      default:
        return "The page you're looking for isn't part of our curriculum yet.";
    }
  }

  /**
   * Navigate to Lamad home
   */
  goToLamadHome(): void {
    void this.router.navigate(['/lamad']);
  }

  /**
   * Navigate to search
   */
  goToSearch(): void {
    void this.router.navigate(['/lamad/search']);
  }

  /**
   * Navigate to explorer
   */
  goToExplore(): void {
    void this.router.navigate(['/lamad/explore']);
  }

  /**
   * Go back to previous page
   */
  goBack(): void {
    window.history.back();
  }
}
