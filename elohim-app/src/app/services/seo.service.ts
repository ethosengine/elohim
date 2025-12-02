import { Injectable, inject } from '@angular/core';
import { Title, Meta } from '@angular/platform-browser';
import { Router, NavigationEnd, ActivatedRoute } from '@angular/router';
import { DOCUMENT } from '@angular/common';
import { filter, map, mergeMap } from 'rxjs/operators';
import { OpenGraphMetadata } from '../lamad/models/open-graph.model';
import { JsonLdMetadata } from '../lamad/models/json-ld.model';

/**
 * SEO configuration for a page
 */
export interface SeoConfig {
  /** Page title (will be appended with site name) */
  title: string;
  /** Meta description */
  description: string;
  /** Canonical URL (optional, auto-generated if not provided) */
  canonicalUrl?: string;
  /** Open Graph metadata */
  openGraph?: Partial<OpenGraphMetadata>;
  /** JSON-LD structured data */
  jsonLd?: JsonLdMetadata;
  /** Keywords for SEO */
  keywords?: string[];
  /** Prevent indexing (for private/draft content) */
  noIndex?: boolean;
}

/**
 * Default SEO values for the site
 */
const DEFAULTS = {
  siteName: 'Elohim Protocol',
  siteUrl: 'https://elohim.host',
  defaultDescription: 'Digital guardians for human flourishing. Technology organized around love.',
  defaultImage: 'https://elohim.host/images/elohim_logo_light.png',
  defaultImageAlt: 'Elohim Protocol Logo'
} as const;

/**
 * SeoService - Manages dynamic page titles, meta tags, and structured data
 *
 * Provides a centralized way to update SEO-related metadata for SPAs.
 * Handles:
 * - Document title updates
 * - Meta description and keywords
 * - Open Graph protocol tags
 * - JSON-LD structured data for rich snippets
 * - Canonical URLs
 *
 * Usage:
 * ```typescript
 * // In a component
 * this.seoService.updateSeo({
 *   title: 'My Page Title',
 *   description: 'Page description...',
 *   openGraph: { ogType: 'article' }
 * });
 *
 * // Or update just the title
 * this.seoService.setTitle('Page Title');
 * ```
 */
@Injectable({
  providedIn: 'root'
})
export class SeoService {
  private readonly titleService = inject(Title);
  private readonly meta = inject(Meta);
  private readonly router = inject(Router);
  private readonly activatedRoute = inject(ActivatedRoute);
  private readonly document = inject(DOCUMENT);

  private jsonLdScript: HTMLScriptElement | null = null;

  constructor() {
    this.initRouteListener();
  }

  /**
   * Listen for route changes and update title from route data
   */
  private initRouteListener(): void {
    this.router.events.pipe(
      filter(event => event instanceof NavigationEnd),
      map(() => this.activatedRoute),
      map(route => {
        // Traverse to the deepest activated route
        while (route.firstChild) {
          route = route.firstChild;
        }
        return route;
      }),
      filter(route => route.outlet === 'primary'),
      mergeMap(route => route.data)
    ).subscribe(data => {
      // If route has seo data, apply it
      if (data['seo']) {
        this.updateSeo(data['seo']);
      } else if (data['title']) {
        // Simple title from route data
        this.setTitle(data['title']);
        // Reset to defaults for other meta
        this.updateMetaDescription(DEFAULTS.defaultDescription);
        this.updateOpenGraphTags({
          ogTitle: data['title'],
          ogDescription: DEFAULTS.defaultDescription
        });
      }
    });
  }

  /**
   * Update all SEO metadata at once
   */
  updateSeo(config: SeoConfig): void {
    this.setTitle(config.title);
    this.updateMetaDescription(config.description);

    if (config.keywords?.length) {
      this.updateMetaKeywords(config.keywords);
    }

    if (config.noIndex) {
      this.setNoIndex();
    } else {
      this.removeNoIndex();
    }

    // Update canonical URL
    const canonicalUrl = config.canonicalUrl ?? this.generateCanonicalUrl();
    this.updateCanonicalUrl(canonicalUrl);

    // Update Open Graph tags
    this.updateOpenGraphTags({
      ogTitle: config.title,
      ogDescription: config.description,
      ogUrl: canonicalUrl,
      ...config.openGraph
    });

    // Update JSON-LD if provided
    if (config.jsonLd) {
      this.updateJsonLd(config.jsonLd);
    } else {
      this.removeJsonLd();
    }
  }

  /**
   * Set the page title (with site name suffix)
   */
  setTitle(title: string): void {
    const fullTitle = title ? `${title} | ${DEFAULTS.siteName}` : DEFAULTS.siteName;
    this.titleService.setTitle(fullTitle);
  }

  /**
   * Update meta description
   */
  updateMetaDescription(description: string): void {
    this.meta.updateTag({ name: 'description', content: description });
  }

  /**
   * Update meta keywords
   */
  updateMetaKeywords(keywords: string[]): void {
    this.meta.updateTag({ name: 'keywords', content: keywords.join(', ') });
  }

  /**
   * Update Open Graph tags
   */
  updateOpenGraphTags(og: Partial<OpenGraphMetadata>): void {
    // Core OG tags
    if (og.ogTitle) {
      this.meta.updateTag({ property: 'og:title', content: og.ogTitle });
    }
    if (og.ogDescription) {
      this.meta.updateTag({ property: 'og:description', content: og.ogDescription });
    }
    if (og.ogUrl) {
      this.meta.updateTag({ property: 'og:url', content: og.ogUrl });
    }

    // Image (use default if not provided)
    const imageUrl = og.ogImage ?? DEFAULTS.defaultImage;
    const imageAlt = og.ogImageAlt ?? DEFAULTS.defaultImageAlt;
    this.meta.updateTag({ property: 'og:image', content: imageUrl });
    this.meta.updateTag({ property: 'og:image:alt', content: imageAlt });

    // Type (default to website)
    this.meta.updateTag({ property: 'og:type', content: og.ogType ?? 'website' });

    // Site name
    this.meta.updateTag({ property: 'og:site_name', content: og.ogSiteName ?? DEFAULTS.siteName });

    // Locale
    if (og.ogLocale) {
      this.meta.updateTag({ property: 'og:locale', content: og.ogLocale });
    }

    // Article-specific tags
    if (og.ogType === 'article') {
      if (og.articlePublishedTime) {
        this.meta.updateTag({ property: 'article:published_time', content: og.articlePublishedTime });
      }
      if (og.articleModifiedTime) {
        this.meta.updateTag({ property: 'article:modified_time', content: og.articleModifiedTime });
      }
      if (og.articleSection) {
        this.meta.updateTag({ property: 'article:section', content: og.articleSection });
      }
      if (og.articleTags?.length) {
        // Remove existing article:tag entries
        this.removeMetaByProperty('article:tag');
        // Add new ones
        og.articleTags.forEach(tag => {
          this.meta.addTag({ property: 'article:tag', content: tag });
        });
      }
    }

    // Profile-specific tags
    if (og.ogType === 'profile') {
      if (og.profileUsername) {
        this.meta.updateTag({ property: 'profile:username', content: og.profileUsername });
      }
      if (og.profileFirstName) {
        this.meta.updateTag({ property: 'profile:first_name', content: og.profileFirstName });
      }
      if (og.profileLastName) {
        this.meta.updateTag({ property: 'profile:last_name', content: og.profileLastName });
      }
    }
  }

  /**
   * Update JSON-LD structured data
   */
  updateJsonLd(data: JsonLdMetadata): void {
    // Remove existing script if present
    this.removeJsonLd();

    // Create new script element
    this.jsonLdScript = this.document.createElement('script');
    this.jsonLdScript.type = 'application/ld+json';
    this.jsonLdScript.id = 'seo-json-ld';
    this.jsonLdScript.textContent = JSON.stringify(data);

    this.document.head.appendChild(this.jsonLdScript);
  }

  /**
   * Remove JSON-LD script
   */
  removeJsonLd(): void {
    if (this.jsonLdScript) {
      this.jsonLdScript.remove();
      this.jsonLdScript = null;
    } else {
      // Also check for any existing script by ID
      const existing = this.document.getElementById('seo-json-ld');
      if (existing) {
        existing.remove();
      }
    }
  }

  /**
   * Update canonical URL
   */
  updateCanonicalUrl(url: string): void {
    let link = this.document.querySelector('link[rel="canonical"]') as HTMLLinkElement;
    if (!link) {
      link = this.document.createElement('link');
      link.rel = 'canonical';
      this.document.head.appendChild(link);
    }
    link.href = url;
  }

  /**
   * Add noindex meta tag
   */
  setNoIndex(): void {
    this.meta.updateTag({ name: 'robots', content: 'noindex, nofollow' });
  }

  /**
   * Remove noindex meta tag
   */
  removeNoIndex(): void {
    this.meta.removeTag('name="robots"');
  }

  /**
   * Generate canonical URL from current route
   */
  private generateCanonicalUrl(): string {
    const path = this.router.url.split('?')[0]; // Remove query params
    return `${DEFAULTS.siteUrl}${path}`;
  }

  /**
   * Remove meta tags by property attribute
   */
  private removeMetaByProperty(property: string): void {
    const elements = this.document.querySelectorAll(`meta[property="${property}"]`);
    elements.forEach(el => el.remove());
  }

  /**
   * Reset to default SEO values
   */
  resetToDefaults(): void {
    this.setTitle('');
    this.updateMetaDescription(DEFAULTS.defaultDescription);
    this.updateOpenGraphTags({
      ogTitle: DEFAULTS.siteName,
      ogDescription: DEFAULTS.defaultDescription,
      ogImage: DEFAULTS.defaultImage,
      ogImageAlt: DEFAULTS.defaultImageAlt,
      ogType: 'website'
    });
    this.removeJsonLd();
  }

  // =========================================================================
  // Convenience methods for common content types
  // =========================================================================

  /**
   * Update SEO for a learning path
   */
  updateForPath(path: {
    id: string;
    title: string;
    description: string;
    thumbnailUrl?: string;
    difficulty?: string;
    estimatedDuration?: string;
  }): void {
    const canonicalUrl = `${DEFAULTS.siteUrl}/lamad/path/${path.id}`;

    this.updateSeo({
      title: path.title,
      description: path.description,
      canonicalUrl,
      openGraph: {
        ogType: 'article',
        ogImage: path.thumbnailUrl ?? DEFAULTS.defaultImage,
        ogImageAlt: `${path.title} - Learning Path`,
        articleSection: 'Learning'
      },
      jsonLd: {
        '@context': 'https://schema.org',
        '@type': 'Course',
        '@id': canonicalUrl,
        name: path.title,
        description: path.description,
        provider: {
          '@type': 'Organization',
          name: DEFAULTS.siteName,
          url: DEFAULTS.siteUrl
        },
        ...(path.difficulty && { educationalLevel: path.difficulty }),
        ...(path.estimatedDuration && { timeRequired: path.estimatedDuration })
      }
    });
  }

  /**
   * Update SEO for a content resource
   */
  updateForContent(content: {
    id: string;
    title: string;
    summary?: string;
    contentType: string;
    thumbnailUrl?: string;
    authors?: string[];
    createdAt?: string;
    updatedAt?: string;
  }): void {
    const canonicalUrl = `${DEFAULTS.siteUrl}/lamad/resource/${content.id}`;
    const description = content.summary ?? `${content.title} - ${content.contentType} content`;

    this.updateSeo({
      title: content.title,
      description,
      canonicalUrl,
      openGraph: {
        ogType: 'article',
        ogImage: content.thumbnailUrl ?? DEFAULTS.defaultImage,
        ogImageAlt: content.title,
        articleSection: content.contentType,
        articlePublishedTime: content.createdAt,
        articleModifiedTime: content.updatedAt
      },
      jsonLd: {
        '@context': 'https://schema.org',
        '@type': this.mapContentTypeToSchemaType(content.contentType),
        '@id': canonicalUrl,
        name: content.title,
        description,
        ...(content.authors?.length && {
          author: content.authors.map(name => ({ '@type': 'Person', name }))
        }),
        ...(content.createdAt && { dateCreated: content.createdAt }),
        ...(content.updatedAt && { dateModified: content.updatedAt })
      }
    });
  }

  /**
   * Update SEO for a profile page
   */
  updateForProfile(profile: {
    username: string;
    displayName?: string;
    bio?: string;
    avatarUrl?: string;
  }): void {
    const canonicalUrl = `${DEFAULTS.siteUrl}/lamad/human`;
    const title = profile.displayName ?? profile.username;
    const description = profile.bio ?? `${title}'s profile on ${DEFAULTS.siteName}`;

    this.updateSeo({
      title,
      description,
      canonicalUrl,
      openGraph: {
        ogType: 'profile',
        ogImage: profile.avatarUrl ?? DEFAULTS.defaultImage,
        ogImageAlt: `${title}'s avatar`,
        profileUsername: profile.username,
      },
      jsonLd: {
        '@context': 'https://schema.org',
        '@type': 'Person',
        '@id': canonicalUrl,
        name: title,
        ...(profile.bio && { description: profile.bio })
      }
    });
  }

  /**
   * Map content type to Schema.org type
   */
  private mapContentTypeToSchemaType(contentType: string): string {
    const mapping: Record<string, string> = {
      'epic': 'Article',
      'feature': 'Article',
      'scenario': 'HowTo',
      'concept': 'DefinedTerm',
      'video': 'VideoObject',
      'assessment': 'Quiz',
      'simulation': 'Game',
      'book-chapter': 'Chapter',
      'tool': 'SoftwareApplication',
      'organization': 'Organization'
    };
    return mapping[contentType] ?? 'Article';
  }
}
