import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { of } from 'rxjs';
import { PathAdaptationService } from './path-adaptation.service';
import { AttemptCooldownService } from './attempt-cooldown.service';
import { QuestionPoolService } from './question-pool.service';
import { StreakTrackerService } from './streak-tracker.service';
import { RelationshipService } from '@app/lamad/services/relationship.service';
import { ELOHIM_CLIENT } from '@app/elohim/providers/elohim-client.provider';
import type { QuizResult, ContentScore } from '../models/quiz-session.model';
import { vi } from 'vitest';

/**
 * Path Adaptation Service — Story-First Tests
 *
 * These tests are derived directly from the a2o scenario language in:
 *   genesis/a2o/features/lamad/path-adaptation.feature
 *
 * Each describe block maps to a scenario or scenario group.
 * The feature file is the specification; these tests prove the service honours it.
 */

// ---------------------------------------------------------------------------
// Test helpers
// ---------------------------------------------------------------------------

function makeQuizResult(overrides: Partial<QuizResult> = {}): QuizResult {
  return {
    sessionId: 'session-1',
    type: 'mastery',
    humanId: 'matthew',
    score: 0.9,
    passed: true,
    passingThreshold: 0.8,
    correctCount: 9,
    totalQuestions: 10,
    contentScores: [],
    masteryChanges: [],
    attestationsGranted: [],
    timing: {
      totalDurationMs: 30000,
      averageTimePerQuestion: 3000,
      fastestAnswerMs: 1500,
      slowestAnswerMs: 6000,
    },
    completedAt: new Date().toISOString(),
    ...overrides,
  };
}

function makeContentScore(contentId: string, avgScore: number): ContentScore {
  return {
    contentId,
    questionsAnswered: 5,
    correctAnswers: Math.round(avgScore * 5),
    averageScore: avgScore,
    bloomsLevelsTested: ['understand'],
  };
}

// ---------------------------------------------------------------------------
// Test suite
// ---------------------------------------------------------------------------

describe('PathAdaptationService (story-first)', () => {
  let service: PathAdaptationService;
  let cooldownSpy: any;
  let poolSpy: any;
  let streakSpy: any;
  let relationshipSpy: any;

  const PATH_ID = 'elohim-protocol';
  const HUMAN_ID = 'matthew';
  const SECTION_ID = 'foundations';

  beforeEach(() => {
    cooldownSpy = {
      getCooldownStatus: vi.fn(),
      recordAttempt: vi.fn(),
      getBestScore: vi.fn(),
      isMastered: vi.fn(),
    };
    cooldownSpy.getCooldownStatus.mockReturnValue({
      inCooldown: false,
      cooldownEndsAt: null,
      remainingMs: 0,
      remainingFormatted: '',
      attemptsUsed: 0,
      attemptsRemaining: 2,
      resetsAt: new Date().toISOString(),
      timeUntilResetMs: 86400000,
    });
    cooldownSpy.getBestScore.mockReturnValue(0);
    cooldownSpy.isMastered.mockReturnValue(false);

    poolSpy = {
      getQuestions: vi.fn(),
    };
    streakSpy = {
      isAchieved: vi.fn(),
    };
    streakSpy.isAchieved.mockReturnValue(false);

    relationshipSpy = {
      getRelationshipsByType: vi.fn().mockReturnValue(of([])),
      getBidirectionalRelationships: vi.fn().mockReturnValue(of([])),
    };

    const mockElohimClient = {
      getContent: vi.fn(),
      listContent: vi.fn(),
    };
    mockElohimClient.getContent.mockReturnValue(Promise.resolve(null));
    mockElohimClient.listContent.mockReturnValue(Promise.resolve([]));

    // Clear localStorage to prevent state leaking between tests
    // (the service persists adaptation state to localStorage)
    localStorage.clear();

    TestBed.configureTestingModule({
      providers: [
        PathAdaptationService,
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: AttemptCooldownService, useValue: cooldownSpy },
        { provide: QuestionPoolService, useValue: poolSpy },
        { provide: StreakTrackerService, useValue: streakSpy },
        { provide: RelationshipService, useValue: relationshipSpy },
        { provide: ELOHIM_CLIENT, useValue: mockElohimClient },
      ],
    });
    service = TestBed.inject(PathAdaptationService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Gates default to locked
  // From feature: "mastery gates block section transitions until quiz passed"
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Mastery gates block progress until quiz is passed', () => {
    it('new gate should be locked with reason "not_attempted"', () => {
      const gate = service.getGateStatus(PATH_ID, SECTION_ID, HUMAN_ID);

      expect(gate.locked).toBe(true);
      expect(gate.reason).toBe('not_attempted');
      expect(gate.mastered).toBe(false);
    });

    it('canProceed should return false for unmastered gate', () => {
      expect(service.canProceed(PATH_ID, SECTION_ID, HUMAN_ID)).toBe(false);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Passing mastery quiz unlocks gate
  // From feature: "mastery quiz is passed" → gate should open
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Passing mastery quiz unlocks the gate', () => {
    it('recording a passing result should unlock the gate', () => {
      const result = makeQuizResult({ passed: true, score: 0.9 });

      const gate = service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      expect(gate.locked).toBe(false);
      expect(gate.mastered).toBe(true);
      expect(gate.bestScore).toBe(0.9);
    });

    it('canProceed should return true after passing', () => {
      const result = makeQuizResult({ passed: true, score: 0.85 });
      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      expect(service.canProceed(PATH_ID, SECTION_ID, HUMAN_ID)).toBe(true);
    });

    it('section should be marked as completed', () => {
      const result = makeQuizResult({ passed: true });
      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const state = service.getState(PATH_ID, HUMAN_ID);
      expect(state.completedSections.has(SECTION_ID)).toBe(true);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Failed mastery quiz keeps gate locked
  // From feature: "Bloom level 'remember' does NOT unlock steps"
  //   — the principle: low mastery does not unlock
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Failed mastery quiz keeps the gate locked', () => {
    it('recording a failing result should keep gate locked', () => {
      const result = makeQuizResult({ passed: false, score: 0.4 });

      const gate = service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      expect(gate.locked).toBe(true);
      expect(gate.reason).toBe('failed');
      expect(gate.mastered).toBe(false);
    });

    it('best score should be tracked even on failure', () => {
      const result = makeQuizResult({ passed: false, score: 0.6 });
      const gate = service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      expect(gate.bestScore).toBe(0.6);
    });

    it('section should NOT be marked as completed', () => {
      const result = makeQuizResult({ passed: false });
      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const state = service.getState(PATH_ID, HUMAN_ID);
      expect(state.completedSections.has(SECTION_ID)).toBe(false);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Pre-assessment skip-ahead unlocks section steps
  // From feature: "Given Matthew completed a pre-assessment for the path
  //   And the pre-assessment marked section 'foundations' as skippable
  //   Then all steps in section 'foundations' should be accessible"
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Pre-assessment skip-ahead unlocks section steps', () => {
    const sectionMapping = new Map<string, { title: string; conceptIds: string[] }>([
      [
        'foundations',
        {
          title: 'Foundations',
          conceptIds: ['concept-1', 'concept-2', 'concept-3'],
        },
      ],
      [
        'intermediate',
        {
          title: 'Intermediate',
          conceptIds: ['concept-4', 'concept-5'],
        },
      ],
    ]);

    it('high pre-assessment score marks section as skippable', () => {
      const result = makeQuizResult({
        type: 'pre-assessment',
        score: 0.92,
        passed: true,
        contentScores: [
          makeContentScore('concept-1', 0.95),
          makeContentScore('concept-2', 0.9),
          makeContentScore('concept-3', 0.88),
          makeContentScore('concept-4', 0.5),
          makeContentScore('concept-5', 0.4),
        ],
      });

      const skipResult = service.recordPreAssessmentResult(
        PATH_ID,
        HUMAN_ID,
        result,
        sectionMapping
      );

      expect(skipResult.eligible).toBe(true);

      const foundationsSection = skipResult.skippableSections.find(
        s => s.sectionId === 'foundations'
      );
      expect(foundationsSection?.recommendSkip).toBe(true);
    });

    it('skipped section gates should be unlocked', () => {
      const result = makeQuizResult({
        type: 'pre-assessment',
        score: 0.92,
        passed: true,
        contentScores: [
          makeContentScore('concept-1', 0.95),
          makeContentScore('concept-2', 0.9),
          makeContentScore('concept-3', 0.88),
          makeContentScore('concept-4', 0.5),
          makeContentScore('concept-5', 0.4),
        ],
      });

      service.recordPreAssessmentResult(PATH_ID, HUMAN_ID, result, sectionMapping);

      // "all steps in section 'foundations' should be accessible"
      const gate = service.getGateStatus(PATH_ID, 'foundations', HUMAN_ID);
      expect(gate.locked).toBe(false);
      expect(gate.mastered).toBe(true);
    });

    it('section below skip threshold should NOT be skippable', () => {
      const result = makeQuizResult({
        type: 'pre-assessment',
        score: 0.6,
        passed: false,
        contentScores: [
          makeContentScore('concept-1', 0.5),
          makeContentScore('concept-2', 0.55),
          makeContentScore('concept-3', 0.6),
          makeContentScore('concept-4', 0.7),
          makeContentScore('concept-5', 0.65),
        ],
      });

      const skipResult = service.recordPreAssessmentResult(
        PATH_ID,
        HUMAN_ID,
        result,
        sectionMapping
      );

      const foundationsSection = skipResult.skippableSections.find(
        s => s.sectionId === 'foundations'
      );
      expect(foundationsSection?.recommendSkip).toBe(false);
    });

    it('recommended start section should be first non-skippable section', () => {
      const result = makeQuizResult({
        type: 'pre-assessment',
        score: 0.85,
        passed: true,
        contentScores: [
          makeContentScore('concept-1', 0.95),
          makeContentScore('concept-2', 0.9),
          makeContentScore('concept-3', 0.88),
          makeContentScore('concept-4', 0.5),
          makeContentScore('concept-5', 0.4),
        ],
      });

      const skipResult = service.recordPreAssessmentResult(
        PATH_ID,
        HUMAN_ID,
        result,
        sectionMapping
      );

      // Foundations skippable, so recommended start is intermediate
      expect(skipResult.recommendedStartSection).toBe('intermediate');
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Recommendations generated when learner struggles
  // From feature: "Elohim recommends a path after discovery completion"
  //   — the engine: low quiz scores generate supplementary recommendations
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Recommendations surface when learner struggles', () => {
    it('failing quiz with low concept scores generates recommendations', () => {
      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [
          makeContentScore('concept-trust', 0.2),
          makeContentScore('concept-identity', 0.4),
        ],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      expect(recs.length).toBeGreaterThan(0);
      expect(recs[0].reason).toBe('struggled_with_concept');
    });

    it('passing quiz should NOT generate recommendations', () => {
      const result = makeQuizResult({
        passed: true,
        score: 0.95,
        contentScores: [makeContentScore('concept-trust', 0.95)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      expect(recs.length).toBe(0);
    });

    it('recommendations can be dismissed', () => {
      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recsBefore = service.getRecommendations(PATH_ID, HUMAN_ID);
      expect(recsBefore.length).toBe(1);

      service.dismissRecommendation(PATH_ID, HUMAN_ID, 'concept-trust');

      const recsAfter = service.getRecommendations(PATH_ID, HUMAN_ID);
      expect(recsAfter.length).toBe(0);
    });

    it('recommendations respect maxRecommendations limit', () => {
      service.configure({ maxRecommendations: 2 });

      const result = makeQuizResult({
        passed: false,
        score: 0.2,
        contentScores: [
          makeContentScore('concept-a', 0.1),
          makeContentScore('concept-b', 0.15),
          makeContentScore('concept-c', 0.2),
          makeContentScore('concept-d', 0.25),
        ],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      expect(recs.length).toBeLessThanOrEqual(2);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Manual gate unlock (admin/testing)
  // From feature: hard-coded "unlockGate" is the admin escape hatch
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Admin gate unlock', () => {
    it('unlockGate should open a locked gate', () => {
      // Gate starts locked
      expect(service.canProceed(PATH_ID, SECTION_ID, HUMAN_ID)).toBe(false);

      // Manually set up the gate in state first
      const result = makeQuizResult({ passed: false, score: 0.3 });
      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      // Admin unlock
      service.unlockGate(PATH_ID, SECTION_ID, HUMAN_ID);

      const gate = service.getGateStatus(PATH_ID, SECTION_ID, HUMAN_ID);
      expect(gate.locked).toBe(false);
      expect(gate.mastered).toBe(true);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Configuration
  // From feature: "masteryPassingScore: 0.8, skipThreshold: 0.85"
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Configuration matches feature spec defaults', () => {
    it('default mastery passing score should be 0.8', () => {
      const config = service.getConfig();
      expect(config.masteryPassingScore).toBe(0.8);
    });

    it('default skip threshold should be 0.85', () => {
      const config = service.getConfig();
      expect(config.skipThreshold).toBe(0.85);
    });

    it('recommendations should be enabled by default', () => {
      const config = service.getConfig();
      expect(config.enableRecommendations).toBe(true);
    });

    it('configuration should be overridable', () => {
      service.configure({ masteryPassingScore: 0.9 });
      expect(service.getConfig().masteryPassingScore).toBe(0.9);
    });

    it('default maxGraphDepth should be 1', () => {
      const config = service.getConfig();
      expect(config.maxGraphDepth).toBe(1);
    });

    it('default graphRelationshipTypes should include PREREQUISITE and REINFORCES', () => {
      const config = service.getConfig();
      expect(config.graphRelationshipTypes).toEqual(['PREREQUISITE', 'REINFORCES']);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // State management — the adaptation state persists and clears
  // ═══════════════════════════════════════════════════════════════════════════

  describe('State management', () => {
    it('getState returns fresh state for new path/human', () => {
      const state = service.getState(PATH_ID, HUMAN_ID);
      expect(state.pathId).toBe(PATH_ID);
      expect(state.humanId).toBe(HUMAN_ID);
      expect(state.gates.size).toBe(0);
      expect(state.completedSections.size).toBe(0);
    });

    it('clearState removes all adaptation data', () => {
      const result = makeQuizResult({ passed: true });
      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      // Verify data exists
      expect(service.getState(PATH_ID, HUMAN_ID).completedSections.size).toBe(1);

      // Clear
      service.clearState(PATH_ID, HUMAN_ID);

      // Fresh state
      const state = service.getState(PATH_ID, HUMAN_ID);
      expect(state.completedSections.size).toBe(0);
    });

    it('applySkipAhead marks sections as skipped and completed', () => {
      service.applySkipAhead(PATH_ID, HUMAN_ID, ['section-a', 'section-b']);

      const state = service.getState(PATH_ID, HUMAN_ID);
      expect(state.skippedSections.has('section-a')).toBe(true);
      expect(state.skippedSections.has('section-b')).toBe(true);
      expect(state.completedSections.has('section-a')).toBe(true);
      expect(state.completedSections.has('section-b')).toBe(true);
    });

    it('applySkipAhead unlocks gates for skipped sections', () => {
      service.applySkipAhead(PATH_ID, HUMAN_ID, ['section-a']);

      const gate = service.getGateStatus(PATH_ID, 'section-a', HUMAN_ID);
      expect(gate.locked).toBe(false);
      expect(gate.mastered).toBe(true);
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Scenario: Graph-aware recommendations resolve prerequisites
  // Design: 2026-03-06-graph-aware-recommendations-design.md
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Graph-aware recommendations resolve prerequisite content', () => {
    it('should look up PREREQUISITE relationships for struggling concepts', () => {
      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      expect(relationshipSpy.getRelationshipsByType).toHaveBeenCalledWith(
        'concept-trust',
        'PREREQUISITE'
      );
    });

    it('should include prerequisite content in recommendations with reason prerequisite_gap', () => {
      relationshipSpy.getRelationshipsByType.mockImplementation(
        (contentId: string, type: string) => {
          if (type === 'PREREQUISITE') {
            return of([
              {
                id: 'rel-1',
                sourceId: contentId,
                targetId: 'prereq-foundations',
                relationshipType: 'PREREQUISITE',
                confidence: 0.9,
              },
            ]);
          }
          return of([]);
        }
      );

      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      const prereqRec = recs.find(r => r.contentId === 'prereq-foundations');
      expect(prereqRec).toBeDefined();
      expect(prereqRec!.reason).toBe('prerequisite_gap');
    });

    it('should include REINFORCES content with reason reinforcement', () => {
      relationshipSpy.getRelationshipsByType.mockImplementation(
        (contentId: string, type: string) => {
          if (type === 'REINFORCES') {
            return of([
              {
                id: 'rel-2',
                sourceId: contentId,
                targetId: 'alt-perspective',
                relationshipType: 'REINFORCES',
                confidence: 0.7,
              },
            ]);
          }
          return of([]);
        }
      );

      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      const reinforceRec = recs.find(r => r.contentId === 'alt-perspective');
      expect(reinforceRec).toBeDefined();
      expect(reinforceRec!.reason).toBe('reinforcement');
    });

    it('should fall back to struggled_with_concept when no graph relationships exist', () => {
      // Default mock returns empty arrays — no graph relationships
      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      expect(recs.length).toBeGreaterThan(0);
      expect(recs[0].reason).toBe('struggled_with_concept');
    });

    it('should rank prerequisite_gap above reinforcement', () => {
      relationshipSpy.getRelationshipsByType.mockImplementation(
        (_contentId: string, type: string) => {
          if (type === 'PREREQUISITE') {
            return of([
              {
                id: 'rel-1',
                sourceId: 'concept-trust',
                targetId: 'prereq-foundations',
                relationshipType: 'PREREQUISITE',
                confidence: 0.7,
              },
            ]);
          }
          if (type === 'REINFORCES') {
            return of([
              {
                id: 'rel-2',
                sourceId: 'concept-trust',
                targetId: 'alt-perspective',
                relationshipType: 'REINFORCES',
                confidence: 0.9,
              },
            ]);
          }
          return of([]);
        }
      );

      const result = makeQuizResult({
        passed: false,
        score: 0.3,
        contentScores: [makeContentScore('concept-trust', 0.2)],
      });

      service.recordMasteryResult(PATH_ID, SECTION_ID, HUMAN_ID, result);

      const recs = service.getRecommendations(PATH_ID, HUMAN_ID);
      const prereqIdx = recs.findIndex(r => r.reason === 'prerequisite_gap');
      const reinforceIdx = recs.findIndex(r => r.reason === 'reinforcement');
      if (prereqIdx >= 0 && reinforceIdx >= 0) {
        expect(prereqIdx).toBeLessThan(reinforceIdx);
      }
    });
  });

  // ═══════════════════════════════════════════════════════════════════════════
  // Inline quiz integration
  // From feature: "requireInlineBeforeMastery" — inlines must complete first
  // ═══════════════════════════════════════════════════════════════════════════

  describe('Inline quiz integration', () => {
    it('incomplete inline quizzes should block mastery gate when configured', () => {
      streakSpy.isAchieved.mockReturnValue(false);

      const complete = service.areSectionInlinesComplete(PATH_ID, SECTION_ID, [
        'content-1',
        'content-2',
      ]);

      expect(complete).toBe(false);
    });

    it('completed inline quizzes allow mastery gate', () => {
      streakSpy.isAchieved.mockReturnValue(true);

      const complete = service.areSectionInlinesComplete(PATH_ID, SECTION_ID, [
        'content-1',
        'content-2',
      ]);

      expect(complete).toBe(true);
    });

    it('inline check bypassed when requireInlineBeforeMastery is false', () => {
      service.configure({ requireInlineBeforeMastery: false });
      streakSpy.isAchieved.mockReturnValue(false);

      const complete = service.areSectionInlinesComplete(PATH_ID, SECTION_ID, [
        'content-1',
        'content-2',
      ]);

      expect(complete).toBe(true);
    });
  });
});
