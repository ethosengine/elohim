import { DocumentNodeAdapter } from './document-node.adapter';
import { DocumentNode, NodeType } from '../models/document-node.model';
import { EpicNode } from '../models/epic-node.model';
import { FeatureNode } from '../models/feature-node.model';
import { ScenarioNode } from '../models/scenario-node.model';
import { ContentNode } from '../models/content-node.model';

describe('DocumentNodeAdapter', () => {
  describe('fromDocumentNode', () => {
    it('should convert basic DocumentNode to ContentNode', () => {
      const docNode: DocumentNode = {
        id: 'node-1',
        type: NodeType.EPIC,
        title: 'Test Node',
        description: 'Test description',
        tags: ['test'],
        sourcePath: '/test.md',
        content: 'Test content',
        relatedNodeIds: ['node-2'],
        metadata: { custom: 'value' }
      };

      const contentNode = DocumentNodeAdapter.fromDocumentNode(docNode);

      expect(contentNode.id).toBe('node-1');
      expect(contentNode.contentType).toBe('epic');
      expect(contentNode.title).toBe('Test Node');
      expect(contentNode.tags).toContain('test');
      expect(contentNode.relatedNodeIds).toContain('node-2');
    });

    it('should convert EpicNode to ContentNode', () => {
      const epicNode: EpicNode = {
        id: 'epic-1',
        type: NodeType.EPIC,
        title: 'Test Epic',
        description: 'Epic description',
        tags: ['epic'],
        sourcePath: '/docs/epic.md',
        content: 'Content',
        relatedNodeIds: [],
        featureIds: ['feature-1', 'feature-2'],
        relatedEpicIds: ['epic-2'],
        markdownContent: '# Test Epic',
        sections: [],
        authors: ['Author 1'],
        version: '1.0',
        category: 'Architecture',
        metadata: {}
      };

      const contentNode = DocumentNodeAdapter.fromDocumentNode(epicNode);

      expect(contentNode.contentType).toBe('epic');
      expect(contentNode.contentFormat).toBe('markdown');
      expect(contentNode.content).toBe('# Test Epic');
      expect(contentNode.metadata['authors']).toContain('Author 1');
      expect(contentNode.metadata['version']).toBe('1.0');
      expect(contentNode.metadata['category']).toBe('Architecture');
      expect(contentNode.relatedNodeIds).toContain('feature-1');
      expect(contentNode.relatedNodeIds).toContain('epic-2');
    });

    it('should convert FeatureNode to ContentNode', () => {
      const featureNode: FeatureNode = {
        id: 'feature-1',
        type: NodeType.FEATURE,
        title: 'Login Feature',
        description: 'User login',
        tags: ['auth'],
        sourcePath: '/features/login.feature',
        content: 'Feature content',
        relatedNodeIds: [],
        category: 'authentication',
        epicIds: ['epic-1'],
        scenarioIds: ['scenario-1'],
        featureDescription: 'Login feature',
        gherkinContent: 'Feature: Login',
        metadata: {}
      };

      const contentNode = DocumentNodeAdapter.fromDocumentNode(featureNode);

      expect(contentNode.contentType).toBe('feature');
      expect(contentNode.contentFormat).toBe('gherkin');
      expect(contentNode.content).toBe('Feature: Login');
      expect(contentNode.metadata['category']).toBe('authentication');
      expect(contentNode.metadata['epicIds']).toContain('epic-1');
      expect(contentNode.relatedNodeIds).toContain('epic-1');
      expect(contentNode.relatedNodeIds).toContain('scenario-1');
    });

    it('should convert ScenarioNode to ContentNode', () => {
      const scenarioNode: ScenarioNode = {
        id: 'scenario-1',
        type: NodeType.SCENARIO,
        title: 'Login Scenario',
        description: 'User logs in',
        tags: [],
        sourcePath: '/features/login.feature',
        content: 'Scenario content',
        relatedNodeIds: [],
        featureId: 'feature-1',
        epicIds: ['epic-1'],
        scenarioType: 'scenario',
        steps: [{ keyword: 'Given', text: 'I am on the login page' }],
        metadata: {}
      };

      const contentNode = DocumentNodeAdapter.fromDocumentNode(scenarioNode);

      expect(contentNode.contentType).toBe('scenario');
      expect(contentNode.contentFormat).toBe('gherkin');
      expect(contentNode.metadata['featureId']).toBe('feature-1');
      expect(contentNode.metadata['scenarioType']).toBe('scenario');
      expect(contentNode.relatedNodeIds).toContain('feature-1');
      expect(contentNode.relatedNodeIds).toContain('epic-1');
    });
  });

  describe('toDocumentNode', () => {
    it('should convert ContentNode back to DocumentNode', () => {
      const contentNode: ContentNode = {
        id: 'node-1',
        contentType: 'epic',
        title: 'Test Node',
        description: 'Description',
        content: 'Content',
        contentFormat: 'markdown',
        tags: ['test'],
        sourcePath: '/test.md',
        relatedNodeIds: ['node-2'],
        metadata: { custom: 'value' }
      };

      const docNode = DocumentNodeAdapter.toDocumentNode(contentNode);

      expect(docNode.id).toBe('node-1');
      expect(docNode.type).toBe(NodeType.EPIC);
      expect(docNode.title).toBe('Test Node');
      expect(docNode.tags).toContain('test');
      expect(docNode.sourcePath).toBe('/test.md');
    });

    it('should handle feature content type', () => {
      const contentNode: ContentNode = {
        id: 'node-1',
        contentType: 'feature',
        title: 'Feature',
        description: 'Desc',
        content: 'Content',
        contentFormat: 'gherkin',
        tags: [],
        relatedNodeIds: [],
        metadata: {}
      };

      const docNode = DocumentNodeAdapter.toDocumentNode(contentNode);

      expect(docNode.type).toBe(NodeType.FEATURE);
    });

    it('should handle scenario content type', () => {
      const contentNode: ContentNode = {
        id: 'node-1',
        contentType: 'scenario',
        title: 'Scenario',
        description: 'Desc',
        content: 'Content',
        contentFormat: 'gherkin',
        tags: [],
        relatedNodeIds: [],
        metadata: {}
      };

      const docNode = DocumentNodeAdapter.toDocumentNode(contentNode);

      expect(docNode.type).toBe(NodeType.SCENARIO);
    });
  });

  describe('fromDocumentNodes', () => {
    it('should batch convert array of DocumentNodes', () => {
      const docNodes: DocumentNode[] = [
        {
          id: 'node-1',
          type: NodeType.EPIC,
          title: 'Epic 1',
          description: 'Desc 1',
          tags: [],
          sourcePath: '/epic1.md',
          content: 'Content 1',
          relatedNodeIds: [],
          metadata: {}
        },
        {
          id: 'node-2',
          type: NodeType.FEATURE,
          title: 'Feature 1',
          description: 'Desc 2',
          tags: [],
          sourcePath: '/feature1.feature',
          content: 'Content 2',
          relatedNodeIds: [],
          metadata: {}
        }
      ];

      const contentNodes = DocumentNodeAdapter.fromDocumentNodes(docNodes);

      expect(contentNodes.length).toBe(2);
      expect(contentNodes[0].contentType).toBe('epic');
      expect(contentNodes[1].contentType).toBe('feature');
    });
  });

  describe('toDocumentNodes', () => {
    it('should batch convert array of ContentNodes', () => {
      const contentNodes: ContentNode[] = [
        {
          id: 'node-1',
          contentType: 'epic',
          title: 'Epic',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'markdown',
          tags: [],
          relatedNodeIds: [],
          metadata: {}
        },
        {
          id: 'node-2',
          contentType: 'feature',
          title: 'Feature',
          description: 'Desc',
          content: 'Content',
          contentFormat: 'gherkin',
          tags: [],
          relatedNodeIds: [],
          metadata: {}
        }
      ];

      const docNodes = DocumentNodeAdapter.toDocumentNodes(contentNodes);

      expect(docNodes.length).toBe(2);
      expect(docNodes[0].type).toBe(NodeType.EPIC);
      expect(docNodes[1].type).toBe(NodeType.FEATURE);
    });
  });
});
