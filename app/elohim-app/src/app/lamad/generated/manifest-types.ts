// AUTO-GENERATED from app manifest: elohim/sdk/domains/lamad/manifest.json
// DO NOT EDIT — regenerate with: pnpm run lamad:codegen

export const LAMAD_CONTENT_TYPES = [
  'concept',
  'lesson',
  'assessment',
  'exercise',
  'reflection',
  'discussion',
  'article',
  'path',
  'epic',
  'scenario',
  'discovery-assessment',
  'instrument',
  'quiz',
  'course-module',
  'module',
  'community',
  'tool',
  'placeholder',
  'simulation',
  'feature',
  'practice',
] as const;
export type LamadContentType = (typeof LAMAD_CONTENT_TYPES)[number];

export const LAMAD_CONTENT_FORMATS = [
  'markdown',
  'gherkin',
  'sophia-quiz-json',
  'html5-app',
  'epr-composite',
  'html',
  'plaintext',
  'video-embed',
  'json',
] as const;
export type LamadContentFormat = (typeof LAMAD_CONTENT_FORMATS)[number];

export const LAMAD_RELATIONSHIPS = [
  'CONTAINS',
  'BELONGS_TO',
  'DESCRIBES',
  'IMPLEMENTS',
  'VALIDATES',
  'RELATES_TO',
  'REFERENCES',
  'DEPENDS_ON',
  'REQUIRES',
  'FOLLOWS',
  'ATTACHED_TO',
] as const;
export type LamadRelationship = (typeof LAMAD_RELATIONSHIPS)[number];

export const LAMAD_SIGNALS = [
  'learning-signal',
  'mastery-achieved',
  'assessment-completed',
  'path-completed',
  'practice-engagement',
  'contribution-created',
  'peer-review-completed',
] as const;
export type LamadSignal = (typeof LAMAD_SIGNALS)[number];

export const LAMAD_RENDERER_MAP: Record<string, string> = {
  'markdown': 'MarkdownRendererComponent',
  'html': 'MarkdownRendererComponent',
  'plaintext': 'MarkdownRendererComponent',
  'gherkin': 'GherkinRendererComponent',
  'sophia-quiz-json': 'SophiaRendererComponent',
  'sophia': 'SophiaRendererComponent',
  'perseus-quiz-json': 'SophiaRendererComponent',
  'html5-app': 'IframeRendererComponent',
  'video-embed': 'IframeRendererComponent',
  'epr-composite': 'PathViewerComponent',
};
