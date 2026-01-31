/**
 * Content Icon Utilities
 *
 * Maps content types and formats to visual icons for consistent display
 * across path overview, navigator, and other views.
 */

import { ContentType, ContentFormat } from '../models/content-node.model';

// @coverage: 16.7% (2026-01-31)

/**
 * Icon mappings for content types.
 * These represent the semantic category of content.
 */
const CONTENT_TYPE_ICONS: Record<ContentType, string> = {
  epic: '📖',
  feature: '⚡',
  scenario: '✓',
  concept: '💡',
  simulation: '🎮',
  video: '🎬',
  assessment: '📝',
  'discovery-assessment': '🔮',
  organization: '🏢',
  'book-chapter': '📚',
  tool: '🛠️',
  role: '👤',
  path: '🛤️',
  placeholder: '⚠️',
};

/**
 * Icon mappings for content formats.
 * These represent how content is rendered/consumed.
 */
const CONTENT_FORMAT_ICONS: Record<ContentFormat, string> = {
  markdown: '📄',
  'html5-app': '🎮',
  'video-embed': '🎬',
  'video-file': '🎬',
  'audio-file': '🎧',
  'perseus-quiz-json': '📝',
  'sophia-quiz-json': '📝',
  'external-link': '🔗',
  epub: '📚',
  gherkin: '✓',
  html: '📄',
  plaintext: '📄',
};

/**
 * Icon mappings for step types.
 * Used when content type is not available (metadata-only views).
 */
const STEP_TYPE_ICONS: Record<string, string> = {
  content: '📄',
  read: '📖',
  assessment: '📝',
  quiz: '📝',
  checkpoint: '🏁',
  path: '🛤️',
  external: '🔗',
  video: '🎬',
  simulation: '🎮',
};

/**
 * Default icon when type is unknown.
 */
const DEFAULT_ICON = '📄';

/**
 * Get icon for a content type.
 *
 * @param contentType - The semantic type of content
 * @returns Emoji icon representing the content type
 */
export function getContentTypeIcon(contentType: ContentType | string | undefined): string {
  if (!contentType) return DEFAULT_ICON;
  return CONTENT_TYPE_ICONS[contentType as ContentType] ?? DEFAULT_ICON;
}

/**
 * Get icon for a content format.
 *
 * @param contentFormat - How the content is rendered
 * @returns Emoji icon representing the format
 */
export function getContentFormatIcon(contentFormat: ContentFormat | string | undefined): string {
  if (!contentFormat) return DEFAULT_ICON;
  return CONTENT_FORMAT_ICONS[contentFormat as ContentFormat] ?? DEFAULT_ICON;
}

/**
 * Get icon for a step type.
 *
 * @param stepType - The type of step in a learning path
 * @returns Emoji icon representing the step type
 */
export function getStepTypeIcon(stepType: string | undefined): string {
  if (!stepType) return DEFAULT_ICON;
  return STEP_TYPE_ICONS[stepType] ?? DEFAULT_ICON;
}

/**
 * Get the best icon for content, preferring type over format.
 *
 * Priority:
 * 1. Content type (semantic category) - most meaningful
 * 2. Content format (rendering format) - fallback
 * 3. Default icon
 *
 * @param contentType - The semantic type of content
 * @param contentFormat - How the content is rendered
 * @returns Emoji icon representing the content
 */
export function getContentIcon(
  contentType?: ContentType | string,
  contentFormat?: ContentFormat | string
): string {
  // Content type takes priority as it's more semantically meaningful
  if (contentType && contentType !== 'concept') {
    const typeIcon = CONTENT_TYPE_ICONS[contentType as ContentType];
    if (typeIcon) return typeIcon;
  }

  // Fall back to format-based icon
  if (contentFormat) {
    const formatIcon = CONTENT_FORMAT_ICONS[contentFormat as ContentFormat];
    if (formatIcon) return formatIcon;
  }

  // If we have a generic 'concept' type, use default
  if (contentType === 'concept') {
    return CONTENT_TYPE_ICONS['concept'];
  }

  return DEFAULT_ICON;
}

/**
 * Infer content type from content ID patterns (fallback when type is not available).
 *
 * This is less reliable than using actual content type, but useful for
 * metadata-only views where content isn't loaded.
 *
 * @param contentId - The content node ID
 * @returns Inferred content type or 'concept' as default
 */
export function inferContentTypeFromId(contentId: string): ContentType {
  const id = contentId.toLowerCase();

  if (id.includes('quiz') || id.includes('assessment')) return 'assessment';
  if (id.includes('discovery-assessment')) return 'discovery-assessment';
  if (id.includes('video')) return 'video';
  if (id.includes('simulation') || id.includes('app-')) return 'simulation';
  if (id.includes('scenario')) return 'scenario';
  if (id.includes('feature')) return 'feature';
  if (id.includes('epic')) return 'epic';
  if (id.includes('book-chapter') || id.includes('chapter')) return 'book-chapter';
  if (id.includes('tool')) return 'tool';
  if (id.includes('organization') || id.includes('org-')) return 'organization';
  if (id.includes('path-')) return 'path';

  return 'concept';
}

/**
 * Get icon for content by ID (using inference as fallback).
 *
 * Use this when you only have the content ID and no type information.
 *
 * @param contentId - The content node ID
 * @param contentType - Optional actual content type (preferred if available)
 * @param contentFormat - Optional content format (used as fallback)
 * @returns Emoji icon representing the content
 */
export function getIconForContent(
  contentId: string,
  contentType?: ContentType | string,
  contentFormat?: ContentFormat | string
): string {
  // If we have actual type info, use it
  if (contentType) {
    return getContentIcon(contentType, contentFormat);
  }

  // Otherwise, infer from ID
  const inferredType = inferContentTypeFromId(contentId);
  return getContentTypeIcon(inferredType);
}
