/**
 * Tests for Holochain model types and constants
 */

import {
  CreateContentInput,
  HolochainContent,
  HolochainContentOutput,
  BulkCreateContentInput,
  BulkCreateContentOutput,
  QueryByTypeInput,
  QueryByIdInput,
  ContentStats,
  CreatePathInput,
  AddPathStepInput,
  HolochainLearningPath,
  HolochainPathStep,
  PathStepOutput,
  PathWithSteps,
  HolochainContentRelationship,
  HolochainClientConfig,
  HolochainImportConfig,
  HolochainImportResult,
  HolochainVerifyResult,
  ZomeCallInput,
  ZomeCallResult,
  VALID_CONTENT_TYPES,
  VALID_CONTENT_FORMATS,
  VALID_REACH_LEVELS,
  VALID_RELATIONSHIP_TYPES,
  VALID_DIFFICULTY_LEVELS,
  ValidContentType,
  ValidContentFormat,
  ValidReachLevel,
  ValidRelationshipType,
  ValidDifficultyLevel
} from './holochain.model';

describe('Holochain Model', () => {
  describe('Constants', () => {
    describe('VALID_CONTENT_TYPES', () => {
      it('should contain all valid content types', () => {
        expect(VALID_CONTENT_TYPES).toContain('source');
        expect(VALID_CONTENT_TYPES).toContain('epic');
        expect(VALID_CONTENT_TYPES).toContain('feature');
        expect(VALID_CONTENT_TYPES).toContain('scenario');
        expect(VALID_CONTENT_TYPES).toContain('concept');
        expect(VALID_CONTENT_TYPES).toContain('role');
        expect(VALID_CONTENT_TYPES).toContain('video');
        expect(VALID_CONTENT_TYPES).toContain('organization');
        expect(VALID_CONTENT_TYPES).toContain('book-chapter');
        expect(VALID_CONTENT_TYPES).toContain('tool');
        expect(VALID_CONTENT_TYPES).toContain('path');
        expect(VALID_CONTENT_TYPES).toContain('assessment');
        expect(VALID_CONTENT_TYPES).toContain('reference');
        expect(VALID_CONTENT_TYPES).toContain('example');
      });

      it('should have exactly 14 content types', () => {
        expect(VALID_CONTENT_TYPES).toHaveLength(14);
      });

      it('should be readonly array', () => {
        expect(Array.isArray(VALID_CONTENT_TYPES)).toBe(true);
      });
    });

    describe('VALID_CONTENT_FORMATS', () => {
      it('should contain all valid content formats', () => {
        expect(VALID_CONTENT_FORMATS).toContain('markdown');
        expect(VALID_CONTENT_FORMATS).toContain('gherkin');
        expect(VALID_CONTENT_FORMATS).toContain('html');
        expect(VALID_CONTENT_FORMATS).toContain('plaintext');
        expect(VALID_CONTENT_FORMATS).toContain('video-embed');
        expect(VALID_CONTENT_FORMATS).toContain('external-link');
        expect(VALID_CONTENT_FORMATS).toContain('quiz-json');
        expect(VALID_CONTENT_FORMATS).toContain('assessment-json');
      });

      it('should have exactly 8 content formats', () => {
        expect(VALID_CONTENT_FORMATS).toHaveLength(8);
      });
    });

    describe('VALID_REACH_LEVELS', () => {
      it('should contain all valid reach levels', () => {
        expect(VALID_REACH_LEVELS).toContain('private');
        expect(VALID_REACH_LEVELS).toContain('invited');
        expect(VALID_REACH_LEVELS).toContain('local');
        expect(VALID_REACH_LEVELS).toContain('community');
        expect(VALID_REACH_LEVELS).toContain('federated');
        expect(VALID_REACH_LEVELS).toContain('commons');
      });

      it('should have exactly 6 reach levels', () => {
        expect(VALID_REACH_LEVELS).toHaveLength(6);
      });
    });

    describe('VALID_RELATIONSHIP_TYPES', () => {
      it('should contain all valid relationship types', () => {
        expect(VALID_RELATIONSHIP_TYPES).toContain('CONTAINS');
        expect(VALID_RELATIONSHIP_TYPES).toContain('BELONGS_TO');
        expect(VALID_RELATIONSHIP_TYPES).toContain('DESCRIBES');
        expect(VALID_RELATIONSHIP_TYPES).toContain('IMPLEMENTS');
        expect(VALID_RELATIONSHIP_TYPES).toContain('VALIDATES');
        expect(VALID_RELATIONSHIP_TYPES).toContain('RELATES_TO');
        expect(VALID_RELATIONSHIP_TYPES).toContain('REFERENCES');
        expect(VALID_RELATIONSHIP_TYPES).toContain('DEPENDS_ON');
        expect(VALID_RELATIONSHIP_TYPES).toContain('REQUIRES');
        expect(VALID_RELATIONSHIP_TYPES).toContain('FOLLOWS');
        expect(VALID_RELATIONSHIP_TYPES).toContain('DERIVED_FROM');
        expect(VALID_RELATIONSHIP_TYPES).toContain('SOURCE_OF');
      });

      it('should have exactly 12 relationship types', () => {
        expect(VALID_RELATIONSHIP_TYPES).toHaveLength(12);
      });
    });

    describe('VALID_DIFFICULTY_LEVELS', () => {
      it('should contain all valid difficulty levels', () => {
        expect(VALID_DIFFICULTY_LEVELS).toContain('beginner');
        expect(VALID_DIFFICULTY_LEVELS).toContain('intermediate');
        expect(VALID_DIFFICULTY_LEVELS).toContain('advanced');
      });

      it('should have exactly 3 difficulty levels', () => {
        expect(VALID_DIFFICULTY_LEVELS).toHaveLength(3);
      });
    });
  });

  describe('Type aliases from constants', () => {
    it('should accept valid content type', () => {
      const contentType: ValidContentType = 'epic';
      expect(contentType).toBe('epic');
    });

    it('should accept valid content format', () => {
      const format: ValidContentFormat = 'markdown';
      expect(format).toBe('markdown');
    });

    it('should accept valid reach level', () => {
      const reach: ValidReachLevel = 'commons';
      expect(reach).toBe('commons');
    });

    it('should accept valid relationship type', () => {
      const relType: ValidRelationshipType = 'CONTAINS';
      expect(relType).toBe('CONTAINS');
    });

    it('should accept valid difficulty level', () => {
      const difficulty: ValidDifficultyLevel = 'intermediate';
      expect(difficulty).toBe('intermediate');
    });
  });

  describe('Content types', () => {
    describe('CreateContentInput', () => {
      it('should accept valid input', () => {
        const input: CreateContentInput = {
          id: 'epic-governance',
          content_type: 'epic',
          title: 'Governance Epic',
          description: 'Governance domain narrative',
          content: '# Governance\n\nContent here...',
          content_format: 'markdown',
          tags: ['governance', 'epic'],
          source_path: 'data/content/elohim-protocol/governance/epic.md',
          related_node_ids: [],
          reach: 'commons',
          metadata_json: '{}'
        };

        expect(input.id).toBe('epic-governance');
        expect(input.content_type).toBe('epic');
      });

      it('should accept null source_path', () => {
        const input: CreateContentInput = {
          id: 'test',
          content_type: 'concept',
          title: 'Test',
          description: 'Test description',
          content: 'Content',
          content_format: 'plaintext',
          tags: [],
          source_path: null,
          related_node_ids: [],
          reach: 'private',
          metadata_json: '{}'
        };

        expect(input.source_path).toBeNull();
      });
    });

    describe('HolochainContent', () => {
      it('should accept complete content entry', () => {
        const content: HolochainContent = {
          id: 'epic-governance',
          content_type: 'epic',
          title: 'Governance Epic',
          description: 'Description',
          content: 'Content body',
          content_format: 'markdown',
          tags: ['governance'],
          source_path: 'data/content/epic.md',
          related_node_ids: [],
          author_id: 'user-123',
          reach: 'commons',
          trust_score: 0.95,
          metadata_json: '{}',
          created_at: '2026-02-01T12:00:00.000Z',
          updated_at: '2026-02-01T12:00:00.000Z'
        };

        expect(content.trust_score).toBe(0.95);
        expect(content.author_id).toBe('user-123');
      });

      it('should accept null author_id and source_path', () => {
        const content: HolochainContent = {
          id: 'test',
          content_type: 'concept',
          title: 'Test',
          description: 'Description',
          content: 'Content',
          content_format: 'plaintext',
          tags: [],
          source_path: null,
          related_node_ids: [],
          author_id: null,
          reach: 'private',
          trust_score: 0,
          metadata_json: '{}',
          created_at: '2026-02-01T12:00:00.000Z',
          updated_at: '2026-02-01T12:00:00.000Z'
        };

        expect(content.author_id).toBeNull();
        expect(content.source_path).toBeNull();
      });
    });

    describe('BulkCreateContentInput', () => {
      it('should accept bulk input with multiple contents', () => {
        const bulkInput: BulkCreateContentInput = {
          import_id: 'import-2026-02-01',
          contents: [
            {
              id: 'epic-1',
              content_type: 'epic',
              title: 'Epic 1',
              description: 'Desc 1',
              content: 'Content 1',
              content_format: 'markdown',
              tags: [],
              source_path: null,
              related_node_ids: [],
              reach: 'commons',
              metadata_json: '{}'
            },
            {
              id: 'epic-2',
              content_type: 'epic',
              title: 'Epic 2',
              description: 'Desc 2',
              content: 'Content 2',
              content_format: 'markdown',
              tags: [],
              source_path: null,
              related_node_ids: [],
              reach: 'commons',
              metadata_json: '{}'
            }
          ]
        };

        expect(bulkInput.contents).toHaveLength(2);
      });
    });

    describe('BulkCreateContentOutput', () => {
      it('should accept bulk output', () => {
        const mockHash = new Uint8Array([1, 2, 3, 4]);

        const output: BulkCreateContentOutput = {
          import_id: 'import-123',
          created_count: 5,
          action_hashes: [mockHash, mockHash, mockHash, mockHash, mockHash],
          errors: []
        };

        expect(output.created_count).toBe(5);
        expect(output.errors).toHaveLength(0);
      });

      it('should accept output with errors', () => {
        const output: BulkCreateContentOutput = {
          import_id: 'import-123',
          created_count: 3,
          action_hashes: [],
          errors: ['Error 1', 'Error 2']
        };

        expect(output.errors).toHaveLength(2);
      });
    });
  });

  describe('Query types', () => {
    describe('QueryByTypeInput', () => {
      it('should accept query with type only', () => {
        const query: QueryByTypeInput = {
          content_type: 'epic'
        };

        expect(query.content_type).toBe('epic');
      });

      it('should accept query with limit', () => {
        const query: QueryByTypeInput = {
          content_type: 'scenario',
          limit: 10
        };

        expect(query.limit).toBe(10);
      });
    });

    describe('QueryByIdInput', () => {
      it('should accept query with ID', () => {
        const query: QueryByIdInput = {
          id: 'epic-governance'
        };

        expect(query.id).toBe('epic-governance');
      });
    });

    describe('ContentStats', () => {
      it('should accept stats', () => {
        const stats: ContentStats = {
          total_count: 100,
          by_type: {
            epic: 5,
            scenario: 30,
            concept: 20,
            resource: 45
          }
        };

        expect(stats.total_count).toBe(100);
        expect(stats.by_type.epic).toBe(5);
      });
    });
  });

  describe('Learning path types', () => {
    describe('CreatePathInput', () => {
      it('should accept path creation input', () => {
        const input: CreatePathInput = {
          id: 'path-governance-intro',
          version: '1.0.0',
          title: 'Introduction to Governance',
          description: 'Learn the basics of governance',
          purpose: 'To understand governance principles',
          difficulty: 'beginner',
          estimated_duration: '2 hours',
          visibility: 'commons',
          path_type: 'guided',
          tags: ['governance', 'intro']
        };

        expect(input.difficulty).toBe('beginner');
        expect(input.purpose).toBe('To understand governance principles');
      });

      it('should accept input without optional fields', () => {
        const input: CreatePathInput = {
          id: 'path-test',
          version: '1.0.0',
          title: 'Test Path',
          description: 'Test description',
          difficulty: 'intermediate',
          visibility: 'private',
          path_type: 'self-paced',
          tags: []
        };

        expect(input.purpose).toBeUndefined();
        expect(input.estimated_duration).toBeUndefined();
      });
    });

    describe('AddPathStepInput', () => {
      it('should accept step input', () => {
        const input: AddPathStepInput = {
          path_id: 'path-governance-intro',
          order_index: 1,
          step_type: 'content',
          resource_id: 'epic-governance',
          step_title: 'Read the Governance Epic',
          step_narrative: 'Start by understanding the big picture',
          is_optional: false
        };

        expect(input.order_index).toBe(1);
        expect(input.is_optional).toBe(false);
      });

      it('should accept optional step', () => {
        const input: AddPathStepInput = {
          path_id: 'path-test',
          order_index: 5,
          step_type: 'optional',
          resource_id: 'resource-123',
          is_optional: true
        };

        expect(input.is_optional).toBe(true);
      });
    });

    describe('HolochainLearningPath', () => {
      it('should accept complete path', () => {
        const path: HolochainLearningPath = {
          id: 'path-governance-intro',
          version: '1.0.0',
          title: 'Introduction to Governance',
          description: 'Learn governance basics',
          purpose: 'Understanding governance',
          created_by: 'user-123',
          difficulty: 'beginner',
          estimated_duration: '2 hours',
          visibility: 'commons',
          path_type: 'guided',
          tags: ['governance'],
          created_at: '2026-02-01T12:00:00.000Z',
          updated_at: '2026-02-01T12:00:00.000Z'
        };

        expect(path.created_by).toBe('user-123');
      });

      it('should accept path with null optional fields', () => {
        const path: HolochainLearningPath = {
          id: 'path-test',
          version: '1.0.0',
          title: 'Test',
          description: 'Description',
          purpose: null,
          created_by: 'user-123',
          difficulty: 'intermediate',
          estimated_duration: null,
          visibility: 'private',
          path_type: 'self-paced',
          tags: [],
          created_at: '2026-02-01T12:00:00.000Z',
          updated_at: '2026-02-01T12:00:00.000Z'
        };

        expect(path.purpose).toBeNull();
        expect(path.estimated_duration).toBeNull();
      });
    });
  });

  describe('Relationship types', () => {
    describe('HolochainContentRelationship', () => {
      it('should accept relationship', () => {
        const rel: HolochainContentRelationship = {
          id: 'rel-1',
          source_node_id: 'epic-governance',
          target_node_id: 'scenario-funding',
          relationship_type: 'DESCRIBES',
          confidence: 1.0,
          metadata_json: '{"source": "explicit"}'
        };

        expect(rel.confidence).toBe(1.0);
        expect(rel.metadata_json).toBeDefined();
      });

      it('should accept relationship with null metadata', () => {
        const rel: HolochainContentRelationship = {
          id: 'rel-2',
          source_node_id: 'node-1',
          target_node_id: 'node-2',
          relationship_type: 'RELATES_TO',
          confidence: 0.5,
          metadata_json: null
        };

        expect(rel.metadata_json).toBeNull();
      });
    });
  });

  describe('Client configuration types', () => {
    describe('HolochainClientConfig', () => {
      it('should accept local config', () => {
        const config: HolochainClientConfig = {
          adminUrl: 'ws://localhost:4444',
          appId: 'elohim'
        };

        expect(config.adminUrl).toContain('localhost');
      });

      it('should accept remote config with happ path', () => {
        const config: HolochainClientConfig = {
          adminUrl: 'wss://holochain-dev.elohim.host',
          appId: 'elohim',
          happPath: '/path/to/elohim.happ'
        };

        expect(config.happPath).toBeDefined();
      });
    });

    describe('HolochainImportConfig', () => {
      it('should accept import config', () => {
        const config: HolochainImportConfig = {
          adminUrl: 'ws://localhost:4444',
          appId: 'elohim',
          batchSize: 100
        };

        expect(config.batchSize).toBe(100);
      });
    });

    describe('HolochainImportResult', () => {
      it('should accept successful result', () => {
        const result: HolochainImportResult = {
          totalNodes: 100,
          createdNodes: 100,
          errors: [],
          importId: 'import-2026-02-01',
          durationMs: 5000
        };

        expect(result.createdNodes).toBe(result.totalNodes);
        expect(result.errors).toHaveLength(0);
      });

      it('should accept result with errors', () => {
        const result: HolochainImportResult = {
          totalNodes: 100,
          createdNodes: 95,
          errors: ['Error 1', 'Error 2', 'Error 3', 'Error 4', 'Error 5'],
          importId: 'import-123'
        };

        expect(result.errors).toHaveLength(5);
        expect(result.createdNodes).toBeLessThan(result.totalNodes);
      });
    });

    describe('HolochainVerifyResult', () => {
      it('should accept verify result', () => {
        const result: HolochainVerifyResult = {
          found: ['node-1', 'node-2', 'node-3'],
          missing: ['node-4', 'node-5']
        };

        expect(result.found).toHaveLength(3);
        expect(result.missing).toHaveLength(2);
      });
    });
  });

  describe('Zome call types', () => {
    describe('ZomeCallInput', () => {
      it('should accept zome call input', () => {
        const input: ZomeCallInput = {
          zomeName: 'content_store',
          fnName: 'get_content_by_id',
          payload: { id: 'epic-governance' }
        };

        expect(input.zomeName).toBe('content_store');
        expect(input.fnName).toBe('get_content_by_id');
      });
    });

    describe('ZomeCallResult', () => {
      it('should accept successful result', () => {
        const result: ZomeCallResult<HolochainContent> = {
          success: true,
          data: {
            id: 'epic-governance',
            content_type: 'epic',
            title: 'Governance',
            description: 'Description',
            content: 'Content',
            content_format: 'markdown',
            tags: [],
            source_path: null,
            related_node_ids: [],
            author_id: null,
            reach: 'commons',
            trust_score: 0,
            metadata_json: '{}',
            created_at: '2026-02-01T12:00:00.000Z',
            updated_at: '2026-02-01T12:00:00.000Z'
          }
        };

        expect(result.success).toBe(true);
        expect(result.data).toBeDefined();
        expect(result.error).toBeUndefined();
      });

      it('should accept error result', () => {
        const result: ZomeCallResult<never> = {
          success: false,
          error: 'Content not found'
        };

        expect(result.success).toBe(false);
        expect(result.error).toBe('Content not found');
        expect(result.data).toBeUndefined();
      });
    });
  });
});
