/**
 * Recovery Coordinator Service Tests
 */

import { TestBed } from '@angular/core/testing';
import { RecoveryCoordinatorService } from './recovery-coordinator.service';
import { DoorwayRegistryService } from './doorway-registry.service';
import { IdentityService } from './identity.service';
import {
  RecoveryRequest,
  RecoveryCredential,
  RecoveryInterview,
  InterviewQuestion,
  PendingRecoveryRequest,
} from '../models/recovery.model';

describe('RecoveryCoordinatorService', () => {
  let service: RecoveryCoordinatorService;
  let mockDoorwayRegistry: jasmine.SpyObj<DoorwayRegistryService>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let originalFetch: typeof fetch;

  const mockDoorwayUrl = 'https://doorway.example.com';

  const mockRecoveryRequest: RecoveryRequest = {
    id: 'recovery-123',
    claimedIdentity: 'john.doe',
    doorwayId: 'doorway-1',
    status: 'pending',
    createdAt: new Date(),
    expiresAt: new Date(Date.now() + 7 * 24 * 60 * 60 * 1000),
    attestations: [],
    requiredAttestations: 3,
    denyThreshold: 2,
    claimantContext: 'Lost my device',
  };

  const mockCredential: RecoveryCredential = {
    id: 'cred-123',
    requestId: 'recovery-123',
    humanId: 'human-456',
    agentPubKey: 'agent-pub-key-789',
    issuedAt: new Date(),
    expiresAt: new Date(Date.now() + 24 * 60 * 60 * 1000),
    claimToken: 'token-abc-123',
    claimed: false,
  };

  const mockInterview: RecoveryInterview = {
    id: 'interview-123',
    requestId: 'recovery-123',
    interviewerId: 'interviewer-456',
    interviewerDisplayName: 'Jane Interviewer',
    status: 'in-progress',
    questions: [],
    responses: [],
    startedAt: new Date(),
  };

  const mockQuestions: InterviewQuestion[] = [
    {
      id: 'q1',
      type: 'network-history',
      question: 'When did you first join the network?',
      difficulty: 1,
      points: 10,
      verifiable: true,
    },
    {
      id: 'q2',
      type: 'relationship',
      question: 'Name three people you connected with early on.',
      difficulty: 2,
      points: 20,
      verifiable: true,
    },
  ];

  const mockPendingRequests: PendingRecoveryRequest[] = [
    {
      requestId: 'recovery-001',
      maskedIdentity: 'joh***',
      doorwayName: 'Main Doorway',
      createdAt: new Date(),
      expiresIn: 6 * 24 * 60 * 60 * 1000,
      progress: {
        affirmCount: 1,
        denyCount: 0,
        abstainCount: 0,
        requiredCount: 3,
        progressPercent: 33,
        thresholdMet: false,
        isDenied: false,
      },
      alreadyAttested: false,
      priority: 5,
    },
  ];

  function mockFetchSuccess(data: any): void {
    (window.fetch as jasmine.Spy).and.resolveTo({
      ok: true,
      json: () => Promise.resolve(data),
    } as Response);
  }

  function mockFetchError(message: string, status = 400): void {
    (window.fetch as jasmine.Spy).and.resolveTo({
      ok: false,
      status,
      json: () => Promise.resolve({ message }),
    } as Response);
  }

  beforeEach(() => {
    originalFetch = window.fetch;
    window.fetch = jasmine.createSpy('fetch');

    mockDoorwayRegistry = jasmine.createSpyObj('DoorwayRegistryService', [], {
      selectedUrl: jasmine.createSpy().and.returnValue(mockDoorwayUrl),
    });

    mockIdentityService = jasmine.createSpyObj('IdentityService', [], {
      agentPubKey: jasmine.createSpy().and.returnValue('agent-123'),
    });

    TestBed.configureTestingModule({
      providers: [
        RecoveryCoordinatorService,
        { provide: DoorwayRegistryService, useValue: mockDoorwayRegistry },
        { provide: IdentityService, useValue: mockIdentityService },
      ],
    });

    service = TestBed.inject(RecoveryCoordinatorService);
  });

  afterEach(() => {
    window.fetch = originalFetch;
  });

  // ===========================================================================
  // Initial State
  // ===========================================================================

  describe('initial state', () => {
    it('should have no active request', () => {
      expect(service.activeRequest()).toBeNull();
      expect(service.hasActiveRequest()).toBe(false);
    });

    it('should have no credential', () => {
      expect(service.credential()).toBeNull();
      expect(service.isRecovered()).toBe(false);
    });

    it('should have no active interview', () => {
      expect(service.activeInterview()).toBeNull();
    });

    it('should have no pending requests', () => {
      expect(service.pendingRequests()).toEqual([]);
      expect(service.pendingCount()).toBe(0);
    });

    it('should have no conducting interview', () => {
      expect(service.conductingInterview()).toBeNull();
    });

    it('should not be loading', () => {
      expect(service.isLoading()).toBe(false);
    });

    it('should have no error', () => {
      expect(service.error()).toBeNull();
    });

    it('should have null progress', () => {
      expect(service.progress()).toBeNull();
    });
  });

  // ===========================================================================
  // Claimant: initiateRecovery
  // ===========================================================================

  describe('initiateRecovery', () => {
    it('should fail if no doorway selected', async () => {
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(null);

      const result = await service.initiateRecovery('john.doe');

      expect(result).toBe(false);
      expect(service.error()).toBe('No doorway selected. Please select a doorway first.');
      expect(window.fetch).not.toHaveBeenCalled();
    });

    it('should initiate recovery successfully', async () => {
      mockFetchSuccess(mockRecoveryRequest);

      const result = await service.initiateRecovery('john.doe', 'Lost my device');

      expect(result).toBe(true);
      expect(service.activeRequest()).toEqual(mockRecoveryRequest);
      expect(service.hasActiveRequest()).toBe(true);
      expect(service.error()).toBeNull();
      expect(window.fetch).toHaveBeenCalledWith(
        `${mockDoorwayUrl}/api/recovery/initiate`,
        jasmine.objectContaining({
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: jasmine.any(String),
        })
      );
    });

    it('should include context in request body', async () => {
      mockFetchSuccess(mockRecoveryRequest);

      await service.initiateRecovery('john.doe', 'Lost my device');

      const [, options] = (window.fetch as jasmine.Spy).calls.mostRecent().args;
      const body = JSON.parse(options.body);
      expect(body.claimedIdentity).toBe('john.doe');
      expect(body.context).toBe('Lost my device');
      expect(body.requiredAttestations).toBe(3);
      expect(body.denyThreshold).toBe(2);
    });

    it('should handle API error', async () => {
      mockFetchError('Identity not found');

      const result = await service.initiateRecovery('unknown.user');

      expect(result).toBe(false);
      expect(service.error()).toBe('Identity not found');
      expect(service.activeRequest()).toBeNull();
    });

    it('should set loading state during request', async () => {
      let capturedLoading = false;
      (window.fetch as jasmine.Spy).and.callFake(async () => {
        capturedLoading = service.isLoading();
        return { ok: true, json: () => Promise.resolve(mockRecoveryRequest) };
      });

      await service.initiateRecovery('john.doe');

      expect(capturedLoading).toBe(true);
      expect(service.isLoading()).toBe(false);
    });
  });

  // ===========================================================================
  // Claimant: refreshRequestStatus
  // ===========================================================================

  describe('refreshRequestStatus', () => {
    it('should do nothing if no active request', async () => {
      await service.refreshRequestStatus();
      expect(window.fetch).not.toHaveBeenCalled();
    });

    it('should do nothing if no doorway selected', async () => {
      // First initiate a request
      mockFetchSuccess(mockRecoveryRequest);
      await service.initiateRecovery('john.doe');

      // Then remove doorway
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(null);

      await service.refreshRequestStatus();

      // Only the initiate call should have been made
      expect(window.fetch).toHaveBeenCalledTimes(1);
    });

    it('should update request status', async () => {
      mockFetchSuccess(mockRecoveryRequest);
      await service.initiateRecovery('john.doe');

      const updatedRequest = {
        ...mockRecoveryRequest,
        status: 'interviewing' as const,
        attestations: [
          {
            id: 'att-1',
            requestId: 'recovery-123',
            attesterId: 'attester-1',
            attesterDisplayName: 'Alice',
            decision: 'affirm' as const,
            confidence: 80,
            timestamp: new Date(),
          },
        ],
      };

      mockFetchSuccess(updatedRequest);
      await service.refreshRequestStatus();

      expect(service.activeRequest()?.status).toBe('interviewing');
      expect(service.activeRequest()?.attestations.length).toBe(1);
    });

    it('should fetch credential when status is attested', async () => {
      mockFetchSuccess(mockRecoveryRequest);
      await service.initiateRecovery('john.doe');

      const attestedRequest = { ...mockRecoveryRequest, status: 'attested' as const };
      (window.fetch as jasmine.Spy).and.callFake((url: string) => {
        if (url.includes('/status')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(attestedRequest),
          });
        }
        if (url.includes('/credential')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve(mockCredential),
          });
        }
        return Promise.resolve({ ok: false });
      });

      await service.refreshRequestStatus();

      expect(service.credential()).toEqual(mockCredential);
      expect(service.isRecovered()).toBe(true);
    });
  });

  // ===========================================================================
  // Claimant: cancelRecovery
  // ===========================================================================

  describe('cancelRecovery', () => {
    it('should do nothing if no active request', async () => {
      await service.cancelRecovery();
      expect(window.fetch).not.toHaveBeenCalled();
    });

    it('should cancel active request', async () => {
      mockFetchSuccess(mockRecoveryRequest);
      await service.initiateRecovery('john.doe');

      mockFetchSuccess({});
      await service.cancelRecovery();

      expect(service.activeRequest()).toBeNull();
      expect(service.hasActiveRequest()).toBe(false);
      expect(window.fetch).toHaveBeenCalledWith(
        `${mockDoorwayUrl}/api/recovery/recovery-123/cancel`,
        jasmine.objectContaining({ method: 'POST' })
      );
    });
  });

  // ===========================================================================
  // Claimant: completeRecovery
  // ===========================================================================

  describe('completeRecovery', () => {
    it('should fail if no credential', async () => {
      const result = await service.completeRecovery();

      expect(result).toBe(false);
      expect(service.error()).toBe('No valid credential available');
    });

    it('should fail if credential already claimed', async () => {
      // Manually set a claimed credential
      (service as any)._credential.set({ ...mockCredential, claimed: true });

      const result = await service.completeRecovery();

      expect(result).toBe(false);
      expect(service.error()).toBe('No valid credential available');
    });

    it('should complete recovery successfully', async () => {
      // Setup: initiate and get credential
      (service as any)._credential.set(mockCredential);
      (service as any)._activeRequest.set(mockRecoveryRequest);

      mockFetchSuccess({});
      const result = await service.completeRecovery();

      expect(result).toBe(true);
      expect(service.credential()?.claimed).toBe(true);
      expect(service.activeRequest()).toBeNull();
      expect(window.fetch).toHaveBeenCalledWith(
        `${mockDoorwayUrl}/api/recovery/${mockCredential.requestId}/complete`,
        jasmine.objectContaining({
          method: 'POST',
          body: JSON.stringify({ claimToken: mockCredential.claimToken }),
        })
      );
    });

    it('should handle completion failure', async () => {
      (service as any)._credential.set(mockCredential);
      mockFetchError('Token expired');

      const result = await service.completeRecovery();

      expect(result).toBe(false);
      expect(service.error()).toBe('Failed to complete recovery');
    });
  });

  // ===========================================================================
  // Interviewer: loadPendingRequests
  // ===========================================================================

  describe('loadPendingRequests', () => {
    it('should do nothing if no doorway selected', async () => {
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(null);

      await service.loadPendingRequests();

      expect(window.fetch).not.toHaveBeenCalled();
    });

    it('should load pending requests', async () => {
      mockFetchSuccess({ requests: mockPendingRequests });

      await service.loadPendingRequests();

      expect(service.pendingRequests()).toEqual(mockPendingRequests);
      expect(service.pendingCount()).toBe(1);
      expect(window.fetch).toHaveBeenCalledWith(
        `${mockDoorwayUrl}/api/recovery/queue`,
        jasmine.objectContaining({ credentials: 'include' })
      );
    });

    it('should handle empty requests', async () => {
      mockFetchSuccess({ requests: [] });

      await service.loadPendingRequests();

      expect(service.pendingRequests()).toEqual([]);
      expect(service.pendingCount()).toBe(0);
    });

    it('should set loading state', async () => {
      let capturedLoading = false;
      (window.fetch as jasmine.Spy).and.callFake(async () => {
        capturedLoading = service.isLoading();
        return { ok: true, json: () => Promise.resolve({ requests: [] }) };
      });

      await service.loadPendingRequests();

      expect(capturedLoading).toBe(true);
      expect(service.isLoading()).toBe(false);
    });
  });

  // ===========================================================================
  // Interviewer: startInterview
  // ===========================================================================

  describe('startInterview', () => {
    it('should return false if no doorway selected', async () => {
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(null);

      const result = await service.startInterview('recovery-123');

      expect(result).toBe(false);
    });

    it('should start interview successfully', async () => {
      mockFetchSuccess(mockInterview);

      const result = await service.startInterview('recovery-123');

      expect(result).toBe(true);
      expect(service.conductingInterview()).toEqual(mockInterview);
      expect(window.fetch).toHaveBeenCalledWith(
        `${mockDoorwayUrl}/api/recovery/recovery-123/interview/start`,
        jasmine.objectContaining({
          method: 'POST',
          credentials: 'include',
        })
      );
    });

    it('should handle start failure', async () => {
      mockFetchError('Interview already in progress');

      const result = await service.startInterview('recovery-123');

      expect(result).toBe(false);
      expect(service.error()).toBe('Failed to start interview');
      expect(service.conductingInterview()).toBeNull();
    });
  });

  // ===========================================================================
  // Interviewer: generateQuestions
  // ===========================================================================

  describe('generateQuestions', () => {
    it('should return empty array if no doorway', async () => {
      (mockDoorwayRegistry.selectedUrl as jasmine.Spy).and.returnValue(null);

      const questions = await service.generateQuestions('recovery-123');

      expect(questions).toEqual([]);
    });

    it('should generate questions', async () => {
      mockFetchSuccess({ questions: mockQuestions });

      const questions = await service.generateQuestions('recovery-123');

      expect(questions).toEqual(mockQuestions);
      expect(questions.length).toBe(2);
      expect(questions[0].type).toBe('network-history');
    });

    it('should return empty array on error', async () => {
      mockFetchError('Failed to generate');

      const questions = await service.generateQuestions('recovery-123');

      expect(questions).toEqual([]);
    });
  });

  // ===========================================================================
  // Interviewer: submitResponse
  // ===========================================================================

  describe('submitResponse', () => {
    it('should return null if no conducting interview', async () => {
      const result = await service.submitResponse('q1', 'My answer');

      expect(result).toBeNull();
      expect(window.fetch).not.toHaveBeenCalled();
    });

    it('should submit response and update interview', async () => {
      // Setup conducting interview
      (service as any)._conductingInterview.set(mockInterview);

      const responseData = {
        questionId: 'q1',
        answer: 'January 2024',
        assessment: 'correct' as const,
      };

      mockFetchSuccess({ response: responseData });

      const result = await service.submitResponse('q1', 'January 2024');

      expect(result).toEqual(responseData);
      expect(service.conductingInterview()?.responses).toContain(responseData);
    });

    it('should return null on error', async () => {
      (service as any)._conductingInterview.set(mockInterview);
      mockFetchError('Invalid question');

      const result = await service.submitResponse('q1', 'My answer');

      expect(result).toBeNull();
    });
  });

  // ===========================================================================
  // Interviewer: submitAttestation
  // ===========================================================================

  describe('submitAttestation', () => {
    it('should fail if no conducting interview', async () => {
      const result = await service.submitAttestation('affirm', 90);

      expect(result).toBe(false);
      expect(service.error()).toBe('No active interview');
    });

    it('should submit attestation successfully', async () => {
      (service as any)._conductingInterview.set(mockInterview);

      // Mock for attestation and for refreshing pending requests
      (window.fetch as jasmine.Spy).and.callFake((url: string) => {
        if (url.includes('/attestation')) {
          return Promise.resolve({ ok: true, json: () => Promise.resolve({}) });
        }
        if (url.includes('/queue')) {
          return Promise.resolve({
            ok: true,
            json: () => Promise.resolve({ requests: [] }),
          });
        }
        return Promise.resolve({ ok: false });
      });

      const result = await service.submitAttestation('affirm', 85, 'Strong match');

      expect(result).toBe(true);
      expect(service.conductingInterview()).toBeNull();
    });

    it('should include all parameters in request', async () => {
      (service as any)._conductingInterview.set(mockInterview);

      (window.fetch as jasmine.Spy).and.callFake((url: string, options?: RequestInit) => {
        if (url.includes('/attestation')) {
          const body = JSON.parse(options?.body as string);
          expect(body.interviewId).toBe('interview-123');
          expect(body.decision).toBe('deny');
          expect(body.confidence).toBe(40);
          expect(body.notes).toBe('Suspicious answers');
          return Promise.resolve({ ok: true, json: () => Promise.resolve({}) });
        }
        return Promise.resolve({ ok: true, json: () => Promise.resolve({ requests: [] }) });
      });

      await service.submitAttestation('deny', 40, 'Suspicious answers');
    });

    it('should handle attestation failure', async () => {
      (service as any)._conductingInterview.set(mockInterview);
      mockFetchError('Already attested');

      const result = await service.submitAttestation('affirm', 90);

      expect(result).toBe(false);
      expect(service.error()).toBe('Failed to submit attestation');
    });
  });

  // ===========================================================================
  // Interviewer: abandonInterview
  // ===========================================================================

  describe('abandonInterview', () => {
    it('should clear conducting interview', () => {
      (service as any)._conductingInterview.set(mockInterview);
      expect(service.conductingInterview()).not.toBeNull();

      service.abandonInterview();

      expect(service.conductingInterview()).toBeNull();
    });
  });

  // ===========================================================================
  // Progress Calculation
  // ===========================================================================

  describe('progress computed signal', () => {
    it('should calculate progress from attestations', async () => {
      const requestWithAttestations: RecoveryRequest = {
        ...mockRecoveryRequest,
        attestations: [
          {
            id: 'att-1',
            requestId: 'recovery-123',
            attesterId: 'a1',
            attesterDisplayName: 'Attester 1',
            decision: 'affirm',
            confidence: 80,
            timestamp: new Date(),
          },
          {
            id: 'att-2',
            requestId: 'recovery-123',
            attesterId: 'a2',
            attesterDisplayName: 'Attester 2',
            decision: 'abstain',
            confidence: 50,
            timestamp: new Date(),
          },
        ],
      };

      (service as any)._activeRequest.set(requestWithAttestations);

      const progress = service.progress();

      expect(progress).not.toBeNull();
      expect(progress!.affirmCount).toBe(1);
      expect(progress!.abstainCount).toBe(1);
      expect(progress!.denyCount).toBe(0);
      expect(progress!.requiredCount).toBe(3);
      expect(progress!.thresholdMet).toBe(false);
      expect(progress!.isDenied).toBe(false);
    });

    it('should detect when threshold is met', () => {
      const requestWithThreshold: RecoveryRequest = {
        ...mockRecoveryRequest,
        attestations: [
          {
            id: 'a1',
            requestId: 'r',
            attesterId: 'x',
            attesterDisplayName: 'X',
            decision: 'affirm',
            confidence: 90,
            timestamp: new Date(),
          },
          {
            id: 'a2',
            requestId: 'r',
            attesterId: 'y',
            attesterDisplayName: 'Y',
            decision: 'affirm',
            confidence: 85,
            timestamp: new Date(),
          },
          {
            id: 'a3',
            requestId: 'r',
            attesterId: 'z',
            attesterDisplayName: 'Z',
            decision: 'affirm',
            confidence: 80,
            timestamp: new Date(),
          },
        ],
      };

      (service as any)._activeRequest.set(requestWithThreshold);

      expect(service.progress()!.thresholdMet).toBe(true);
      expect(service.progress()!.progressPercent).toBe(100);
    });

    it('should detect when denied', () => {
      const deniedRequest: RecoveryRequest = {
        ...mockRecoveryRequest,
        attestations: [
          {
            id: 'a1',
            requestId: 'r',
            attesterId: 'x',
            attesterDisplayName: 'X',
            decision: 'deny',
            confidence: 90,
            timestamp: new Date(),
          },
          {
            id: 'a2',
            requestId: 'r',
            attesterId: 'y',
            attesterDisplayName: 'Y',
            decision: 'deny',
            confidence: 85,
            timestamp: new Date(),
          },
        ],
      };

      (service as any)._activeRequest.set(deniedRequest);

      expect(service.progress()!.isDenied).toBe(true);
      expect(service.progress()!.denyCount).toBe(2);
    });
  });

  // ===========================================================================
  // Utility Methods
  // ===========================================================================

  describe('clearError', () => {
    it('should clear error state', () => {
      (service as any)._error.set('Some error');
      expect(service.error()).toBe('Some error');

      service.clearError();

      expect(service.error()).toBeNull();
    });
  });

  describe('reset', () => {
    it('should reset all state', async () => {
      // Setup state
      (service as any)._activeRequest.set(mockRecoveryRequest);
      (service as any)._activeInterview.set(mockInterview);
      (service as any)._credential.set(mockCredential);
      (service as any)._pendingRequests.set(mockPendingRequests);
      (service as any)._conductingInterview.set(mockInterview);
      (service as any)._error.set('Some error');

      service.reset();

      expect(service.activeRequest()).toBeNull();
      expect(service.activeInterview()).toBeNull();
      expect(service.credential()).toBeNull();
      expect(service.pendingRequests()).toEqual([]);
      expect(service.conductingInterview()).toBeNull();
      expect(service.error()).toBeNull();
      expect(service.hasActiveRequest()).toBe(false);
      expect(service.isRecovered()).toBe(false);
      expect(service.pendingCount()).toBe(0);
    });
  });

  // ===========================================================================
  // Network Error Handling
  // ===========================================================================

  describe('network error handling', () => {
    it('should handle fetch exceptions', async () => {
      (window.fetch as jasmine.Spy).and.rejectWith(new Error('Network failure'));

      const result = await service.initiateRecovery('john.doe');

      expect(result).toBe(false);
      expect(service.error()).toBe('Network failure');
    });

    it('should clear loading state on exception', async () => {
      (window.fetch as jasmine.Spy).and.rejectWith(new Error('Network failure'));

      await service.initiateRecovery('john.doe');

      expect(service.isLoading()).toBe(false);
    });
  });
});
