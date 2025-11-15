import { DocumentNode, NodeType } from './document-node.model';

/**
 * Represents an epic document - high-level narrative documentation
 * Source: docs/*.md files
 */
export interface EpicNode extends DocumentNode {
  type: NodeType.EPIC;

  /** Author(s) of the epic */
  authors?: string[];

  /** Version number */
  version?: string;

  /** Epic category/theme */
  category?: string;

  /** Featured features implementing this epic */
  featureIds: string[];

  /** Related epic IDs */
  relatedEpicIds: string[];

  /** Markdown content with custom directives */
  markdownContent: string;

  /** Extracted sections/headings for quick navigation */
  sections: EpicSection[];
}

export interface EpicSection {
  /** Section heading */
  title: string;

  /** Depth level (h1=1, h2=2, etc.) */
  level: number;

  /** Anchor ID for navigation */
  anchor: string;

  /** Section content */
  content: string;

  /** Embedded feature/scenario references */
  embeddedReferences: EmbeddedReference[];
}

export interface EmbeddedReference {
  /** Type of reference */
  type: 'feature' | 'scenario';

  /** Referenced node ID */
  nodeId: string;

  /** Position in content */
  position: number;

  /** Display text */
  displayText: string;
}
