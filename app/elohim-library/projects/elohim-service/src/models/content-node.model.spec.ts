/**
 * Tests for ContentNode model types and enums
 */

import {
  ContentNode,
  ContentType,
  ContentFormat,
  ContentReach,
  ContentRelationshipType,
  ContentRelationship,
  ContentMetadata
} from './content-node.model';

describe('ContentNode Model', () => {
  describe('ContentType', () => {
    it('should include all valid content types', () => {
      const types: ContentType[] = [
        'source',
        'epic',
        'feature',
        'scenario',
        'concept',
        'role',
        'video',
        'organization',
        'book-chapter',
        'tool',
        'path',
        'assessment',
        'reference',
        'example'
      ];

      expect(types).toHaveLength(14);
    });
  });

  describe('ContentFormat', () => {
    it('should include all valid content formats', () => {
      const formats: ContentFormat[] = [
        'markdown',
        'gherkin',
        'html',
        'plaintext',
        'video-embed',
        'external-link',
        'perseus-quiz-json'
      ];

      expect(formats).toHaveLength(7);
    });
  });

  describe('ContentReach', () => {
    it('should include all valid reach levels', () => {
      const reaches: ContentReach[] = [
        'private',
        'invited',
        'local',
        'community',
        'federated',
        'commons'
      ];

      expect(reaches).toHaveLength(6);
    });

    it('should order from most restrictive to most open', () => {
      const reaches: ContentReach[] = [
        'private',
        'invited',
        'local',
        'community',
        'federated',
        'commons'
      ];

      // Verify ordering is logical
      expect(reaches.indexOf('private')).toBeLessThan(reaches.indexOf('commons'));
      expect(reaches.indexOf('invited')).toBeLessThan(reaches.indexOf('federated'));
    });
  });

  describe('ContentRelationshipType enum', () => {
    it('should include all relationship types', () => {
      const types = Object.values(ContentRelationshipType);

      expect(types).toContain('CONTAINS');
      expect(types).toContain('BELONGS_TO');
      expect(types).toContain('DESCRIBES');
      expect(types).toContain('IMPLEMENTS');
      expect(types).toContain('VALIDATES');
      expect(types).toContain('RELATES_TO');
      expect(types).toContain('REFERENCES');
      expect(types).toContain('DEPENDS_ON');
      expect(types).toContain('REQUIRES');
      expect(types).toContain('FOLLOWS');
      expect(types).toContain('DERIVED_FROM');
      expect(types).toContain('SOURCE_OF');
    });

    it('should have inverse relationships', () => {
      // CONTAINS <-> BELONGS_TO
      expect(ContentRelationshipType.CONTAINS).toBe('CONTAINS');
      expect(ContentRelationshipType.BELONGS_TO).toBe('BELONGS_TO');

      // DESCRIBES <-> IMPLEMENTS
      expect(ContentRelationshipType.DESCRIBES).toBe('DESCRIBES');
      expect(ContentRelationshipType.IMPLEMENTS).toBe('IMPLEMENTS');

      // DERIVED_FROM <-> SOURCE_OF
      expect(ContentRelationshipType.DERIVED_FROM).toBe('DERIVED_FROM');
      expect(ContentRelationshipType.SOURCE_OF).toBe('SOURCE_OF');
    });

    it('should support string comparison', () => {
      const relType: string = ContentRelationshipType.CONTAINS;
      expect(relType).toBe('CONTAINS');
    });
  });

  describe('ContentMetadata interface', () => {
    it('should accept minimal metadata', () => {
      const metadata: ContentMetadata = {};

      expect(metadata).toBeDefined();
    });

    it('should accept standard metadata fields', () => {
      const metadata: ContentMetadata = {
        category: 'governance',
        authors: ['Alice', 'Bob'],
        version: '1.0.0',
        status: 'published',
        priority: 1
      };

      expect(metadata.category).toBe('governance');
      expect(metadata.authors).toHaveLength(2);
      expect(metadata.version).toBe('1.0.0');
    });

    it('should accept import-specific metadata', () => {
      const metadata: ContentMetadata = {
        sourcePath: 'data/content/elohim-protocol/governance/epic.md',
        importedAt: '2026-02-01T12:00:00.000Z',
        importVersion: '0.1.0',
        derivedFrom: 'source-epic-governance',
        extractionMethod: 'markdown-parser'
      };

      expect(metadata.sourcePath).toContain('governance');
      expect(metadata.derivedFrom).toBe('source-epic-governance');
    });

    it('should accept epic and archetype metadata', () => {
      const metadata: ContentMetadata = {
        epic: 'governance',
        userType: 'policy_maker',
        governanceScope: ['local', 'regional', 'global']
      };

      expect(metadata.epic).toBe('governance');
      expect(metadata.userType).toBe('policy_maker');
      expect(metadata.governanceScope).toHaveLength(3);
    });

    it('should accept custom domain-specific fields', () => {
      const metadata: ContentMetadata = {
        customField1: 'value1',
        customField2: 42,
        customField3: true,
        customNested: {
          nested1: 'nested-value',
          nested2: [1, 2, 3]
        }
      };

      expect(metadata.customField1).toBe('value1');
      expect(metadata.customField2).toBe(42);
      expect(metadata.customField3).toBe(true);
    });

    it('should accept source and reference metadata', () => {
      const metadata: ContentMetadata = {
        source: 'Climate Justice Book',
        sourceUrl: 'https://example.com/climate-justice',
        estimatedTime: '30 minutes',
        keywords: ['climate', 'justice', 'governance']
      };

      expect(metadata.sourceUrl).toContain('https://');
      expect(metadata.keywords).toHaveLength(3);
    });
  });

  describe('ContentNode interface', () => {
    it('should accept minimal valid node', () => {
      const node: ContentNode = {
        id: 'epic-governance',
        contentType: 'epic',
        title: 'Governance Epic',
        description: 'Governance domain narrative',
        content: '# Governance\n\nContent here...',
        contentFormat: 'markdown',
        tags: ['governance', 'epic'],
        relatedNodeIds: [],
        metadata: {},
        createdAt: '2026-02-01T12:00:00.000Z',
        updatedAt: '2026-02-01T12:00:00.000Z'
      };

      expect(node.id).toBe('epic-governance');
      expect(node.contentType).toBe('epic');
      expect(node.tags).toHaveLength(2);
    });

    it('should accept node with all optional fields', () => {
      const node: ContentNode = {
        id: 'scenario-governance-policy-maker-funding',
        contentType: 'scenario',
        title: 'Policy Maker Funding Scenario',
        description: 'Funding allocation scenario for policy makers',
        content: 'Feature: Funding\n\nScenario: Allocate funds...',
        contentFormat: 'gherkin',
        tags: ['governance', 'policy-maker', 'scenario'],
        sourcePath: 'data/content/elohim-protocol/governance/policy_maker/scenarios/funding.feature',
        relatedNodeIds: ['archetype-governance-policy-maker', 'epic-governance'],
        metadata: {
          epic: 'governance',
          userType: 'policy_maker',
          sourcePath: 'data/content/elohim-protocol/governance/policy_maker/scenarios/funding.feature',
          importedAt: '2026-02-01T12:00:00.000Z'
        },
        authorId: 'user-123',
        reach: 'commons',
        createdAt: '2026-02-01T12:00:00.000Z',
        updatedAt: '2026-02-01T12:00:00.000Z'
      };

      expect(node.sourcePath).toBeDefined();
      expect(node.authorId).toBe('user-123');
      expect(node.reach).toBe('commons');
      expect(node.relatedNodeIds).toHaveLength(2);
    });

    it('should accept different content types with appropriate formats', () => {
      const epicNode: ContentNode = {
        id: 'epic-1',
        contentType: 'epic',
        title: 'Epic',
        description: 'Description',
        content: '# Epic',
        contentFormat: 'markdown',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
        createdAt: '2026-02-01T12:00:00.000Z',
        updatedAt: '2026-02-01T12:00:00.000Z'
      };

      const scenarioNode: ContentNode = {
        id: 'scenario-1',
        contentType: 'scenario',
        title: 'Scenario',
        description: 'Description',
        content: 'Feature: Test',
        contentFormat: 'gherkin',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
        createdAt: '2026-02-01T12:00:00.000Z',
        updatedAt: '2026-02-01T12:00:00.000Z'
      };

      const videoNode: ContentNode = {
        id: 'video-1',
        contentType: 'video',
        title: 'Video',
        description: 'Description',
        content: 'https://youtube.com/watch?v=123',
        contentFormat: 'video-embed',
        tags: [],
        relatedNodeIds: [],
        metadata: {},
        createdAt: '2026-02-01T12:00:00.000Z',
        updatedAt: '2026-02-01T12:00:00.000Z'
      };

      expect(epicNode.contentFormat).toBe('markdown');
      expect(scenarioNode.contentFormat).toBe('gherkin');
      expect(videoNode.contentFormat).toBe('video-embed');
    });

    it('should support rich metadata', () => {
      const node: ContentNode = {
        id: 'book-chapter-1',
        contentType: 'book-chapter',
        title: 'Climate Justice Chapter 1',
        description: 'Introduction to climate justice',
        content: '# Chapter 1\n\nIntroduction...',
        contentFormat: 'markdown',
        tags: ['climate', 'justice'],
        relatedNodeIds: ['epic-governance'],
        metadata: {
          category: 'resource',
          authors: ['Mary Robinson'],
          version: '1.0.0',
          source: 'Climate Justice: Hope, Resilience, and the Fight for a Sustainable Future',
          sourceUrl: 'https://example.com/climate-justice',
          estimatedTime: '30 minutes',
          keywords: ['climate', 'justice', 'sustainability'],
          epic: 'governance'
        },
        reach: 'commons',
        createdAt: '2026-02-01T12:00:00.000Z',
        updatedAt: '2026-02-01T12:00:00.000Z'
      };

      expect(node.metadata.authors).toContain('Mary Robinson');
      expect(node.metadata.keywords).toHaveLength(3);
    });
  });

  describe('ContentRelationship interface', () => {
    it('should accept minimal relationship', () => {
      const relationship: ContentRelationship = {
        id: 'rel-1',
        sourceNodeId: 'epic-governance',
        targetNodeId: 'scenario-funding',
        relationshipType: ContentRelationshipType.DESCRIBES
      };

      expect(relationship.sourceNodeId).toBe('epic-governance');
      expect(relationship.targetNodeId).toBe('scenario-funding');
      expect(relationship.relationshipType).toBe('DESCRIBES');
    });

    it('should accept relationship with confidence score', () => {
      const relationship: ContentRelationship = {
        id: 'rel-2',
        sourceNodeId: 'concept-1',
        targetNodeId: 'concept-2',
        relationshipType: ContentRelationshipType.RELATES_TO,
        confidence: 0.85
      };

      expect(relationship.confidence).toBe(0.85);
    });

    it('should accept relationship with inference source', () => {
      const relationship: ContentRelationship = {
        id: 'rel-3',
        sourceNodeId: 'epic-1',
        targetNodeId: 'scenario-1',
        relationshipType: ContentRelationshipType.CONTAINS,
        inferenceSource: 'path'
      };

      expect(relationship.inferenceSource).toBe('path');
    });

    it('should accept relationship with metadata', () => {
      const relationship: ContentRelationship = {
        id: 'rel-4',
        sourceNodeId: 'source-epic',
        targetNodeId: 'epic-derived',
        relationshipType: ContentRelationshipType.SOURCE_OF,
        confidence: 1.0,
        inferenceSource: 'explicit',
        metadata: {
          transformationType: 'markdown-extraction',
          extractedAt: '2026-02-01T12:00:00.000Z'
        }
      };

      expect(relationship.metadata?.transformationType).toBe('markdown-extraction');
      expect(relationship.confidence).toBe(1.0);
    });

    it('should support all relationship types', () => {
      const relationships: ContentRelationship[] = [
        {
          id: 'r1',
          sourceNodeId: 'parent',
          targetNodeId: 'child',
          relationshipType: ContentRelationshipType.CONTAINS
        },
        {
          id: 'r2',
          sourceNodeId: 'child',
          targetNodeId: 'parent',
          relationshipType: ContentRelationshipType.BELONGS_TO
        },
        {
          id: 'r3',
          sourceNodeId: 'epic',
          targetNodeId: 'scenario',
          relationshipType: ContentRelationshipType.DESCRIBES
        },
        {
          id: 'r4',
          sourceNodeId: 'scenario',
          targetNodeId: 'epic',
          relationshipType: ContentRelationshipType.IMPLEMENTS
        },
        {
          id: 'r5',
          sourceNodeId: 'test',
          targetNodeId: 'feature',
          relationshipType: ContentRelationshipType.VALIDATES
        },
        {
          id: 'r6',
          sourceNodeId: 'concept1',
          targetNodeId: 'concept2',
          relationshipType: ContentRelationshipType.RELATES_TO
        },
        {
          id: 'r7',
          sourceNodeId: 'article',
          targetNodeId: 'source',
          relationshipType: ContentRelationshipType.REFERENCES
        },
        {
          id: 'r8',
          sourceNodeId: 'step2',
          targetNodeId: 'step1',
          relationshipType: ContentRelationshipType.DEPENDS_ON
        },
        {
          id: 'r9',
          sourceNodeId: 'advanced',
          targetNodeId: 'basic',
          relationshipType: ContentRelationshipType.REQUIRES
        },
        {
          id: 'r10',
          sourceNodeId: 'step1',
          targetNodeId: 'step2',
          relationshipType: ContentRelationshipType.FOLLOWS
        },
        {
          id: 'r11',
          sourceNodeId: 'derived',
          targetNodeId: 'source',
          relationshipType: ContentRelationshipType.DERIVED_FROM
        },
        {
          id: 'r12',
          sourceNodeId: 'source',
          targetNodeId: 'derived',
          relationshipType: ContentRelationshipType.SOURCE_OF
        }
      ];

      expect(relationships).toHaveLength(12);
    });

    it('should support bidirectional relationships', () => {
      const forward: ContentRelationship = {
        id: 'rel-forward',
        sourceNodeId: 'node-a',
        targetNodeId: 'node-b',
        relationshipType: ContentRelationshipType.CONTAINS
      };

      const reverse: ContentRelationship = {
        id: 'rel-reverse',
        sourceNodeId: 'node-b',
        targetNodeId: 'node-a',
        relationshipType: ContentRelationshipType.BELONGS_TO
      };

      expect(forward.sourceNodeId).toBe(reverse.targetNodeId);
      expect(forward.targetNodeId).toBe(reverse.sourceNodeId);
    });
  });

  describe('Inference sources', () => {
    it('should support all inference source types', () => {
      const sources: ContentRelationship['inferenceSource'][] = [
        'explicit',
        'path',
        'tag',
        'semantic'
      ];

      sources.forEach(source => {
        const rel: ContentRelationship = {
          id: `rel-${source}`,
          sourceNodeId: 'a',
          targetNodeId: 'b',
          relationshipType: ContentRelationshipType.RELATES_TO,
          inferenceSource: source
        };

        expect(rel.inferenceSource).toBe(source);
      });
    });
  });
});
