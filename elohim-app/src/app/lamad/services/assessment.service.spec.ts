import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { AssessmentService, AssessmentSession, AssessmentResult } from './assessment.service';
import { DataLoaderService, AssessmentIndex, AssessmentIndexEntry } from '@app/elohim/services/data-loader.service';
import { SessionHumanService } from './session-human.service';
import { ContentNode } from '../models/content-node.model';

describe('AssessmentService', () => {
  let service: AssessmentService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let sessionUserSpy: jasmine.SpyObj<SessionHumanService>;
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
        estimatedTime: '15 minutes'
      },
      {
        id: 'assessment-2',
        title: 'Attachment Style',
        domain: 'attachment',
        instrumentType: 'questionnaire',
        estimatedTime: '20 minutes'
      }
    ]
  };

  const mockAssessmentNode: ContentNode = {
    id: 'assessment-1',
    title: 'Values Assessment',
    description: 'Discover your core values',
    contentType: 'assessment',
    contentFormat: 'quiz-json',
    content: {
      questions: [
        { id: 'q1', text: 'Question 1', subscales: ['value-a'] },
        { id: 'q2', text: 'Question 2', subscales: ['value-b'], reverseScored: true }
      ],
      sections: [],
      interpretation: { method: 'ranking' }
    },
    tags: ['values', 'self-knowledge'],
    relatedNodeIds: [],
    metadata: { attestationId: 'values-self-knowledge' }
  };

  const mockGatedAssessment: ContentNode = {
    id: 'gated-assessment',
    title: 'Advanced Assessment',
    description: 'Requires prerequisite',
    contentType: 'assessment',
    contentFormat: 'quiz-json',
    content: { questions: [], sections: [] },
    tags: [],
    relatedNodeIds: [],
    metadata: { prerequisiteAttestation: 'basic-attestation' }
  };

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', [
      'getAssessmentIndex',
      'getAssessmentsByDomain',
      'getAssessment'
    ]);
    const sessionUserSpyObj = jasmine.createSpyObj('SessionHumanService', [
      'getSessionId',
      'getSession'
    ]);

    // Mock localStorage
    localStorageMock = {};
    mockStorage = {
      getItem: (key: string) => localStorageMock[key] || null,
      setItem: (key: string, value: string) => { localStorageMock[key] = value; },
      removeItem: (key: string) => { delete localStorageMock[key]; },
      key: (index: number) => Object.keys(localStorageMock)[index] || null,
      get length() { return Object.keys(localStorageMock).length; },
      clear: () => { localStorageMock = {}; }
    };
    spyOnProperty(window, 'localStorage', 'get').and.returnValue(mockStorage);

    TestBed.configureTestingModule({
      providers: [
        AssessmentService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: SessionHumanService, useValue: sessionUserSpyObj }
      ]
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    sessionUserSpy = TestBed.inject(SessionHumanService) as jasmine.SpyObj<SessionHumanService>;

    // Default spy return values
    dataLoaderSpy.getAssessmentIndex.and.returnValue(of(mockAssessmentIndex));
    dataLoaderSpy.getAssessmentsByDomain.and.returnValue(of(mockAssessmentIndex.assessments));
    dataLoaderSpy.getAssessment.and.returnValue(of(mockAssessmentNode));
    sessionUserSpy.getSessionId.and.returnValue('session-123');
    sessionUserSpy.getSession.and.returnValue(null);

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
    it('should return assessment index from data loader', (done) => {
      service.getAssessmentIndex().subscribe(index => {
        expect(index).toEqual(mockAssessmentIndex);
        expect(dataLoaderSpy.getAssessmentIndex).toHaveBeenCalled();
        done();
      });
    });
  });

  describe('getAssessmentsByDomain', () => {
    it('should filter assessments by domain', (done) => {
      service.getAssessmentsByDomain('values').subscribe(assessments => {
        expect(dataLoaderSpy.getAssessmentsByDomain).toHaveBeenCalledWith('values');
        done();
      });
    });
  });

  describe('getAssessment', () => {
    it('should load assessment by ID', (done) => {
      service.getAssessment('assessment-1').subscribe(assessment => {
        expect(assessment).toEqual(mockAssessmentNode);
        expect(dataLoaderSpy.getAssessment).toHaveBeenCalledWith('assessment-1');
        done();
      });
    });

    it('should return null for missing assessment', (done) => {
      dataLoaderSpy.getAssessment.and.returnValue(of(null));

      service.getAssessment('missing').subscribe(assessment => {
        expect(assessment).toBeNull();
        done();
      });
    });
  });

  describe('canAccessAssessment', () => {
    it('should allow access when no prerequisite', (done) => {
      service.canAccessAssessment('assessment-1').subscribe(canAccess => {
        expect(canAccess).toBe(true);
        done();
      });
    });

    it('should deny access when prerequisite attestation is missing', (done) => {
      dataLoaderSpy.getAssessment.and.returnValue(of(mockGatedAssessment));

      service.canAccessAssessment('gated-assessment').subscribe(canAccess => {
        expect(canAccess).toBe(false);
        done();
      });
    });

    it('should allow access when prerequisite attestation is present', (done) => {
      dataLoaderSpy.getAssessment.and.returnValue(of(mockGatedAssessment));
      // Store the attestation in localStorage
      localStorageMock['lamad-assessment-attestations-session-123'] = JSON.stringify(['basic-attestation']);

      service.canAccessAssessment('gated-assessment').subscribe(canAccess => {
        expect(canAccess).toBe(true);
        done();
      });
    });

    it('should return false for non-existent assessment', (done) => {
      dataLoaderSpy.getAssessment.and.returnValue(of(null));

      service.canAccessAssessment('missing').subscribe(canAccess => {
        expect(canAccess).toBe(false);
        done();
      });
    });
  });

  // =========================================================================
  // Assessment Sessions
  // =========================================================================

  describe('startAssessment', () => {
    it('should create a new session', (done) => {
      service.startAssessment('assessment-1').subscribe(session => {
        expect(session.assessmentId).toBe('assessment-1');
        expect(session.agentId).toBe('session-123');
        expect(session.currentQuestionIndex).toBe(0);
        expect(session.responses).toEqual({});
        expect(session.timeSpentMs).toBe(0);
        done();
      });
    });

    it('should save session to localStorage', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        const key = 'lamad-assessment-session-session-123-assessment-1';
        expect(localStorageMock[key]).toBeDefined();
        done();
      });
    });

    it('should use anonymous for missing session ID', (done) => {
      sessionUserSpy.getSessionId.and.returnValue(undefined as unknown as string);

      service.startAssessment('assessment-1').subscribe(session => {
        expect(session.agentId).toBe('anonymous');
        done();
      });
    });
  });

  describe('getActiveSession', () => {
    it('should return null when no active session', (done) => {
      service.getActiveSession().subscribe(session => {
        expect(session).toBeNull();
        done();
      });
    });

    it('should return active session after starting assessment', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.getActiveSession().subscribe(session => {
          expect(session).not.toBeNull();
          expect(session?.assessmentId).toBe('assessment-1');
          done();
        });
      });
    });
  });

  describe('resumeAssessment', () => {
    it('should resume session from localStorage', (done) => {
      const savedSession: AssessmentSession = {
        assessmentId: 'assessment-1',
        agentId: 'session-123',
        startedAt: '2025-01-01T00:00:00.000Z',
        currentQuestionIndex: 2,
        responses: { 'q1': { questionId: 'q1', questionType: 'likert', value: 5, answeredAt: '2025-01-01T00:00:00.000Z' } },
        timeSpentMs: 60000
      };
      localStorageMock['lamad-assessment-session-session-123-assessment-1'] = JSON.stringify(savedSession);

      service.resumeAssessment('assessment-1').subscribe(session => {
        expect(session).not.toBeNull();
        expect(session?.currentQuestionIndex).toBe(2);
        expect(Object.keys(session?.responses ?? {})).toContain('q1');
        done();
      });
    });

    it('should return null when no saved session', (done) => {
      service.resumeAssessment('missing').subscribe(session => {
        expect(session).toBeNull();
        done();
      });
    });
  });

  describe('recordResponse', () => {
    it('should record response and increment question index', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'likert', 5);

        service.getActiveSession().subscribe(session => {
          expect(session?.responses['q1']).toBeDefined();
          expect(session?.responses['q1'].value).toBe(5);
          expect(session?.currentQuestionIndex).toBe(1);
          done();
        });
      });
    });

    it('should handle string responses', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'text', 'My answer');

        service.getActiveSession().subscribe(session => {
          expect(session?.responses['q1'].value).toBe('My answer');
          done();
        });
      });
    });

    it('should handle array responses', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'multi-select', ['a', 'b']);

        service.getActiveSession().subscribe(session => {
          expect(session?.responses['q1'].value).toEqual(['a', 'b']);
          done();
        });
      });
    });

    it('should do nothing when no active session', () => {
      spyOn(console, 'error');
      service.recordResponse('q1', 'likert', 5);
      expect(console.error).toHaveBeenCalled();
    });
  });

  describe('updateTimeSpent', () => {
    it('should accumulate time spent', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.updateTimeSpent(5000);
        service.updateTimeSpent(3000);

        service.getActiveSession().subscribe(session => {
          expect(session?.timeSpentMs).toBe(8000);
          done();
        });
      });
    });

    it('should do nothing when no active session', () => {
      service.updateTimeSpent(5000);
      // Should not throw
    });
  });

  describe('abandonAssessment', () => {
    it('should clear active session', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.abandonAssessment();

        service.getActiveSession().subscribe(session => {
          expect(session).toBeNull();
          done();
        });
      });
    });

    it('should remove session from localStorage', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        const key = 'lamad-assessment-session-session-123-assessment-1';
        expect(localStorageMock[key]).toBeDefined();

        service.abandonAssessment();
        expect(localStorageMock[key]).toBeUndefined();
        done();
      });
    });
  });

  // =========================================================================
  // Scoring & Completion
  // =========================================================================

  describe('completeAssessment', () => {
    it('should return null when no active session', (done) => {
      service.completeAssessment().subscribe(result => {
        expect(result).toBeNull();
        done();
      });
    });

    it('should compute scores and return result', (done) => {
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
    });

    it('should clear session after completion', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.completeAssessment().subscribe(() => {
          service.getActiveSession().subscribe(session => {
            expect(session).toBeNull();
            done();
          });
        });
      });
    });

    it('should save result to localStorage', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.completeAssessment().subscribe(() => {
          const key = 'lamad-assessment-result-session-123-assessment-1';
          expect(localStorageMock[key]).toBeDefined();
          done();
        });
      });
    });

    it('should return null when assessment not found', (done) => {
      dataLoaderSpy.getAssessment.and.returnValue(of(null));

      service.startAssessment('missing').subscribe(() => {
        service.completeAssessment().subscribe(result => {
          expect(result).toBeNull();
          done();
        });
      });
    });

    it('should handle quadrant interpretation', (done) => {
      const quadrantAssessment: ContentNode = {
        ...mockAssessmentNode,
        content: {
          questions: [
            { id: 'q1', subscales: ['anxiety'] },
            { id: 'q2', subscales: ['avoidance'] }
          ],
          sections: [],
          interpretation: {
            method: 'quadrant',
            dimensions: ['anxiety', 'avoidance'],
            outcomes: [
              { name: 'Secure', anxiety: 'low', avoidance: 'low' },
              { name: 'Anxious', anxiety: 'high', avoidance: 'low' },
              { name: 'Avoidant', anxiety: 'low', avoidance: 'high' },
              { name: 'Disorganized', anxiety: 'high', avoidance: 'high' }
            ]
          }
        }
      };
      dataLoaderSpy.getAssessment.and.returnValue(of(quadrantAssessment));

      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'likert', 2); // Low anxiety
        service.recordResponse('q2', 'likert', 2); // Low avoidance

        service.completeAssessment().subscribe(result => {
          expect(result?.interpretation).toBe('Secure');
          done();
        });
      });
    });

    it('should handle ranking interpretation', (done) => {
      service.startAssessment('assessment-1').subscribe(() => {
        service.recordResponse('q1', 'likert', 7);
        service.recordResponse('q2', 'likert', 5);

        service.completeAssessment().subscribe(result => {
          // Ranking returns the top subscale
          expect(result?.interpretation).toBeDefined();
          done();
        });
      });
    });
  });

  // =========================================================================
  // Results History
  // =========================================================================

  describe('getMyResults', () => {
    it('should return empty array when no results', (done) => {
      service.getMyResults().subscribe(results => {
        expect(results).toEqual([]);
        done();
      });
    });

    it('should return sorted results from localStorage', (done) => {
      const result1: AssessmentResult = {
        assessmentId: 'assessment-1',
        agentId: 'session-123',
        completedAt: '2025-01-01T00:00:00.000Z',
        timeSpentMs: 60000,
        responses: {},
        scores: { 'value-a': 10 }
      };
      const result2: AssessmentResult = {
        assessmentId: 'assessment-2',
        agentId: 'session-123',
        completedAt: '2025-01-02T00:00:00.000Z',
        timeSpentMs: 90000,
        responses: {},
        scores: { 'attachment': 15 }
      };
      localStorageMock['lamad-assessment-result-session-123-assessment-1'] = JSON.stringify(result1);
      localStorageMock['lamad-assessment-result-session-123-assessment-2'] = JSON.stringify(result2);

      service.getMyResults().subscribe(results => {
        expect(results.length).toBe(2);
        // Should be sorted by completedAt descending
        expect(results[0].assessmentId).toBe('assessment-2');
        done();
      });
    });

    it('should skip malformed entries', (done) => {
      localStorageMock['lamad-assessment-result-session-123-bad'] = 'invalid json';

      service.getMyResults().subscribe(results => {
        expect(results.length).toBe(0);
        done();
      });
    });
  });

  describe('getResultForAssessment', () => {
    it('should return specific result', (done) => {
      const result: AssessmentResult = {
        assessmentId: 'assessment-1',
        agentId: 'session-123',
        completedAt: '2025-01-01T00:00:00.000Z',
        timeSpentMs: 60000,
        responses: {},
        scores: {}
      };
      localStorageMock['lamad-assessment-result-session-123-assessment-1'] = JSON.stringify(result);

      service.getResultForAssessment('assessment-1').subscribe(r => {
        expect(r).not.toBeNull();
        expect(r?.assessmentId).toBe('assessment-1');
        done();
      });
    });

    it('should return null for missing result', (done) => {
      service.getResultForAssessment('missing').subscribe(result => {
        expect(result).toBeNull();
        done();
      });
    });
  });

  describe('hasCompleted', () => {
    it('should return true when result exists', (done) => {
      const result: AssessmentResult = {
        assessmentId: 'assessment-1',
        agentId: 'session-123',
        completedAt: '2025-01-01T00:00:00.000Z',
        timeSpentMs: 60000,
        responses: {},
        scores: {}
      };
      localStorageMock['lamad-assessment-result-session-123-assessment-1'] = JSON.stringify(result);

      service.hasCompleted('assessment-1').subscribe(hasCompleted => {
        expect(hasCompleted).toBe(true);
        done();
      });
    });

    it('should return false when no result', (done) => {
      service.hasCompleted('missing').subscribe(hasCompleted => {
        expect(hasCompleted).toBe(false);
        done();
      });
    });
  });
});
