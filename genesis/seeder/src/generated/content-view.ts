/* Generated from protocol schema: views/content-view.schema.json -- DO NOT EDIT */

/**
 * Content type vocabulary. Core types are DNA-notarized (three-leg coupled: knowledge+value+governance). Storage-only types are cross-domain entity references. Extensible types are community vocabulary.
 */
export type ContentType =
  | 'epic'
  | 'concept'
  | 'lesson'
  | 'scenario'
  | 'assessment'
  | 'reflection'
  | 'discussion'
  | 'exercise'
  | 'article'
  | 'path'
  | 'human'
  | 'role'
  | 'collective'
  | 'example'
  | 'reference'
  | 'feature'
  | 'practice'
  | 'contributor'
  | 'video'
  | 'audio'
  | 'book'
  | 'book-chapter'
  | 'documentary'
  | 'bible-verse'
  | 'activity'
  | 'narrative'
  | 'course-module'
  | 'module'
  | 'quiz'
  | 'podcast'
  | 'simulation'
  | 'node-context'
  | 'stewardship-context'
  | 'work-story'
  | 'work-project'
  | 'issue-report';
/**
 * Content format for rendering. Core formats are DNA-notarized. Extended formats are storage-level rendering hints.
 */
export type ContentFormat =
  | 'markdown'
  | 'html'
  | 'video'
  | 'audio'
  | 'interactive'
  | 'external'
  | 'epr-composite'
  | 'plaintext'
  | 'text'
  | 'plain'
  | 'gherkin'
  | 'perseus'
  | 'perseus-json'
  | 'perseus-quiz-json'
  | 'video-embed'
  | 'audio-file'
  | 'html5-app'
  | 'human-json'
  | 'organization-json'
  | 'json'
  | 'sophia'
  | 'sophia-quiz-json';
/**
 * Content reach/visibility level. Ordered from most restrictive to most open.
 */
export type Reach =
  | 'private'
  | 'self'
  | 'intimate'
  | 'trusted'
  | 'familiar'
  | 'community'
  | 'public'
  | 'commons';
/**
 * Schema migration status for records. Defined in views.rs.
 */
export type ValidationStatus = 'valid' | 'migrated' | 'degraded' | 'healing';

/**
 * Content record as returned by the storage API. Source of truth: DHT (Notarized, Category A).
 */
export interface ContentView {
  id: string;
  appId: string;
  title: string;
  description?: string | null;
  contentType: ContentType;
  contentFormat: ContentFormat;
  blobHash?: string | null;
  blobCid?: string | null;
  contentSizeBytes?: number | null;
  metadata?: unknown;
  reach: Reach;
  validationStatus: ValidationStatus;
  createdBy?: string | null;
  createdAt: string;
  updatedAt: string;
  contentBody?: string | null;
  dhtAnchorHash?: string | null;
}
