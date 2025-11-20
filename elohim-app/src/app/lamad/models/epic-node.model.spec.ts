import { EpicNode, EpicSection, EmbeddedReference } from './epic-node.model';
import { NodeType } from './document-node.model';

describe('EpicNode Model', () => {
  describe('EpicNode interface', () => {
    it('should create valid epic node', () => {
      const epic: EpicNode = {
        id: 'epic-1',
        type: NodeType.EPIC,
        title: 'Test Epic',
        description: 'An epic describing a feature',
        tags: ['test'],
        sourcePath: '/docs/test-epic.md',
        content: '# Test Epic',
        relatedNodeIds: [],
        featureIds: ['feature-1', 'feature-2'],
        relatedEpicIds: ['epic-2'],
        markdownContent: '# Test Epic\n\n## Section 1',
        sections: [],
        metadata: {}
      };

      expect(epic.type).toBe(NodeType.EPIC);
      expect(epic.featureIds).toContain('feature-1');
      expect(epic.relatedEpicIds).toContain('epic-2');
      expect(epic.markdownContent).toContain('# Test Epic');
    });

    it('should support optional author field', () => {
      const epic: EpicNode = {
        id: 'epic-1',
        type: NodeType.EPIC,
        title: 'Authored Epic',
        description: 'Epic with authors',
        tags: [],
        sourcePath: '/docs/epic.md',
        content: 'Content',
        relatedNodeIds: [],
        authors: ['Author 1', 'Author 2'],
        featureIds: [],
        relatedEpicIds: [],
        markdownContent: 'Content',
        sections: [],
        metadata: {}
      };

      expect(epic.authors).toContain('Author 1');
      expect(epic.authors).toContain('Author 2');
    });

    it('should support optional version field', () => {
      const epic: EpicNode = {
        id: 'epic-1',
        type: NodeType.EPIC,
        title: 'Versioned Epic',
        description: 'Epic with version',
        tags: [],
        sourcePath: '/docs/epic.md',
        content: 'Content',
        relatedNodeIds: [],
        version: '1.2.0',
        featureIds: [],
        relatedEpicIds: [],
        markdownContent: 'Content',
        sections: [],
        metadata: {}
      };

      expect(epic.version).toBe('1.2.0');
    });

    it('should support optional category field', () => {
      const epic: EpicNode = {
        id: 'epic-1',
        type: NodeType.EPIC,
        title: 'Categorized Epic',
        description: 'Epic with category',
        tags: [],
        sourcePath: '/docs/epic.md',
        content: 'Content',
        relatedNodeIds: [],
        category: 'Architecture',
        featureIds: [],
        relatedEpicIds: [],
        markdownContent: 'Content',
        sections: [],
        metadata: {}
      };

      expect(epic.category).toBe('Architecture');
    });
  });

  describe('EpicSection interface', () => {
    it('should create valid epic section', () => {
      const section: EpicSection = {
        title: 'Introduction',
        level: 2,
        anchor: 'introduction',
        content: 'This is the introduction section.',
        embeddedReferences: []
      };

      expect(section.title).toBe('Introduction');
      expect(section.level).toBe(2);
      expect(section.anchor).toBe('introduction');
      expect(section.content).toContain('introduction');
    });

    it('should support different heading levels', () => {
      const h1: EpicSection = {
        title: 'Main Heading',
        level: 1,
        anchor: 'main',
        content: 'Content',
        embeddedReferences: []
      };

      const h3: EpicSection = {
        title: 'Sub Section',
        level: 3,
        anchor: 'sub',
        content: 'Content',
        embeddedReferences: []
      };

      expect(h1.level).toBe(1);
      expect(h3.level).toBe(3);
    });

    it('should include embedded references', () => {
      const reference: EmbeddedReference = {
        type: 'feature',
        nodeId: 'feature-1',
        position: 42,
        displayText: 'User Login Feature'
      };

      const section: EpicSection = {
        title: 'Features',
        level: 2,
        anchor: 'features',
        content: 'This section references features',
        embeddedReferences: [reference]
      };

      expect(section.embeddedReferences).toHaveLength(1);
      expect(section.embeddedReferences[0].type).toBe('feature');
    });
  });

  describe('EmbeddedReference interface', () => {
    it('should create feature reference', () => {
      const reference: EmbeddedReference = {
        type: 'feature',
        nodeId: 'feature-1',
        position: 100,
        displayText: 'Login Feature'
      };

      expect(reference.type).toBe('feature');
      expect(reference.nodeId).toBe('feature-1');
      expect(reference.position).toBe(100);
      expect(reference.displayText).toBe('Login Feature');
    });

    it('should create scenario reference', () => {
      const reference: EmbeddedReference = {
        type: 'scenario',
        nodeId: 'scenario-1',
        position: 200,
        displayText: 'Successful login scenario'
      };

      expect(reference.type).toBe('scenario');
      expect(reference.nodeId).toBe('scenario-1');
    });

    it('should track position in content', () => {
      const ref1: EmbeddedReference = {
        type: 'feature',
        nodeId: 'f-1',
        position: 50,
        displayText: 'First'
      };

      const ref2: EmbeddedReference = {
        type: 'scenario',
        nodeId: 's-1',
        position: 150,
        displayText: 'Second'
      };

      expect(ref1.position).toBeLessThan(ref2.position);
    });
  });

  describe('EpicNode with sections', () => {
    it('should organize content into sections', () => {
      const sections: EpicSection[] = [
        {
          title: 'Overview',
          level: 1,
          anchor: 'overview',
          content: 'Overview content',
          embeddedReferences: []
        },
        {
          title: 'Details',
          level: 2,
          anchor: 'details',
          content: 'Detailed content',
          embeddedReferences: []
        }
      ];

      const epic: EpicNode = {
        id: 'epic-1',
        type: NodeType.EPIC,
        title: 'Structured Epic',
        description: 'Epic with sections',
        tags: [],
        sourcePath: '/docs/epic.md',
        content: 'Full content',
        relatedNodeIds: [],
        featureIds: [],
        relatedEpicIds: [],
        markdownContent: '# Overview\n\n## Details',
        sections,
        metadata: {}
      };

      expect(epic.sections).toHaveLength(2);
      expect(epic.sections[0].title).toBe('Overview');
      expect(epic.sections[1].title).toBe('Details');
    });
  });
});
