/**
 * Metadata describing a content format supported by a plugin.
 *
 * NOTE: Rendering is handled separately by the existing RendererRegistryService.
 * This interface focuses on I/O-related metadata only.
 */
export interface FormatMetadata {
  /** Unique identifier for the format (e.g., 'markdown', 'gherkin') */
  formatId: string;

  /** Human-readable name (e.g., 'Markdown') */
  displayName: string;

  /** Description of the format */
  description: string;

  /** File extensions this format handles (e.g., ['.md', '.markdown']) */
  fileExtensions: string[];

  /** MIME types this format handles */
  mimeTypes: string[];

  /** Material icon name for UI */
  icon?: string;

  /** Category for grouping in UI */
  category: 'document' | 'data' | 'book' | 'code' | 'media';

  /** Whether export → import produces equivalent content */
  supportsRoundTrip: boolean;

  /** Priority for format detection (higher = checked first) */
  priority?: number;
}
