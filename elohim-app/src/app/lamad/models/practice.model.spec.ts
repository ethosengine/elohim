/**
 * Practice Pool & Mastery Challenge Models - Tests
 *
 * Validates JSON parsing functions and type structures for practice system.
 */

import {
  PracticePool,
  MasteryChallenge,
  DiscoveryCandidate,
  ContentMixEntry,
  ChallengeQuestion,
  MasteryChallengeResponse,
  LevelChange,
  ChallengeDiscovery,
  parseContentMix,
  parseLevelChanges,
  parseQuestions,
  parseResponses,
  parseDiscoveries,
  getActiveContentIds,
  getRefreshQueueIds,
  getDiscoveryCandidates,
  PoolSources,
  MasteryChallengeStates,
} from './practice.model';

describe('Practice Model', () => {
  // =============================================================================
  // Constants
  // =============================================================================

  describe('PoolSources', () => {
    it('should define all pool source types', () => {
      expect(PoolSources.PATH_ACTIVE).toBe('path_active');
      expect(PoolSources.REFRESH_QUEUE).toBe('refresh_queue');
      expect(PoolSources.GRAPH_NEIGHBOR).toBe('graph_neighbor');
      expect(PoolSources.SERENDIPITY).toBe('serendipity');
    });
  });

  describe('MasteryChallengeStates', () => {
    it('should define all challenge states', () => {
      expect(MasteryChallengeStates.IN_PROGRESS).toBe('in_progress');
      expect(MasteryChallengeStates.COMPLETED).toBe('completed');
      expect(MasteryChallengeStates.ABANDONED).toBe('abandoned');
    });
  });

  // =============================================================================
  // Parse Content Mix
  // =============================================================================

  describe('parseContentMix', () => {
    it('should parse valid content mix JSON', () => {
      const json = JSON.stringify([
        {
          content_id: 'content-1',
          source: 'path_active',
          question_count: 5,
        },
        {
          content_id: 'content-2',
          source: 'refresh_queue',
          question_count: 3,
        },
      ]);

      const result = parseContentMix(json);

      expect(result).toEqual([
        {
          content_id: 'content-1',
          source: 'path_active',
          question_count: 5,
        },
        {
          content_id: 'content-2',
          source: 'refresh_queue',
          question_count: 3,
        },
      ]);
    });

    it('should return empty array for invalid JSON', () => {
      const result = parseContentMix('invalid json');
      expect(result).toEqual([]);
    });

    it('should return empty array for empty string', () => {
      const result = parseContentMix('');
      expect(result).toEqual([]);
    });

    it('should handle empty array JSON', () => {
      const result = parseContentMix('[]');
      expect(result).toEqual([]);
    });
  });

  // =============================================================================
  // Parse Level Changes
  // =============================================================================

  describe('parseLevelChanges', () => {
    it('should parse valid level changes JSON', () => {
      const json = JSON.stringify([
        {
          content_id: 'content-1',
          from_level: 'remember',
          to_level: 'understand',
          from_index: 2,
          to_index: 3,
          change: 'up',
        },
        {
          content_id: 'content-2',
          from_level: 'understand',
          to_level: 'remember',
          from_index: 3,
          to_index: 2,
          change: 'down',
        },
      ]);

      const result = parseLevelChanges(json);

      expect(result).toEqual([
        {
          content_id: 'content-1',
          from_level: 'remember',
          to_level: 'understand',
          from_index: 2,
          to_index: 3,
          change: 'up',
        },
        {
          content_id: 'content-2',
          from_level: 'understand',
          to_level: 'remember',
          from_index: 3,
          to_index: 2,
          change: 'down',
        },
      ]);
    });

    it('should return empty array for invalid JSON', () => {
      const result = parseLevelChanges('invalid json');
      expect(result).toEqual([]);
    });

    it('should return empty array for empty string', () => {
      const result = parseLevelChanges('');
      expect(result).toEqual([]);
    });

    it('should handle empty array JSON', () => {
      const result = parseLevelChanges('[]');
      expect(result).toEqual([]);
    });
  });

  // =============================================================================
  // Parse Questions
  // =============================================================================

  describe('parseQuestions', () => {
    it('should parse valid questions JSON', () => {
      const json = JSON.stringify([
        {
          content_id: 'content-1',
          question_type: 'multiple_choice',
          question_text: 'What is 2+2?',
          options_json: JSON.stringify(['3', '4', '5']),
          correct_answer: '4',
        },
        {
          content_id: 'content-2',
          question_type: 'true_false',
          question_text: 'The sky is blue.',
          options_json: JSON.stringify(['true', 'false']),
          correct_answer: 'true',
        },
      ]);

      const result = parseQuestions(json);

      expect(result).toEqual([
        {
          content_id: 'content-1',
          question_type: 'multiple_choice',
          question_text: 'What is 2+2?',
          options_json: JSON.stringify(['3', '4', '5']),
          correct_answer: '4',
        },
        {
          content_id: 'content-2',
          question_type: 'true_false',
          question_text: 'The sky is blue.',
          options_json: JSON.stringify(['true', 'false']),
          correct_answer: 'true',
        },
      ]);
    });

    it('should return empty array for invalid JSON', () => {
      const result = parseQuestions('invalid json');
      expect(result).toEqual([]);
    });

    it('should return empty array for empty string', () => {
      const result = parseQuestions('');
      expect(result).toEqual([]);
    });

    it('should handle empty array JSON', () => {
      const result = parseQuestions('[]');
      expect(result).toEqual([]);
    });
  });

  // =============================================================================
  // Parse Responses
  // =============================================================================

  describe('parseResponses', () => {
    it('should parse valid responses JSON', () => {
      const json = JSON.stringify([
        {
          content_id: 'content-1',
          question_index: 0,
          response: '4',
          correct: true,
          time_taken_ms: 1500,
        },
        {
          content_id: 'content-2',
          question_index: 1,
          response: 'false',
          correct: false,
          time_taken_ms: 2300,
        },
      ]);

      const result = parseResponses(json);

      expect(result).toEqual([
        {
          content_id: 'content-1',
          question_index: 0,
          response: '4',
          correct: true,
          time_taken_ms: 1500,
        },
        {
          content_id: 'content-2',
          question_index: 1,
          response: 'false',
          correct: false,
          time_taken_ms: 2300,
        },
      ]);
    });

    it('should return empty array for invalid JSON', () => {
      const result = parseResponses('invalid json');
      expect(result).toEqual([]);
    });

    it('should return empty array for empty string', () => {
      const result = parseResponses('');
      expect(result).toEqual([]);
    });

    it('should handle empty array JSON', () => {
      const result = parseResponses('[]');
      expect(result).toEqual([]);
    });
  });

  // =============================================================================
  // Parse Discoveries
  // =============================================================================

  describe('parseDiscoveries', () => {
    it('should parse valid discoveries JSON', () => {
      const json = JSON.stringify([
        {
          content_id: 'discovered-1',
          discovered_via: 'content-source-1',
          relationship_type: 'related_to',
        },
        {
          content_id: 'discovered-2',
          discovered_via: 'content-source-2',
          relationship_type: 'prerequisite',
        },
      ]);

      const result = parseDiscoveries(json);

      expect(result).toEqual([
        {
          content_id: 'discovered-1',
          discovered_via: 'content-source-1',
          relationship_type: 'related_to',
        },
        {
          content_id: 'discovered-2',
          discovered_via: 'content-source-2',
          relationship_type: 'prerequisite',
        },
      ]);
    });

    it('should return empty array for invalid JSON', () => {
      const result = parseDiscoveries('invalid json');
      expect(result).toEqual([]);
    });

    it('should return empty array for empty string', () => {
      const result = parseDiscoveries('');
      expect(result).toEqual([]);
    });

    it('should handle empty array JSON', () => {
      const result = parseDiscoveries('[]');
      expect(result).toEqual([]);
    });
  });

  // =============================================================================
  // Get Active Content IDs
  // =============================================================================

  describe('getActiveContentIds', () => {
    it('should parse active content IDs from pool', () => {
      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: JSON.stringify(['content-1', 'content-2', 'content-3']),
        refresh_queue_ids_json: '[]',
        discovery_candidates_json: '[]',
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getActiveContentIds(pool);

      expect(result).toEqual(['content-1', 'content-2', 'content-3']);
    });

    it('should return empty array for invalid JSON', () => {
      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: 'invalid json',
        refresh_queue_ids_json: '[]',
        discovery_candidates_json: '[]',
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getActiveContentIds(pool);

      expect(result).toEqual([]);
    });

    it('should handle empty array JSON', () => {
      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: '[]',
        refresh_queue_ids_json: '[]',
        discovery_candidates_json: '[]',
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getActiveContentIds(pool);

      expect(result).toEqual([]);
    });
  });

  // =============================================================================
  // Get Refresh Queue IDs
  // =============================================================================

  describe('getRefreshQueueIds', () => {
    it('should parse refresh queue IDs from pool', () => {
      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: '[]',
        refresh_queue_ids_json: JSON.stringify(['stale-1', 'stale-2']),
        discovery_candidates_json: '[]',
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getRefreshQueueIds(pool);

      expect(result).toEqual(['stale-1', 'stale-2']);
    });

    it('should return empty array for invalid JSON', () => {
      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: '[]',
        refresh_queue_ids_json: 'invalid json',
        discovery_candidates_json: '[]',
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getRefreshQueueIds(pool);

      expect(result).toEqual([]);
    });

    it('should handle empty array JSON', () => {
      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: '[]',
        refresh_queue_ids_json: '[]',
        discovery_candidates_json: '[]',
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getRefreshQueueIds(pool);

      expect(result).toEqual([]);
    });
  });

  // =============================================================================
  // Get Discovery Candidates
  // =============================================================================

  describe('getDiscoveryCandidates', () => {
    it('should parse discovery candidates from pool', () => {
      const candidates: DiscoveryCandidate[] = [
        {
          content_id: 'discovery-1',
          source_content_id: 'source-1',
          relationship_type: 'related_to',
          discovery_reason: 'graph_neighbor',
        },
        {
          content_id: 'discovery-2',
          source_content_id: 'source-2',
          relationship_type: 'prerequisite',
          discovery_reason: 'serendipity',
        },
      ];

      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: '[]',
        refresh_queue_ids_json: '[]',
        discovery_candidates_json: JSON.stringify(candidates),
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getDiscoveryCandidates(pool);

      expect(result).toEqual(candidates);
    });

    it('should return empty array for invalid JSON', () => {
      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: '[]',
        refresh_queue_ids_json: '[]',
        discovery_candidates_json: 'invalid json',
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getDiscoveryCandidates(pool);

      expect(result).toEqual([]);
    });

    it('should handle empty array JSON', () => {
      const pool: PracticePool = {
        id: 'pool-1',
        agent_id: 'agent-1',
        active_content_ids_json: '[]',
        refresh_queue_ids_json: '[]',
        discovery_candidates_json: '[]',
        contributing_path_ids_json: '[]',
        max_active_size: 10,
        refresh_threshold: 0.5,
        discovery_probability: 0.2,
        regression_enabled: true,
        challenge_cooldown_hours: 24,
        last_challenge_at: null,
        last_challenge_id: null,
        total_challenges_taken: 0,
        total_level_ups: 0,
        total_level_downs: 0,
        discoveries_unlocked: 0,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-01T00:00:00Z',
      };

      const result = getDiscoveryCandidates(pool);

      expect(result).toEqual([]);
    });
  });

  // =============================================================================
  // Integration Tests - Complex Scenarios
  // =============================================================================

  describe('Integration Scenarios', () => {
    it('should handle complete practice pool with all fields populated', () => {
      const pool: PracticePool = {
        id: 'pool-complete',
        agent_id: 'agent-123',
        active_content_ids_json: JSON.stringify(['content-1', 'content-2', 'content-3']),
        refresh_queue_ids_json: JSON.stringify(['stale-1', 'stale-2']),
        discovery_candidates_json: JSON.stringify([
          {
            content_id: 'discovery-1',
            source_content_id: 'content-1',
            relationship_type: 'related_to',
            discovery_reason: 'graph_neighbor',
          },
        ]),
        contributing_path_ids_json: JSON.stringify(['path-1', 'path-2']),
        max_active_size: 15,
        refresh_threshold: 0.6,
        discovery_probability: 0.3,
        regression_enabled: false,
        challenge_cooldown_hours: 48,
        last_challenge_at: '2025-01-15T10:30:00Z',
        last_challenge_id: 'challenge-42',
        total_challenges_taken: 10,
        total_level_ups: 8,
        total_level_downs: 2,
        discoveries_unlocked: 5,
        created_at: '2025-01-01T00:00:00Z',
        updated_at: '2025-01-15T10:30:00Z',
      };

      const activeIds = getActiveContentIds(pool);
      const refreshIds = getRefreshQueueIds(pool);
      const discoveries = getDiscoveryCandidates(pool);

      expect(activeIds).toEqual(['content-1', 'content-2', 'content-3']);
      expect(refreshIds).toEqual(['stale-1', 'stale-2']);
      expect(discoveries).toHaveSize(1);
      expect(discoveries[0].content_id).toBe('discovery-1');
    });

    it('should handle complete mastery challenge lifecycle', () => {
      const contentMix: ContentMixEntry[] = [
        { content_id: 'content-1', source: 'path_active', question_count: 3 },
        { content_id: 'content-2', source: 'refresh_queue', question_count: 2 },
        { content_id: 'content-3', source: 'graph_neighbor', question_count: 1 },
      ];

      const questions: ChallengeQuestion[] = [
        {
          content_id: 'content-1',
          question_type: 'multiple_choice',
          question_text: 'Question 1?',
          options_json: JSON.stringify(['A', 'B', 'C']),
          correct_answer: 'B',
        },
        {
          content_id: 'content-1',
          question_type: 'multiple_choice',
          question_text: 'Question 2?',
          options_json: JSON.stringify(['X', 'Y', 'Z']),
          correct_answer: 'Y',
        },
      ];

      const responses: MasteryChallengeResponse[] = [
        { content_id: 'content-1', question_index: 0, response: 'B', correct: true, time_taken_ms: 1500 },
        { content_id: 'content-1', question_index: 1, response: 'Z', correct: false, time_taken_ms: 2000 },
      ];

      const levelChanges: LevelChange[] = [
        {
          content_id: 'content-1',
          from_level: 'remember',
          to_level: 'remember',
          from_index: 2,
          to_index: 2,
          change: 'same',
        },
      ];

      const discoveries: ChallengeDiscovery[] = [
        {
          content_id: 'content-3',
          discovered_via: 'content-1',
          relationship_type: 'related_to',
        },
      ];

      const challenge: MasteryChallenge = {
        id: 'challenge-123',
        agent_id: 'agent-123',
        pool_id: 'pool-1',
        path_id: 'path-1',
        content_mix_json: JSON.stringify(contentMix),
        total_questions: 6,
        discovery_questions: 1,
        state: 'completed',
        started_at: '2025-01-15T10:00:00Z',
        completed_at: '2025-01-15T10:05:00Z',
        time_limit_seconds: 600,
        actual_time_seconds: 300,
        questions_json: JSON.stringify(questions),
        responses_json: JSON.stringify(responses),
        score: 0.5,
        score_by_content_json: JSON.stringify({ 'content-1': 0.5 }),
        level_changes_json: JSON.stringify(levelChanges),
        net_level_change: 0,
        discoveries_json: JSON.stringify(discoveries),
        created_at: '2025-01-15T10:00:00Z',
      };

      const parsedMix = parseContentMix(challenge.content_mix_json);
      const parsedQuestions = parseQuestions(challenge.questions_json);
      const parsedResponses = parseResponses(challenge.responses_json);
      const parsedLevelChanges = parseLevelChanges(challenge.level_changes_json);
      const parsedDiscoveries = parseDiscoveries(challenge.discoveries_json);

      expect(parsedMix).toHaveSize(3);
      expect(parsedQuestions).toHaveSize(2);
      expect(parsedResponses).toHaveSize(2);
      expect(parsedLevelChanges).toHaveSize(1);
      expect(parsedDiscoveries).toHaveSize(1);

      expect(parsedMix[0].source).toBe('path_active');
      expect(parsedQuestions[0].question_text).toBe('Question 1?');
      expect(parsedResponses[0].correct).toBe(true);
      expect(parsedLevelChanges[0].change).toBe('same');
      expect(parsedDiscoveries[0].content_id).toBe('content-3');
    });
  });
});
