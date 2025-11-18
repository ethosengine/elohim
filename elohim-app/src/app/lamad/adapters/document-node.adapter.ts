import { ContentNode, ContentFormat } from '../models/content-node.model';
import { DocumentNode, NodeType } from '../models/document-node.model';
import { EpicNode } from '../models/epic-node.model';
import { FeatureNode } from '../models/feature-node.model';
import { ScenarioNode } from '../models/scenario-node.model';

/**
 * Adapter to convert between legacy DocumentNode types and new ContentNode model
 */
export class DocumentNodeAdapter {
  /**
   * Convert any DocumentNode type to ContentNode
   */
  static fromDocumentNode(node: DocumentNode): ContentNode {
    const baseNode: ContentNode = {
      id: node.id,
      contentType: this.mapNodeTypeToContentType(node.type),
      title: node.title,
      description: node.description,
      content: node.content,
      contentFormat: this.determineContentFormat(node),
      tags: node.tags || [],
      sourcePath: node.sourcePath,
      relatedNodeIds: node.relatedNodeIds || [],
      metadata: { ...node.metadata },
    };

    // Add type-specific metadata
    switch (node.type) {
      case NodeType.EPIC:
        return this.fromEpicNode(node as EpicNode, baseNode);
      case NodeType.FEATURE:
        return this.fromFeatureNode(node as FeatureNode, baseNode);
      case NodeType.SCENARIO:
        return this.fromScenarioNode(node as ScenarioNode, baseNode);
      default:
        return baseNode;
    }
  }

  /**
   * Convert ContentNode back to DocumentNode (for backward compatibility)
   */
  static toDocumentNode(node: ContentNode): DocumentNode {
    const nodeType = this.mapContentTypeToNodeType(node.contentType);

    const baseNode: DocumentNode = {
      id: node.id,
      type: nodeType,
      title: node.title,
      description: node.description,
      tags: node.tags,
      sourcePath: node.sourcePath ?? '',
      content: node.content,
      relatedNodeIds: node.relatedNodeIds,
      metadata: node.metadata,
    };

    return baseNode;
  }

  /**
   * Convert EpicNode to ContentNode
   */
  private static fromEpicNode(
    epic: EpicNode,
    base: ContentNode
  ): ContentNode {
    return {
      ...base,
      content: epic.markdownContent || epic.content,
      contentFormat: 'markdown',
      metadata: {
        ...base.metadata,
        authors: epic.authors,
        version: epic.version,
        category: epic.category,
        featureIds: epic.featureIds,
        relatedEpicIds: epic.relatedEpicIds,
        sections: epic.sections,
      },
      relatedNodeIds: [
        ...base.relatedNodeIds,
        ...(epic.featureIds || []),
        ...(epic.relatedEpicIds || []),
      ],
    };
  }

  /**
   * Convert FeatureNode to ContentNode
   */
  private static fromFeatureNode(
    feature: FeatureNode,
    base: ContentNode
  ): ContentNode {
    return {
      ...base,
      content: feature.gherkinContent || feature.content,
      contentFormat: 'gherkin',
      metadata: {
        ...base.metadata,
        category: feature.category,
        epicIds: feature.epicIds,
        scenarioIds: feature.scenarioIds,
        featureDescription: feature.featureDescription,
        background: feature.background,
        testStatus: feature.testStatus,
      },
      relatedNodeIds: [
        ...base.relatedNodeIds,
        ...(feature.epicIds || []),
        ...(feature.scenarioIds || []),
      ],
    };
  }

  /**
   * Convert ScenarioNode to ContentNode
   */
  private static fromScenarioNode(
    scenario: ScenarioNode,
    base: ContentNode
  ): ContentNode {
    return {
      ...base,
      contentFormat: 'gherkin',
      metadata: {
        ...base.metadata,
        featureId: scenario.featureId,
        epicIds: scenario.epicIds,
        scenarioType: scenario.scenarioType,
        steps: scenario.steps,
        examples: scenario.examples,
        testStatus: scenario.testStatus,
        stepResults: scenario.stepResults,
      },
      relatedNodeIds: [
        ...base.relatedNodeIds,
        scenario.featureId,
        ...(scenario.epicIds || []),
      ],
    };
  }

  /**
   * Map NodeType to contentType string
   */
  private static mapNodeTypeToContentType(nodeType: NodeType): string {
    switch (nodeType) {
      case NodeType.EPIC:
        return 'epic';
      case NodeType.FEATURE:
        return 'feature';
      case NodeType.SCENARIO:
        return 'scenario';
      default:
        return 'unknown';
    }
  }

  /**
   * Map contentType string back to NodeType
   */
  private static mapContentTypeToNodeType(contentType: string): NodeType {
    switch (contentType.toLowerCase()) {
      case 'epic':
        return NodeType.EPIC;
      case 'feature':
        return NodeType.FEATURE;
      case 'scenario':
        return NodeType.SCENARIO;
      default:
        return NodeType.EPIC; // Default fallback
    }
  }

  /**
   * Determine content format from node type
   */
  private static determineContentFormat(node: DocumentNode): ContentFormat {
    if ('markdownContent' in node) {
      return 'markdown';
    }
    if ('gherkinContent' in node || node.type === NodeType.FEATURE || node.type === NodeType.SCENARIO) {
      return 'gherkin';
    }
    return 'plaintext';
  }

  /**
   * Batch convert array of DocumentNodes to ContentNodes
   */
  static fromDocumentNodes(nodes: DocumentNode[]): ContentNode[] {
    return nodes.map((node) => this.fromDocumentNode(node));
  }

  /**
   * Batch convert array of ContentNodes to DocumentNodes
   */
  static toDocumentNodes(nodes: ContentNode[]): DocumentNode[] {
    return nodes.map((node) => this.toDocumentNode(node));
  }
}
