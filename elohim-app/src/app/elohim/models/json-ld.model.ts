/**
 * JSON-LD Metadata - Minimal alignment with Linked Data standards
 *
 * This is NOT a full JSON-LD implementation - just the basic structure
 * to prevent tech debt when we need semantic web interoperability later.
 *
 * Composition pattern: Add as optional `linkedData?: JsonLdMetadata` field
 * to existing models (same approach as `socialMetadata?: OpenGraphMetadata`).
 *
 * Reference: https://www.w3.org/TR/json-ld11/
 */

/**
 * JSON-LD Metadata - Basic W3C JSON-LD structure
 *
 * Enables semantic web interoperability without changing existing models.
 * When populated, allows models to be serialized as Linked Data for:
 * - Schema.org structured data (SEO, rich snippets)
 * - RDF triple stores and SPARQL queries
 * - Decentralized knowledge graphs
 * - Interoperability with other semantic web systems
 *
 * Example usage:
 * ```typescript
 * const contentNode: ContentNode = {
 *   // ... existing fields ...
 *   linkedData: {
 *     '@context': 'https://schema.org/',
 *     '@type': 'Article',
 *     '@id': 'https://elohim-protocol.org/content/abc123'
 *   }
 * };
 * ```
 */
export interface JsonLdMetadata {
  /**
   * JSON-LD context (vocabulary mappings)
   *
   * Can be:
   * - String URL: 'https://schema.org/'
   * - Object: { '@vocab': 'https://schema.org/', 'lamad': 'https://...' }
   * - Array: ['https://schema.org/', { 'custom': '...' }]
   */
  '@context'?: string | Record<string, string> | (string | Record<string, string>)[];

  /**
   * Type from Schema.org or custom vocabulary
   *
   * Examples:
   * - 'Article' (single type)
   * - ['Course', 'LearningResource'] (multiple types)
   */
  '@type'?: string | string[];

  /**
   * Canonical identifier (IRI/URL)
   *
   * Unique identifier for this resource in Linked Data format.
   * Example: 'https://elohim-protocol.org/content/abc123'
   */
  '@id'?: string;

  /**
   * Additional Schema.org properties (extensible)
   *
   * Any Schema.org properties can be added here.
   * Common examples:
   * - name: string
   * - description: string
   * - dateCreated: string (ISO 8601)
   * - author: { '@type': 'Person', '@id': '...' }
   * - inLanguage: string (BCP 47 code)
   */
  [key: string]: any;
}
