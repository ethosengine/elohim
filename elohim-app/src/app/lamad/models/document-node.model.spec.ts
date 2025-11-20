import { DocumentNode, NodeType, SearchResult } from './document-node.model';

describe('DocumentNode Model', () => {
  describe('NodeType enum', () => {
    it('should have all node types', () => {
      expect(NodeType.EPIC).toBe('epic');
      expect(NodeType.FEATURE).toBe('feature');
      expect(NodeType.SCENARIO).toBe('scenario');
    });
  });

  describe('DocumentNode interface', () => {
    it('should create valid document node', () => {
      const node: DocumentNode = {
        id: 'node-1',
        type: NodeType.EPIC,
        title: 'Test Epic',
        description: 'A test epic node',
        tags: ['test', 'documentation'],
        sourcePath: '/docs/test-epic.md',
        content: '# Test Epic\n\nThis is test content.',
        relatedNodeIds: ['node-2', 'node-3'],
        metadata: {}
      };

      expect(node.id).toBe('node-1');
      expect(node.type).toBe(NodeType.EPIC);
      expect(node.title).toBe('Test Epic');
      expect(node.tags).toContain('test');
      expect(node.relatedNodeIds).toHaveLength(2);
    });

    it('should support optional timestamps', () => {
      const createdAt = new Date('2025-01-01');
      const updatedAt = new Date('2025-01-15');

      const node: DocumentNode = {
        id: 'node-1',
        type: NodeType.FEATURE,
        title: 'Test Feature',
        description: 'A test feature',
        tags: [],
        sourcePath: '/features/test.feature',
        content: 'Feature content',
        relatedNodeIds: [],
        createdAt,
        updatedAt,
        metadata: {}
      };

      expect(node.createdAt).toEqual(createdAt);
      expect(node.updatedAt).toEqual(updatedAt);
    });

    it('should support metadata', () => {
      const node: DocumentNode = {
        id: 'node-1',
        type: NodeType.SCENARIO,
        title: 'Test Scenario',
        description: 'Test description',
        tags: [],
        sourcePath: '/test.feature',
        content: 'Scenario content',
        relatedNodeIds: [],
        metadata: {
          author: 'Test Author',
          version: '1.0',
          customField: 'custom value'
        }
      };

      expect(node.metadata.author).toBe('Test Author');
      expect(node.metadata.version).toBe('1.0');
      expect(node.metadata.customField).toBe('custom value');
    });

    it('should handle empty tags and relatedNodeIds', () => {
      const node: DocumentNode = {
        id: 'node-1',
        type: NodeType.EPIC,
        title: 'Isolated Node',
        description: 'Node with no connections',
        tags: [],
        sourcePath: '/docs/isolated.md',
        content: 'Content',
        relatedNodeIds: [],
        metadata: {}
      };

      expect(node.tags).toHaveLength(0);
      expect(node.relatedNodeIds).toHaveLength(0);
    });
  });

  describe('SearchResult interface', () => {
    it('should create valid search result', () => {
      const node: DocumentNode = {
        id: 'node-1',
        type: NodeType.EPIC,
        title: 'Search Result Node',
        description: 'Found in search',
        tags: ['search'],
        sourcePath: '/docs/result.md',
        content: 'This contains the search term',
        relatedNodeIds: [],
        metadata: {}
      };

      const result: SearchResult = {
        node,
        score: 0.95,
        highlightedContent: 'This contains the <mark>search term</mark>',
        matchedIn: ['content', 'tags']
      };

      expect(result.node).toBe(node);
      expect(result.score).toBe(0.95);
      expect(result.highlightedContent).toContain('<mark>');
      expect(result.matchedIn).toContain('content');
      expect(result.matchedIn).toContain('tags');
    });

    it('should support matches in title', () => {
      const node: DocumentNode = {
        id: 'node-1',
        type: NodeType.FEATURE,
        title: 'Search in Title',
        description: 'Description',
        tags: [],
        sourcePath: '/test.feature',
        content: 'Content',
        relatedNodeIds: [],
        metadata: {}
      };

      const result: SearchResult = {
        node,
        score: 1.0,
        highlightedContent: '<mark>Search</mark> in Title',
        matchedIn: ['title']
      };

      expect(result.matchedIn).toContain('title');
      expect(result.matchedIn).toHaveLength(1);
    });

    it('should support matches in description', () => {
      const node: DocumentNode = {
        id: 'node-1',
        type: NodeType.SCENARIO,
        title: 'Node Title',
        description: 'Description with search term',
        tags: [],
        sourcePath: '/test.feature',
        content: 'Content',
        relatedNodeIds: [],
        metadata: {}
      };

      const result: SearchResult = {
        node,
        score: 0.8,
        highlightedContent: 'Description with <mark>search term</mark>',
        matchedIn: ['description']
      };

      expect(result.matchedIn).toContain('description');
    });

    it('should support multiple match locations', () => {
      const node: DocumentNode = {
        id: 'node-1',
        type: NodeType.EPIC,
        title: 'Search term in title',
        description: 'Search term in description',
        tags: ['search'],
        sourcePath: '/docs/search.md',
        content: 'Search term in content',
        relatedNodeIds: [],
        metadata: {}
      };

      const result: SearchResult = {
        node,
        score: 1.0,
        highlightedContent: 'Matches everywhere',
        matchedIn: ['title', 'description', 'content', 'tags']
      };

      expect(result.matchedIn).toHaveLength(4);
      expect(result.matchedIn).toContain('title');
      expect(result.matchedIn).toContain('description');
      expect(result.matchedIn).toContain('content');
      expect(result.matchedIn).toContain('tags');
    });
  });
});
