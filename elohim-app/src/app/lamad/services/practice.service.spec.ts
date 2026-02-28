import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { firstValueFrom } from 'rxjs';

import { LearnerBackendService } from '@app/elohim/services/learner-backend.service';

import { PracticeService } from './practice.service';

import type {
  PracticePool,
  MasteryChallenge,
  ChallengeResult,
  CooldownCheckResult,
  PoolRecommendations,
  MasteryChallengeResponse,
} from '../models/practice.model';
import { vi } from 'vitest';

describe('PracticeService', () => {
  let service: PracticeService;
  let mockBackend: any;

  // ===========================================================================
  // Fixture Builders
  // ===========================================================================

  const buildPool = (overrides: Partial<PracticePool> = {}): PracticePool => ({
    id: 'pool-1',
    agent_id: 'agent-1',
    active_content_ids_json: JSON.stringify(['content-1', 'content-2', 'content-3']),
    refresh_queue_ids_json: JSON.stringify(['stale-1', 'stale-2']),
    discovery_candidates_json: JSON.stringify([
      {
        content_id: 'disc-1',
        source_content_id: 'content-1',
        relationship_type: 'related_to',
        discovery_reason: 'graph_neighbor',
      },
    ]),
    contributing_path_ids_json: JSON.stringify(['path-1']),
    max_active_size: 10,
    refresh_threshold: 0.5,
    discovery_probability: 0.2,
    regression_enabled: true,
    challenge_cooldown_hours: 24,
    last_challenge_at: '2026-01-15T10:00:00Z',
    last_challenge_id: 'challenge-prev',
    total_challenges_taken: 10,
    total_level_ups: 5,
    total_level_downs: 2,
    discoveries_unlocked: 3,
    created_at: '2026-01-01T00:00:00Z',
    updated_at: '2026-01-15T10:00:00Z',
    ...overrides,
  });

  const buildChallenge = (overrides: Partial<MasteryChallenge> = {}): MasteryChallenge => ({
    id: 'challenge-1',
    agent_id: 'agent-1',
    pool_id: 'pool-1',
    path_id: 'path-1',
    content_mix_json: JSON.stringify([
      { content_id: 'content-1', source: 'path_active', question_count: 3 },
    ]),
    total_questions: 5,
    discovery_questions: 1,
    state: 'in_progress',
    started_at: '2026-02-20T10:00:00Z',
    completed_at: null,
    time_limit_seconds: 600,
    actual_time_seconds: null,
    questions_json: '[]',
    responses_json: '[]',
    score: null,
    score_by_content_json: '{}',
    level_changes_json: '[]',
    net_level_change: 0,
    discoveries_json: '[]',
    created_at: '2026-02-20T10:00:00Z',
    ...overrides,
  });

  const buildCooldown = (overrides: Partial<CooldownCheckResult> = {}): CooldownCheckResult => ({
    can_take_challenge: true,
    cooldown_remaining_hours: 0,
    last_challenge_at: '2026-02-19T10:00:00Z',
    next_available_at: null,
    ...overrides,
  });

  const buildRecommendations = (
    overrides: Partial<PoolRecommendations> = {}
  ): PoolRecommendations => ({
    priority_refresh: ['stale-1'],
    active_practice: ['content-1', 'content-2'],
    discovery_suggestions: [],
    total_pool_size: 6,
    ...overrides,
  });

  const buildChallengeResult = (overrides: Partial<ChallengeResult> = {}): ChallengeResult => ({
    challenge: {
      action_hash: new Uint8Array([1, 2, 3]),
      challenge: buildChallenge({
        state: 'completed',
        score: 0.8,
        completed_at: '2026-02-20T10:05:00Z',
      }),
    },
    score: 0.8,
    level_changes: [],
    discoveries: [],
    net_level_change: 1,
    can_retake_at: '2026-02-21T10:00:00Z',
    ...overrides,
  });

  // Helper: initialize pool state through service method
  async function setPoolState(pool: PracticePool = buildPool()): Promise<void> {
    mockBackend.getOrCreatePracticePool.mockReturnValue(
      Promise.resolve({ action_hash: new Uint8Array(), pool })
    );
    await firstValueFrom(service.initializePool(['path-1']));
  }

  // ===========================================================================
  // Setup
  // ===========================================================================

  beforeEach(() => {
    mockBackend = {
    getOrCreatePracticePool: vi.fn(),
    addPathToPool: vi.fn(),
    refreshPracticePool: vi.fn(),
    getPoolRecommendations: vi.fn(),
    checkChallengeCooldown: vi.fn(),
    startMasteryChallenge: vi.fn(),
    submitMasteryChallenge: vi.fn(),
    getChallengeHistory: vi.fn(),
  };

    // Default return values for fire-and-forget side-effect methods
    mockBackend.getPoolRecommendations.mockReturnValue(Promise.resolve(null));
    mockBackend.checkChallengeCooldown.mockReturnValue(Promise.resolve(null));
    mockBackend.refreshPracticePool.mockReturnValue(Promise.resolve(null));

    TestBed.configureTestingModule({
      providers: [PracticeService, { provide: LearnerBackendService, useValue: mockBackend }],
    });
    service = TestBed.inject(PracticeService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // ===========================================================================
  // Pool Management
  // ===========================================================================

  describe('Pool Management', () => {
    describe('initializePool()', () => {
      it('should call backend with contributing_path_ids', async () => {
        mockBackend.getOrCreatePracticePool.mockReturnValue(Promise.resolve(null));
        await firstValueFrom(service.initializePool(['path-1', 'path-2']));
        expect(mockBackend.getOrCreatePracticePool).toHaveBeenCalledWith({
          contributing_path_ids: ['path-1', 'path-2'],
        });
      });

      it('should update pool$ when backend returns pool', async () => {
        const pool = buildPool();
        mockBackend.getOrCreatePracticePool.mockReturnValue(
          Promise.resolve({ action_hash: new Uint8Array(), pool })
        );
        const result = await firstValueFrom(service.initializePool(['path-1']));
        expect(result).toEqual(pool);
        expect(service.getPoolSync()).toEqual(pool);
      });

      it('should trigger refreshRecommendations and checkCooldown on success', fakeAsync(() => {
        const pool = buildPool();
        mockBackend.getOrCreatePracticePool.mockReturnValue(
          Promise.resolve({ action_hash: new Uint8Array(), pool })
        );
        service.initializePool(['path-1']).subscribe();
        tick();
        expect(mockBackend.getPoolRecommendations).toHaveBeenCalled();
        expect(mockBackend.checkChallengeCooldown).toHaveBeenCalled();
      }));

      it('should NOT trigger side effects when backend returns null', fakeAsync(() => {
        mockBackend.getOrCreatePracticePool.mockReturnValue(Promise.resolve(null));
        // Reset spy call counts since defaults are set in beforeEach
        mockBackend.getPoolRecommendations.mockClear();
        mockBackend.checkChallengeCooldown.mockClear();
        service.initializePool(['path-1']).subscribe();
        tick();
        expect(mockBackend.getPoolRecommendations).not.toHaveBeenCalled();
        expect(mockBackend.checkChallengeCooldown).not.toHaveBeenCalled();
      }));

      it('should return null on backend error', async () => {
        mockBackend.getOrCreatePracticePool.mockReturnValue(
          Promise.reject(new Error('network error'))
        );
        const result = await firstValueFrom(service.initializePool(['path-1']));
        expect(result).toBeNull();
      });
    });

    describe('addPathToPool()', () => {
      it('should update pool$ and trigger refreshRecommendations on success', fakeAsync(() => {
        const pool = buildPool();
        mockBackend.addPathToPool.mockReturnValue(
          Promise.resolve({ action_hash: new Uint8Array(), pool })
        );
        mockBackend.getPoolRecommendations.mockClear();
        service.addPathToPool('new-path').subscribe();
        tick();
        expect(service.getPoolSync()).toEqual(pool);
        expect(mockBackend.getPoolRecommendations).toHaveBeenCalled();
      }));

      it('should not update pool$ when backend returns null', async () => {
        mockBackend.addPathToPool.mockReturnValue(Promise.resolve(null));
        await firstValueFrom(service.addPathToPool('path-1'));
        expect(service.getPoolSync()).toBeNull();
      });

      it('should return null on error', async () => {
        mockBackend.addPathToPool.mockReturnValue(Promise.reject(new Error('fail')));
        const result = await firstValueFrom(service.addPathToPool('path-1'));
        expect(result).toBeNull();
      });
    });

    describe('refreshPool()', () => {
      it('should update pool$ on success', async () => {
        const pool = buildPool({ total_challenges_taken: 20 });
        mockBackend.refreshPracticePool.mockReturnValue(
          Promise.resolve({ action_hash: new Uint8Array(), pool })
        );
        const result = await firstValueFrom(service.refreshPool());
        expect(result).toEqual(pool);
        expect(service.getPoolSync()).toEqual(pool);
      });

      it('should return null on error', async () => {
        mockBackend.refreshPracticePool.mockReturnValue(Promise.reject(new Error('fail')));
        const result = await firstValueFrom(service.refreshPool());
        expect(result).toBeNull();
      });
    });

    describe('getPoolSync()', () => {
      it('should return null initially', () => {
        expect(service.getPoolSync()).toBeNull();
      });
    });
  });

  // ===========================================================================
  // Recommendations
  // ===========================================================================

  describe('Recommendations', () => {
    describe('refreshRecommendations()', () => {
      it('should update recommendations$ from backend', fakeAsync(() => {
        const recs = buildRecommendations();
        mockBackend.getPoolRecommendations.mockReturnValue(Promise.resolve(recs));
        service.refreshRecommendations();
        tick();
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let latest: any = null;
        service.recommendations$.subscribe(r => (latest = r));
        expect(latest).toEqual(recs);
      }));

      it('should set recommendations$ to null on backend error', fakeAsync(() => {
        mockBackend.getPoolRecommendations.mockReturnValue(Promise.reject(new Error('fail')));
        service.refreshRecommendations();
        tick();
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        let latest: any = 'sentinel';
        service.recommendations$.subscribe(r => (latest = r));
        expect(latest).toBeNull();
      }));
    });

    describe('getRecommendations$()', () => {
      it('should trigger refresh when no cached value exists', fakeAsync(() => {
        mockBackend.getPoolRecommendations.mockClear();
        service.getRecommendations$();
        tick();
        expect(mockBackend.getPoolRecommendations).toHaveBeenCalled();
      }));

      it('should NOT trigger refresh when cached value exists', fakeAsync(() => {
        const recs = buildRecommendations();
        mockBackend.getPoolRecommendations.mockReturnValue(Promise.resolve(recs));
        service.refreshRecommendations();
        tick();
        mockBackend.getPoolRecommendations.mockClear();
        service.getRecommendations$();
        expect(mockBackend.getPoolRecommendations).not.toHaveBeenCalled();
      }));
    });
  });

  // ===========================================================================
  // Challenge Cooldown
  // ===========================================================================

  describe('Challenge Cooldown', () => {
    describe('checkCooldown()', () => {
      it('should update cooldown$ from backend', fakeAsync(() => {
        const cooldown = buildCooldown();
        mockBackend.checkChallengeCooldown.mockReturnValue(Promise.resolve(cooldown));
        service.checkCooldown();
        tick();
        expect(service.getCooldownSync()).toEqual(cooldown);
      }));

      it('should handle backend error gracefully', fakeAsync(() => {
        mockBackend.checkChallengeCooldown.mockReturnValue(Promise.reject(new Error('fail')));
        service.checkCooldown();
        tick();
        // Should not throw; cooldown stays at initial null
        expect(service.getCooldownSync()).toBeNull();
      }));
    });

    describe('canTakeChallenge$()', () => {
      it('should emit true when cooldown allows', fakeAsync(() => {
        mockBackend.checkChallengeCooldown.mockReturnValue(
          Promise.resolve(buildCooldown({ can_take_challenge: true }))
        );
        service.checkCooldown();
        tick();
        let canTake = false;
        service.canTakeChallenge$().subscribe(v => (canTake = v));
        expect(canTake).toBe(true);
      }));

      it('should emit false when cooldown blocks', fakeAsync(() => {
        mockBackend.checkChallengeCooldown.mockReturnValue(
          Promise.resolve(buildCooldown({ can_take_challenge: false, cooldown_remaining_hours: 12 }))
        );
        service.checkCooldown();
        tick();
        let canTake = true;
        service.canTakeChallenge$().subscribe(v => (canTake = v));
        expect(canTake).toBe(false);
      }));

      it('should emit false when cooldown is null', () => {
        let canTake = true;
        service.canTakeChallenge$().subscribe(v => (canTake = v));
        expect(canTake).toBe(false);
      });
    });
  });

  // ===========================================================================
  // Challenge Flow
  // ===========================================================================

  describe('Challenge Flow', () => {
    describe('startChallenge()', () => {
      it('should return null immediately when cooldown blocks', fakeAsync(() => {
        mockBackend.checkChallengeCooldown.mockReturnValue(
          Promise.resolve(buildCooldown({ can_take_challenge: false }))
        );
        service.checkCooldown();
        tick();
        let result: MasteryChallenge | null = 'sentinel' as unknown as MasteryChallenge;
        service.startChallenge(10).subscribe(v => (result = v));
        tick();
        expect(result).toBeNull();
        expect(mockBackend.startMasteryChallenge).not.toHaveBeenCalled();
      }));

      it('should proceed when cooldown is null (no data yet)', fakeAsync(() => {
        const challenge = buildChallenge();
        mockBackend.startMasteryChallenge.mockReturnValue(
          Promise.resolve({ action_hash: new Uint8Array(), challenge })
        );
        service.startChallenge(10).subscribe();
        tick();
        expect(mockBackend.startMasteryChallenge).toHaveBeenCalled();
      }));

      it('should call backend with correct StartChallengeInput', fakeAsync(() => {
        mockBackend.startMasteryChallenge.mockReturnValue(Promise.resolve(null));
        service.startChallenge(10, 'path-1', false, 600).subscribe();
        tick();
        expect(mockBackend.startMasteryChallenge).toHaveBeenCalledWith({
          question_count: 10,
          path_id: 'path-1',
          include_discoveries: false,
          time_limit_seconds: 600,
        });
      }));

      it('should default include_discoveries to true', fakeAsync(() => {
        mockBackend.startMasteryChallenge.mockReturnValue(Promise.resolve(null));
        service.startChallenge(5).subscribe();
        tick();
        const callArgs = mockBackend.startMasteryChallenge.mock.lastCall[0];
        expect(callArgs.include_discoveries).toBe(true);
      }));

      it('should update currentChallenge$ with unwrapped challenge', async () => {
        const challenge = buildChallenge();
        mockBackend.startMasteryChallenge.mockReturnValue(
          Promise.resolve({ action_hash: new Uint8Array(), challenge })
        );
        const result = await firstValueFrom(service.startChallenge(10));
        expect(result).toEqual(challenge);
        expect(service.getCurrentChallengeSync()).toEqual(challenge);
      });

      it('should return null on backend error', async () => {
        mockBackend.startMasteryChallenge.mockReturnValue(Promise.reject(new Error('fail')));
        const result = await firstValueFrom(service.startChallenge(10));
        expect(result).toBeNull();
      });
    });

    describe('submitChallenge()', () => {
      const responses: MasteryChallengeResponse[] = [
        {
          content_id: 'content-1',
          question_index: 0,
          response: 'answer',
          correct: true,
          time_taken_ms: 5000,
        },
      ];

      it('should call backend with correct SubmitChallengeInput', fakeAsync(() => {
        mockBackend.submitMasteryChallenge.mockReturnValue(
          Promise.resolve(buildChallengeResult())
        );
        service.submitChallenge('challenge-1', responses, 300).subscribe();
        tick();
        expect(mockBackend.submitMasteryChallenge).toHaveBeenCalledWith({
          challenge_id: 'challenge-1',
          responses,
          actual_time_seconds: 300,
        });
      }));

      it('should clear currentChallenge$ on success', fakeAsync(async () => {
        // Set a current challenge first
        const challenge = buildChallenge();
        mockBackend.startMasteryChallenge.mockReturnValue(
          Promise.resolve({ action_hash: new Uint8Array(), challenge })
        );
        await firstValueFrom(service.startChallenge(10));
        expect(service.getCurrentChallengeSync()).not.toBeNull();

        mockBackend.submitMasteryChallenge.mockReturnValue(
          Promise.resolve(buildChallengeResult())
        );
        service.submitChallenge('challenge-1', responses, 300).subscribe();
        tick();
        expect(service.getCurrentChallengeSync()).toBeNull();
      }));

      it('should call refreshPool when pool exists and result has challenge', fakeAsync(async () => {
        await setPoolState();
        mockBackend.refreshPracticePool.mockClear();
        mockBackend.submitMasteryChallenge.mockReturnValue(
          Promise.resolve(buildChallengeResult())
        );
        service.submitChallenge('challenge-1', responses, 300).subscribe();
        tick();
        expect(mockBackend.refreshPracticePool).toHaveBeenCalled();
      }));

      it('should NOT call refreshPool when pool is null', fakeAsync(() => {
        mockBackend.refreshPracticePool.mockClear();
        mockBackend.submitMasteryChallenge.mockReturnValue(
          Promise.resolve(buildChallengeResult())
        );
        service.submitChallenge('challenge-1', responses, 300).subscribe();
        tick();
        expect(mockBackend.refreshPracticePool).not.toHaveBeenCalled();
      }));

      it('should call checkCooldown on success', fakeAsync(() => {
        mockBackend.checkChallengeCooldown.mockClear();
        mockBackend.submitMasteryChallenge.mockReturnValue(
          Promise.resolve(buildChallengeResult())
        );
        service.submitChallenge('challenge-1', responses, 300).subscribe();
        tick();
        expect(mockBackend.checkChallengeCooldown).toHaveBeenCalled();
      }));

      it('should prepend completed challenge to challengeHistory$', fakeAsync(() => {
        const completedChallenge = buildChallenge({
          id: 'challenge-completed',
          state: 'completed',
        });
        mockBackend.submitMasteryChallenge.mockReturnValue(
          Promise.resolve(
            buildChallengeResult({
              challenge: {
                action_hash: new Uint8Array(),
                challenge: completedChallenge,
              },
            })
          )
        );
        service.submitChallenge('challenge-1', responses, 300).subscribe();
        tick();
        const history = service.getChallengeHistorySync();
        expect(history.length).toBe(1);
        expect(history[0].id).toBe('challenge-completed');
      }));

      it('should not update history when result is null', fakeAsync(() => {
        mockBackend.submitMasteryChallenge.mockReturnValue(Promise.resolve(null));
        service.submitChallenge('challenge-1', responses, 300).subscribe();
        tick();
        expect(service.getChallengeHistorySync().length).toBe(0);
      }));

      it('should return null on backend error', async () => {
        mockBackend.submitMasteryChallenge.mockReturnValue(Promise.reject(new Error('fail')));
        const result = await firstValueFrom(
          service.submitChallenge('challenge-1', responses, 300)
        );
        expect(result).toBeNull();
      });
    });

    describe('abandonChallenge()', () => {
      it('should set currentChallenge$ to null', async () => {
        // Set a challenge first
        const challenge = buildChallenge();
        mockBackend.startMasteryChallenge.mockReturnValue(
          Promise.resolve({ action_hash: new Uint8Array(), challenge })
        );
        await firstValueFrom(service.startChallenge(10));
        expect(service.getCurrentChallengeSync()).not.toBeNull();

        service.abandonChallenge();
        expect(service.getCurrentChallengeSync()).toBeNull();
      });
    });

    describe('hasActiveChallenge()', () => {
      it('should return false when no challenge', () => {
        expect(service.hasActiveChallenge()).toBe(false);
      });

      it('should return true when challenge state is in_progress', async () => {
        mockBackend.startMasteryChallenge.mockReturnValue(
          Promise.resolve({
            action_hash: new Uint8Array(),
            challenge: buildChallenge({ state: 'in_progress' }),
          })
        );
        await firstValueFrom(service.startChallenge(10));
        expect(service.hasActiveChallenge()).toBe(true);
      });

      it('should return false when challenge state is completed', async () => {
        mockBackend.startMasteryChallenge.mockReturnValue(
          Promise.resolve({
            action_hash: new Uint8Array(),
            challenge: buildChallenge({ state: 'completed' }),
          })
        );
        await firstValueFrom(service.startChallenge(10));
        expect(service.hasActiveChallenge()).toBe(false);
      });

      it('should return false when challenge state is abandoned', async () => {
        mockBackend.startMasteryChallenge.mockReturnValue(
          Promise.resolve({
            action_hash: new Uint8Array(),
            challenge: buildChallenge({ state: 'abandoned' }),
          })
        );
        await firstValueFrom(service.startChallenge(10));
        expect(service.hasActiveChallenge()).toBe(false);
      });
    });
  });

  // ===========================================================================
  // Challenge History
  // ===========================================================================

  describe('Challenge History', () => {
    describe('loadChallengeHistory()', () => {
      it('should map MasteryChallengeOutputs to MasteryChallenges', async () => {
        const c1 = buildChallenge({ id: 'c1' });
        const c2 = buildChallenge({ id: 'c2' });
        mockBackend.getChallengeHistory.mockReturnValue(
          Promise.resolve([
            { action_hash: new Uint8Array(), challenge: c1 },
            { action_hash: new Uint8Array(), challenge: c2 },
          ])
        );
        const result = await firstValueFrom(service.loadChallengeHistory());
        expect(result.length).toBe(2);
        expect(result[0].id).toBe('c1');
        expect(result[1].id).toBe('c2');
      });

      it('should update challengeHistory$ subject', async () => {
        mockBackend.getChallengeHistory.mockReturnValue(
          Promise.resolve([
            { action_hash: new Uint8Array(), challenge: buildChallenge({ id: 'c1' }) },
          ])
        );
        await firstValueFrom(service.loadChallengeHistory());
        const history = service.getChallengeHistorySync();
        expect(history.length).toBe(1);
        expect(history[0].id).toBe('c1');
      });

      it('should return empty array on backend error', async () => {
        mockBackend.getChallengeHistory.mockReturnValue(Promise.reject(new Error('fail')));
        const result = await firstValueFrom(service.loadChallengeHistory());
        expect(result).toEqual([]);
      });

      it('should handle empty history from backend', async () => {
        mockBackend.getChallengeHistory.mockReturnValue(Promise.resolve([]));
        const result = await firstValueFrom(service.loadChallengeHistory());
        expect(result).toEqual([]);
      });
    });

    describe('getChallengeHistorySync()', () => {
      it('should return empty array initially', () => {
        expect(service.getChallengeHistorySync()).toEqual([]);
      });
    });
  });

  // ===========================================================================
  // Pool Analysis
  // ===========================================================================

  describe('Pool Analysis', () => {
    describe('getActiveCount()', () => {
      it('should return 0 when pool is null', () => {
        expect(service.getActiveCount()).toBe(0);
      });

      it('should return count of active content IDs', async () => {
        await setPoolState(buildPool());
        expect(service.getActiveCount()).toBe(3);
      });

      it('should return 0 for malformed JSON', async () => {
        await setPoolState(buildPool({ active_content_ids_json: 'not json' }));
        expect(service.getActiveCount()).toBe(0);
      });
    });

    describe('getRefreshCount()', () => {
      it('should return 0 when pool is null', () => {
        expect(service.getRefreshCount()).toBe(0);
      });

      it('should return count of refresh queue IDs', async () => {
        await setPoolState(buildPool());
        expect(service.getRefreshCount()).toBe(2);
      });

      it('should return 0 for malformed JSON', async () => {
        await setPoolState(buildPool({ refresh_queue_ids_json: '{bad' }));
        expect(service.getRefreshCount()).toBe(0);
      });
    });

    describe('getDiscoveryCount()', () => {
      it('should return 0 when pool is null', () => {
        expect(service.getDiscoveryCount()).toBe(0);
      });

      it('should return count of discovery candidates', async () => {
        await setPoolState(buildPool());
        expect(service.getDiscoveryCount()).toBe(1);
      });

      it('should return 0 for malformed JSON', async () => {
        await setPoolState(buildPool({ discovery_candidates_json: '' }));
        expect(service.getDiscoveryCount()).toBe(0);
      });
    });

    describe('getPoolStats()', () => {
      it('should return all-zero stats when pool is null', () => {
        const stats = service.getPoolStats();
        expect(stats).toEqual({
          active: 0,
          refresh: 0,
          discovery: 0,
          totalChallenges: 0,
          levelUps: 0,
          levelDowns: 0,
          discoveries: 0,
        });
      });

      it('should return computed stats from pool data', async () => {
        await setPoolState(buildPool());
        const stats = service.getPoolStats();
        expect(stats).toEqual({
          active: 3,
          refresh: 2,
          discovery: 1,
          totalChallenges: 10,
          levelUps: 5,
          levelDowns: 2,
          discoveries: 3,
        });
      });
    });
  });
});
