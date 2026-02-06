/**
 * Open Graph Metadata - Platform-Agnostic Social Sharing
 *
 * Provides standard Open Graph protocol fields that can be composed
 * into any model that needs to be shareable on social media.
 *
 * Philosophy:
 * - Platform-agnostic: No Twitter, Facebook, or other platform-specific fields
 * - Standard compliance: Follows Open Graph protocol specification
 * - Accessibility: Includes alt text and proper semantic markup
 * - SEO-friendly: Keywords and proper metadata
 *
 * Usage:
 * ```typescript
 * interface MyShareableModel extends OpenGraphMetadata {
 *   // ... your model fields
 * }
 * ```
 *
 * Or composition:
 * ```typescript
 * interface MyModel {
 *   // ... your fields
 *   socialMetadata: OpenGraphMetadata;
 * }
 * ```
 *
 * Resources:
 * - https://ogp.me/
 * - https://developers.facebook.com/docs/sharing/webmasters/
 */

/**
 * OpenGraphMetadata - Standard social graph metadata
 *
 * Maps to Open Graph protocol tags:
 * - og:title
 * - og:description
 * - og:image
 * - og:url
 * - og:type
 * - og:locale
 * - article:published_time (for article types)
 * - article:modified_time (for article types)
 * - article:section (for article types)
 * - profile:username (for profile types)
 */
export interface OpenGraphMetadata {
  // =========================================================================
  // Core Open Graph Fields (og:*)
  // =========================================================================

  /**
   * The title of the content as it should appear in social shares.
   * Maps to: og:title
   *
   * Best practices:
   * - 60-90 characters optimal
   * - Clear and descriptive
   * - No clickbait
   */
  ogTitle?: string;

  /**
   * A brief description of the content (1-2 sentences).
   * Maps to: og:description
   *
   * Best practices:
   * - 150-200 characters optimal
   * - Compelling but accurate
   * - Complements the title
   */
  ogDescription?: string;

  /**
   * URL to the preview image for social sharing.
   * Maps to: og:image
   *
   * Best practices:
   * - Minimum 1200x630px (recommended)
   * - JPG or PNG format
   * - Under 8MB file size
   * - Aspect ratio 1.91:1
   */
  ogImage?: string;

  /**
   * Alt text for the preview image (accessibility).
   * Maps to: og:image:alt
   *
   * Required for accessibility compliance.
   */
  ogImageAlt?: string;

  /**
   * Canonical URL - the authoritative location of this content.
   * Maps to: og:url
   *
   * Should be the permanent, shareable link.
   */
  ogUrl?: string;

  /**
   * The type of content being shared.
   * Maps to: og:type
   *
   * Common values:
   * - 'website' (default)
   * - 'article'
   * - 'profile'
   * - 'book'
   * - 'video.other'
   *
   * See: https://ogp.me/#types
   */
  ogType?: OpenGraphType;

  /**
   * Content locale/language.
   * Maps to: og:locale
   *
   * Format: language_TERRITORY (e.g., 'en_US', 'es_ES', 'fr_CA')
   * See: https://en.wikipedia.org/wiki/List_of_ISO_639-1_codes
   */
  ogLocale?: string;

  /**
   * Alternate locales this content is available in.
   * Maps to: og:locale:alternate
   */
  ogLocaleAlternate?: string[];

  /**
   * Site name (typically constant across the site).
   * Maps to: og:site_name
   *
   * Example: "Elohim Protocol" or "Lamad"
   */
  ogSiteName?: string;

  // =========================================================================
  // Article-Specific Fields (article:*)
  // =========================================================================

  /**
   * When the article was first published.
   * Maps to: article:published_time
   *
   * Format: ISO 8601 (e.g., '2024-01-15T10:30:00Z')
   */
  articlePublishedTime?: string;

  /**
   * When the article was last modified.
   * Maps to: article:modified_time
   *
   * Format: ISO 8601
   */
  articleModifiedTime?: string;

  /**
   * High-level section/category.
   * Maps to: article:section
   *
   * Examples: 'Technology', 'Learning', 'Governance'
   */
  articleSection?: string;

  /**
   * Keywords/tags for the article.
   * Maps to: article:tag (can be multiple)
   */
  articleTags?: string[];

  /**
   * Authors of the article.
   * Maps to: article:author (can be multiple profile URLs)
   */
  articleAuthors?: string[];

  // =========================================================================
  // Profile-Specific Fields (profile:*)
  // =========================================================================

  /**
   * Profile username or handle.
   * Maps to: profile:username
   *
   * Not platform-specific - just a display name/handle.
   */
  profileUsername?: string;

  /**
   * First name of the person.
   * Maps to: profile:first_name
   */
  profileFirstName?: string;

  /**
   * Last name of the person.
   * Maps to: profile:last_name
   */
  profileLastName?: string;

  // =========================================================================
  // SEO & Discovery
  // =========================================================================

  /**
   * Keywords for search engine optimization.
   * Not part of OG protocol, but complementary metadata.
   */
  seoKeywords?: string[];

  /**
   * Canonical URL for SEO (may differ from ogUrl in some cases).
   * Maps to: <link rel="canonical">
   */
  canonicalUrl?: string;
}

/**
 * OpenGraphType - Standard Open Graph content types
 *
 * See: https://ogp.me/#types
 */
export type OpenGraphType =
  // Basic types
  | 'website' // Default for most content
  | 'article' // Blog posts, news articles, learning content
  | 'book' // Books, chapters
  | 'profile' // Person or organization profile

  // Media types
  | 'video.movie' // Feature film
  | 'video.episode' // TV episode
  | 'video.tv_show' // TV series
  | 'video.other' // Other video content
  | 'music.song' // Individual song
  | 'music.album' // Music album
  | 'music.playlist' // Playlist
  | 'music.radio_station'; // Radio station

/**
 * OpenGraphImage - Detailed image metadata
 *
 * For cases where you need more control over image metadata.
 */
export interface OpenGraphImage {
  /** Image URL */
  url: string;

  /** Secure URL (HTTPS) - recommended */
  secureUrl?: string;

  /** MIME type (e.g., 'image/jpeg', 'image/png') */
  type?: string;

  /** Image width in pixels */
  width?: number;

  /** Image height in pixels */
  height?: number;

  /** Alt text for accessibility */
  alt?: string;
}

/**
 * Helper to extract basic OG metadata from various model types.
 *
 * This allows services to generate OG tags without knowing
 * the specific model structure.
 */
export interface OpenGraphExtractor<T> {
  /**
   * Extract OG metadata from a model instance.
   */
  extract(model: T): OpenGraphMetadata;
}

/**
 * Factory for creating OpenGraphMetadata from common fields.
 *
 * Usage:
 * ```typescript
 * const ogData = createOpenGraphMetadata({
 *   title: 'My Learning Path',
 *   description: 'A journey through...',
 *   imageUrl: 'https://...',
 *   url: 'https://...',
 *   type: 'article'
 * });
 * ```
 */
export function createOpenGraphMetadata(params: {
  title: string;
  description: string;
  imageUrl?: string;
  imageAlt?: string;
  url?: string;
  type?: OpenGraphType;
  locale?: string;
  publishedTime?: string;
  modifiedTime?: string;
  section?: string;
  tags?: string[];
  siteName?: string;
}): OpenGraphMetadata {
  return {
    ogTitle: params.title,
    ogDescription: params.description,
    ogImage: params.imageUrl,
    ogImageAlt: params.imageAlt,
    ogUrl: params.url,
    ogType: params.type ?? 'website',
    ogLocale: params.locale,
    ogSiteName: params.siteName ?? 'Elohim Protocol',
    articlePublishedTime: params.publishedTime,
    articleModifiedTime: params.modifiedTime,
    articleSection: params.section,
    articleTags: params.tags,
  };
}

/**
 * Validate that required OG fields are present for proper sharing.
 *
 * Returns array of missing required fields.
 */
export function validateOpenGraphMetadata(og: OpenGraphMetadata): string[] {
  const missing: string[] = [];

  // Required fields for all types
  if (!og.ogTitle) missing.push('ogTitle');
  if (!og.ogDescription) missing.push('ogDescription');
  if (!og.ogImage) missing.push('ogImage');
  if (!og.ogUrl) missing.push('ogUrl');

  // Image accessibility
  if (og.ogImage && !og.ogImageAlt) {
    missing.push('ogImageAlt (required for accessibility when ogImage present)');
  }

  return missing;
}
