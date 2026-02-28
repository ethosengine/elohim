import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { AssessmentService, AssessmentSession, AssessmentResult } from './assessment.service';
import {
  DataLoaderService,
  AssessmentIndex,
  AssessmentIndexEntry,
} from '@app/elohim/services/data-loader.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { ContentNode } from '../models/content-node.model';
import { vi, Mock } from 'vitest';

describe('AssessmentService', () => {
  let service: AssessmentService;
  let dataLoaderSpy: any;
  let sessionUserSpy: any;
  let localStorageMock: { [key: string]: string };
  let mockStorage: Storage;

  const mockAssessmentIndex: AssessmentIndex = {
    lastUpdated: '2025-01-01T00:00:00.000Z',
    totalCount: 2,
    assessments: [
      {
        id: 'assessment-1',
        title: 'Values Assessment',
        domain: 'values',
        instrumentType: 'questionnaire',
        estimatedTime: '15 minutes',
      },
      {
        id: 'assessment-2',
        title: 'Attachment Style',
        domain: 'attachment',
        instrumentType: 'questionnaire',
        estimatedTime: '20 minutes',
      },
    ],
  };

  const mockAssessmentNode: ContentNode = {
    id: 'assessment-1',
    title: 'Values Assessment',
    description: 'Discover your core values',
    contentType: 'assessment',
    contentFormat: 'perseus-quiz-json',
    content: {
      questions: [
        { id: 'q1', text: 'Question 1', subscales: ['value-a'] },
        { id: 'q2', text: 'Question 2', subscales: ['value-b'], reverseScored: true },
      ],
      sections: [],
      interpretation: { method: 'ranking' },
    },
    tags: ['values', 'self-knowledge'],
    relatedNodeIds: [],
    metadata: { attestationId: 'values-self-knowledge' },
  };

  const mockGatedAssessment: ContentNode = {
    id: 'gated-assessment',
    title: 'Advanced Assessment',
    description: 'Requires prerequisite',
    contentType: 'assessment',
    contentFormat: 'perseus-quiz-json',
    content: { questions: [], sections: [] },
    tags: [],
    relatedNodeIds: [],
    metadata: { prerequisiteAttestation: 'basic-attestation' },
  };

  beforeEach(() => {
    const dataLoaderSpyObj = {
      getAssessmentIndex: vi.fn(),
      getAssessmentsByDomain: vi.fn(),
      getAssessment: vi.fn(),
    };
    const sessionUserSpyObj = {
      getSessionId: vi.fn(),
      getSession: vi.fn(),
    };

    // Mock localStorage
    localStorageMock = {};
    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => {
        localStorageMock[key] = value;
      },
      removeItem: (key: string) => {
        delete localStorageMock[key];
      },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() {
        return Object.keys(localStorageMock).length;
      },
      clear: () => {
        localStorageMock = {};
      },
    };
    vi.spyOn(window, 'localStorage', 'get').mockReturnValue(mockStorage);

    TestBed.configureTestingModule({
      providers: [
        AssessmentService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: SessionHumanService, useValue: sessionUserSpyObj },
      ],
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as { [K in keyof DataLoaderService]?: Mock };
    sessionUserSpy = TestBed.inject(SessionHumanService) as { [K in keyof SessionHumanService]?: Mock };

    // Default spy return values
    dataLoaderSpy.getAssessmentIndex.mockReturnValue(of(mockAssessmentIndex));
    dataLoaderSpy.getAssessmentsByDomain.mockReturnValue(of(mockAssessmentIndex.assessments));
    dataLoaderSpy.getAssessment.mockReturnValue(of(mockAssessmentNode));
    sessionUserSpy.getSessionId.mockReturnValue('session-123');
    sessionUserSpy.getSession.mockReturnValue(null);

    service = TestBed.inject(AssessmentService);
  });

  afterEach(() => {
    localStorageMock = {};
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // Assessment Discovery
  // =========================================================================

  describe('getAssessmentIndex', () => {
    it('should return assessment index from data loader', () => new Promise<void>(done => {
      service.getAssessmentIndex().subscribe(index => {
        expect(index).toEqual(mockAssessmentIndex);
        expect(dataLoaderSpy.getAssessmentIndex).toHaveBeenCalled();
        done();
      });
    }));
  });

  describe('getAssessmentsByDomain', () => {
    it('should filter assessments by domain', () => new Promise<void>(done => {
      service.getAssessmentsByDomain('values').subscribe(assessments => {
        expect(dataLoaderSpy.getAssessmentsByDomain).toHaveBeenCalledWith('values');
        done();
      });
    }));
  });

  describe('getAssessment', () => {
    it('should load assessment by ID', () => new Promise<void>(done => {
      service.getAssessment('assessment-1').subscribe(assessment => {
        expect(assessment).toEqual(mockAssessmentNode);
        expect(dataLoaderSpy.getAssessment).toHaveBeenCalledWith('assessment-1');
        done();
      });
    }));

    it('should return null for missing assessment', () => new Promise<void>(done => {
      dataLoaderSpy.getAssessment.mockReturnValue(of(null));

      service.getAssessment('missing').subscribe(assessment => {
        expect(assessment).toBeNull();
        done();
      });
    }));
  });

  describe('canAccessAssessment', () => {
    it('should allow access when no prerequisite', () => new Promise<void>(done => {
      service.canAccessAssessment('assessment-1').subscribe(canAccess => {
        expect(canAccess).toBe(true);
        done();
      });
    }));

    it('should deny access when prerequisite attestation is missing', () => new Promise<void>(done => {
      dataLoaderSpy.getAssessment.mockReturnValue(of(mockGatedAssessment));

      service.canAccessAssessment('gated-assessment').subscribe(canAccess => {
        expect(canAccess).toBe(false);
        done();
      });
    }));

    it('should allow access when prerequisite attestation is present', () => new Promise<void>(done => {
      dataLoaderSpy.getAssessment.mockReturnValue(of(mockGatedAssessment));
      // Store the attestation in localStorage
      localStorageMock['lamad-assessment-attestations-session-123'] = JSON.stringify([
        'basic-attestation',
      ]);

      service.canAccessAssessment('gated-assessment').subscribe(canAccess => {
        expect(canAccess).toBe(true);
        done();
      });
    }));

    it('should return false for non-existent assessment', () => new Promise<void>(done => {
      dataLoaderSpy.getAssessment.mockReturnValue(of(null));

      service.canAccessAssessment('missing').subscribe(canAccess => {
        expect(canAccess).toBe(false);
        done();
      });
    }));
  });

  // =========================================================================
  // Assessment Sessions
  // =========================================================================

  describe('startAssessment', () => {
    it('should create a new session', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(session => {
        expect(session.assessmentId).toBe('assessment-1');
        expect(session.agentId).toBe('session-123');
        expect(session.currentQuestionIndex).toBe(0);
        expect(session.responses).toEqual({});
        expect(session.timeSpentMs).toBe(0);
        done();
      });
    }));

    it('should save session to localStorage', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        const key = 'lamad-assessment-session-session-123-assessment-1';
        expect(localStorageMock[key]).toBeDefined();
        done();
      });
    }));

    it('should use anonymous for missing session ID', () => new Promise<void>(done => {
      sessionUserSpy.getSessionId.mockReturnValue(undefined as unknown as string);

      service.startAssessment('assessment-1').subscribe(session => {
        expect(session.agentId).toBe('anonymous');
        done();
      });
    }));
  });

  describe('getActiveSession', () => {
    it('should return null when no active session', () => new Promise<void>(done => {
      service.getActiveSession().subscribe(session => {
        expect(session).toBeNull();
        done();
      });
    }));

    it('should return active session after starting assessment', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.getActiveSession().subscribe(session => {
          expect(session).not.toBeNull();
          expect(session?.assessmentId).toBe('assessment-1');
          done();
        });
      });
    }));
  });

  describe('resumeAssessment', () => {
    it('should resume session from localStorage', () => new Promise<void>(done => {
      const savedSession: AssessmentSession = {
        assessmentId: 'assessment-1',
        agentId: 'session-123',
        startedAt: '2025-01-01T00:00:00.000Z',
        currentQuestionIndex: 2,
        responses: {
          q1: {
            questionId: 'q1',
            questionType: 'likert',
            value: 5,
            answeredAt: '2025-01-01T00:00:00.000Z',
          },
        },
        timeSpentMs: 60000,
      };
      localStorageMock['lamad-assessment-session-session-123-assessment-1'] =
        JSON.stringify(savedSession);

      service.resumeAssessment('assessment-1').subscribe(session => {
        expect(session).not.toBeNull();
        expect(session?.currentQuestionIndex).toBe(2);
        expect(Object.keys(session?.responses ?? {})).toContain('q1');
        done();
      });
    }));

    it('should return null when no saved session', () => new Promise<void>(done => {
      service.resumeAssessment('missing').subscribe(session => {
        expect(session).toBeNull();
        done();
      });
    }));
  });

  describe('recordResponse', () => {
    it('should record response and increment question index', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'likert', 5);

        service.getActiveSession().subscribe(session => {
          expect(session?.responses['q1']).toBeDefined();
          expect(session?.responses['q1'].value).toBe(5);
          expect(session?.currentQuestionIndex).toBe(1);
          done();
        });
      });
    }));

    it('should handle string responses', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'text', 'My answer');

        service.getActiveSession().subscribe(session => {
          expect(session?.responses['q1'].value).toBe('My answer');
          done();
        });
      });
    }));

    it('should handle array responses', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'multi-select', ['a', 'b']);

        service.getActiveSession().subscribe(session => {
          expect(session?.responses['q1'].value).toEqual(['a', 'b']);
          done();
        });
      });
    }));

    it('should do nothing when no active session', () => {
      vi.spyOn(console, 'error');
      service.recordResponse('q1', 'likert', 5);
      expect(console.error).toHaveBeenCalled();
    });
  });

  describe('updateTimeSpent', () => {
    it('should accumulate time spent', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.updateTimeSpent(5000);
        service.updateTimeSpent(3000);

        service.getActiveSession().subscribe(session => {
          expect(session?.timeSpentMs).toBe(8000);
          done();
        });
      });
    }));

    it('should do nothing when no active session', () => {
      service.updateTimeSpent(5000);
      // Should not throw
    });
  });

  describe('abandonAssessment', () => {
    it('should clear active session', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.abandonAssessment();

        service.getActiveSession().subscribe(session => {
          expect(session).toBeNull();
          done();
        });
      });
    }));

    it('should remove session from localStorage', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        const key = 'lamad-assessment-session-session-123-assessment-1';
        expect(localStorageMock[key]).toBeDefined();

        service.abandonAssessment();
        expect(localStorageMock[key]).toBeUndefined();
        done();
      });
    }));
  });

  // =========================================================================
  // Scoring & Completion
  // =========================================================================

  describe('completeAssessment', () => {
    it('should return null when no active session', () => new Promise<void>(done => {
      service.completeAssessment().subscribe(result => {
        expect(result).toBeNull();
        done();
      });
    }));

    it('should compute scores and return result', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'likert', 7);
        service.recordResponse('q2', 'likert', 3);

        service.completeAssessment().subscribe(result => {
          expect(result).not.toBeNull();
          expect(result?.assessmentId).toBe('assessment-1');
          expect(result?.agentId).toBe('session-123');
          expect(result?.scores).toBeDefined();
          expect(result?.attestationGranted).toBe('values-self-knowledge');
          done();
        });
      });
    }));

    it('should clear session after completion', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.completeAssessment().subscribe(() => {
          service.getActiveSession().subscribe(session => {
            expect(session).toBeNull();
            done();
          });
        });
      });
    }));

    it('should save result to localStorage', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.completeAssessment().subscribe(() => {
          const key = 'lamad-assessment-result-session-123-assessment-1';
          expect(localStorageMock[key]).toBeDefined();
          done();
        });
      });
    }));

    it('should return null when assessment not found', () => new Promise<void>(done => {
      dataLoaderSpy.getAssessment.mockReturnValue(of(null));

      service.startAssessment('missing').subscribe(() => {
        service.completeAssessment().subscribe(result => {
          expect(result).toBeNull();
          done();
        });
      });
    }));

    it('should handle quadrant interpretation', () => new Promise<void>(done => {
      const quadrantAssessment: ContentNode = {
        ...mockAssessmentNode,
        content: {
          questions: [
            { id: 'q1', subscales: ['anxiety'] },
            { id: 'q2', subscales: ['avoidance'] },
          ],
          sections: [],
          interpretation: {
            method: 'quadrant',
            dimensions: ['anxiety', 'avoidance'],
            outcomes: [
              { name: 'Secure', anxiety: 'low', avoidance: 'low' },
              { name: 'Anxious', anxiety: 'high', avoidance: 'low' },
              { name: 'Avoidant', anxiety: 'low', avoidance: 'high' },
              { name: 'Disorganized', anxiety: 'high', avoidance: 'high' },
            ],
          },
        },
      };
      dataLoaderSpy.getAssessment.mockReturnValue(of(quadrantAssessment));

      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'likert', 2); // Low anxiety
        service.recordResponse('q2', 'likert', 2); // Low avoidance

        service.completeAssessment().subscribe(result => {
          expect(result?.interpretation).toBe('Secure');
          done();
        });
      });
    }));

    it('should handle ranking interpretation', () => new Promise<void>(done => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'likert', 7);
        service.recordResponse('q2', 'likert', 5);

        service.completeAssessment().subscribe(result => {
          // Ranking returns the top subscale
          expect(result?.interpretation).toBeDefined();
          done();
        });
      });
    }));
  });

  // =========================================================================
  // Results History
  // =========================================================================

  describe('getMyResults', () => {
    it('should return empty array when no results', () => new Promise<void>(done => {
      service.getMyResults().subscribe(results => {
        expect(results).toEqual([]);
        done();
      });
    }));

    it('should return sorted results from localStorage', () => new Promise<void>(done => {
      const result1: AssessmentResult = {
        assessmentId: 'assessment-1',
        agentId: 'session-123',
        completedAt: '2025-01-01T00:00:00.000Z',
        timeSpentMs: 60000,
        responses: {},
        scores: { 'value-a': 10 },
      };
      const result2: AssessmentResult = {
        assessmentId: 'assessment-2',
        agentId: 'session-123',
        completedAt: '2025-01-02T00:00:00.000Z',
        timeSpentMs: 90000,
        responses: {},
        scores: { attachment: 15 },
      };
      localStorageMock['lamad-assessment-result-session-123-assessment-1'] =
        JSON.stringify(result1);
      localStorageMock['lamad-assessment-result-session-123-assessment-2'] =
        JSON.stringify(result2);

      service.getMyResults().subscribe(results => {
        expect(results.length).toBe(2);
        // Should be sorted by completedAt descending
        expect(results[0].assessmentId).toBe('assessment-2');
        done();
      });
    }));

    it('should skip malformed entries', () => new Promise<void>(done => {
      localStorageMock['lamad-assessment-result-session-123-bad'] = 'invalid json';

      service.getMyResults().subscribe(results => {
        expect(results.length).toBe(0);
        done();
      });
    }));
  });

  describe('getResultForAssessment', () => {
    it('should return specific result', () => new Promise<void>(done => {
      const result: AssessmentResult = {
        assessmentId: 'assessment-1',
        agentId: 'session-123',
        completedAt: '2025-01-01T00:00:00.000Z',
        timeSpentMs: 60000,
        responses: {},
        scores: {},
      };
      localStorageMock['lamad-assessment-result-session-123-assessment-1'] = JSON.stringify(result);

      service.getResultForAssessment('assessment-1').subscribe(r => {
        expect(r).not.toBeNull();
        expect(r?.assessmentId).toBe('assessment-1');
        done();
      });
    }));

    it('should return null for missing result', () => new Promise<void>(done => {
      service.getResultForAssessment('missing').subscribe(result => {
        expect(result).toBeNull();
        done();
      });
    }));
  });

  describe('hasCompleted', () => {
    it('should return true when result exists', () => new Promise<void>(done => {
      const result: AssessmentResult = {
        assessmentId: 'assessment-1',
        agentId: 'session-123',
        completedAt: '2025-01-01T00:00:00.000Z',
        timeSpentMs: 60000,
        responses: {},
        scores: {},
      };
      localStorageMock['lamad-assessment-result-session-123-assessment-1'] = JSON.stringify(result);

      service.hasCompleted('assessment-1').subscribe(hasCompleted => {
        expect(hasCompleted).toBe(true);
        done();
      });
    }));

    it('should return false when no result', () => new Promise<void>(done => {
      service.hasCompleted('missing').subscribe(hasCompleted => {
        expect(hasCompleted).toBe(false);
        done();
      });
    }));
  });
});
