import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { of, throwError } from 'rxjs';

import { HumanConsentService } from './human-consent.service';
import { LocalSourceChainService } from './local-source-chain.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { HumanConsent } from '@app/elohim/models/human-consent.model';
import { LearningPath } from '@app/lamad/models/learning-path.model';

/**
 * Comprehensive tests for HumanConsentService
 *
 * Tests coverage:
 * - Consent creation (recognition, connection)
 * - Consent state management (accept, decline, revoke)
 * - Intimacy level elevation
 * - Visibility checks for learning paths
 * - Query methods
 * - Error handling
 */
describe('HumanConsentService', () => {
  let service: HumanConsentService;
  let sourceChainMock: jasmine.SpyObj<LocalSourceChainService>;
  let sessionHumanMock: jasmine.SpyObj<SessionHumanService>;

  const mockAgentId = 'agent-123';

  beforeEach(() => {
    const sourceChainSpy = jasmine.createSpyObj('LocalSourceChainService', [
      'isInitialized',
      'getAgentId',
      'getEntriesByType',
      'createEntry',
    ]);
    const sessionHumanSpy = jasmine.createSpyObj('SessionHumanService', ['getCurrentHuman']);

    sourceChainSpy.isInitialized.and.returnValue(true);
    sourceChainSpy.getAgentId.and.returnValue(mockAgentId);
    sourceChainSpy.getEntriesByType.and.returnValue([]);

    TestBed.configureTestingModule({
      providers: [
        HumanConsentService,
        { provide: LocalSourceChainService, useValue: sourceChainSpy },
        { provide: SessionHumanService, useValue: sessionHumanSpy },
      ],
    });

    service = TestBed.inject(HumanConsentService);
    sourceChainMock = TestBed.inject(LocalSourceChainService) as jasmine.SpyObj<LocalSourceChainService>;
    sessionHumanMock = TestBed.inject(SessionHumanService) as jasmine.SpyObj<SessionHumanService>;
  });

  // ===========================================================================
  // Service Creation & Initialization
  // ===========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should initialize without errors', () => {
      expect(() => service.initialize()).not.toThrow();
    });

    it('should not load consents if source chain not initialized', () => {
      sourceChainMock.isInitialized.and.returnValue(false);
      service.initialize();
      expect(sourceChainMock.getEntriesByType).not.toHaveBeenCalled();
    });
  });

  // ===========================================================================
  // sendRecognition
  // ===========================================================================

  describe('sendRecognition', () => {
    it('should create recognition consent', fakeAsync(() => {
      let result: HumanConsent | null = null;
      service.sendRecognition('target-human').subscribe(consent => {
        result = consent;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result!.initiatorId).toBe(mockAgentId);
      expect(result!.participantId).toBe('target-human');
      expect(result!.intimacyLevel).toBe('recognition');
      expect(result!.consentState).toBe('not_required');
      expect(sourceChainMock.createEntry).toHaveBeenCalled();
    }));

    it('should include optional note', fakeAsync(() => {
      let result: HumanConsent | null = null;
      service.sendRecognition('target-human', 'Great work!').subscribe(consent => {
        result = consent;
      });
      tick();

      expect(result!.requestMessage).toBe('Great work!');
    }));

    it('should reject sending recognition to self', fakeAsync(() => {
      let error: Error | null = null;
      service.sendRecognition(mockAgentId).subscribe({
        error: err => {
          error = err;
        },
      });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('yourself');
    }));

    it('should reject if relationship already exists', fakeAsync(() => {
      // First create a consent
      service.sendRecognition('target-human').subscribe();
      tick();

      // Try to create another
      let error: Error | null = null;
      service.sendRecognition('target-human').subscribe({
        error: err => {
          error = err;
        },
      });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('already exists');
    }));
  });

  // ===========================================================================
  // requestConnection
  // ===========================================================================

  describe('requestConnection', () => {
    it('should create connection request', fakeAsync(() => {
      let result: HumanConsent | null = null;
      service.requestConnection('target-human', 'Want to connect').subscribe(consent => {
        result = consent;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result!.intimacyLevel).toBe('connection');
      expect(result!.consentState).toBe('pending');
      expect(result!.requestMessage).toBe('Want to connect');
    }));

    it('should reject connection to self', fakeAsync(() => {
      let error: Error | null = null;
      service.requestConnection(mockAgentId).subscribe({
        error: err => {
          error = err;
        },
      });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('yourself');
    }));

    it('should allow upgrading from recognition to connection', fakeAsync(() => {
      // First send recognition
      service.sendRecognition('target-human').subscribe();
      tick();

      // Then request connection upgrade
      let result: HumanConsent | null = null;
      service.requestConnection('target-human', 'Upgrade?').subscribe(consent => {
        result = consent;
      });
      tick();

      expect(result!.intimacyLevel).toBe('connection');
      expect(result!.consentState).toBe('pending');
    }));
  });

  // ===========================================================================
  // acceptConsent
  // ===========================================================================

  describe('acceptConsent', () => {
    it('should accept pending consent request', fakeAsync(() => {
      // Create a pending consent from another user
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: 'other-human',
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: 'other-human',
          participantId: mockAgentId,
          intimacyLevel: 'connection',
          consentState: 'pending',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let result: HumanConsent | null = null;
      service.acceptConsent('consent-1').subscribe(consent => {
        result = consent;
      });
      tick();

      expect(result!.consentState).toBe('accepted');
      expect(result!.consentedAt).toBeDefined();
    }));

    it('should reject if consent not found', fakeAsync(() => {
      let error: Error | null = null;
      service.acceptConsent('nonexistent').subscribe({
        error: err => {
          error = err;
        },
      });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('not found');
    }));

    it('should reject if not the participant', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: mockAgentId,
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: mockAgentId,
          participantId: 'other-human',
          intimacyLevel: 'connection',
          consentState: 'pending',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let error: Error | null = null;
      service.acceptConsent('consent-1').subscribe({
        error: err => {
          error = err;
        },
      });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('participant');
    }));

    it('should reject if consent not pending', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: 'other-human',
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: 'other-human',
          participantId: mockAgentId,
          intimacyLevel: 'connection',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let error: Error | null = null;
      service.acceptConsent('consent-1').subscribe({
        error: err => {
          error = err;
        },
      });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('not pending');
    }));
  });

  // ===========================================================================
  // declineConsent
  // ===========================================================================

  describe('declineConsent', () => {
    it('should decline pending consent', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: 'other-human',
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: 'other-human',
          participantId: mockAgentId,
          intimacyLevel: 'connection',
          consentState: 'pending',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let completed = false;
      service.declineConsent('consent-1', 'Not interested').subscribe(() => {
        completed = true;
      });
      tick();

      expect(completed).toBe(true);
      expect(sourceChainMock.createEntry).toHaveBeenCalled();
    }));
  });

  // ===========================================================================
  // revokeConsent
  // ===========================================================================

  describe('revokeConsent', () => {
    it('should allow initiator to revoke accepted consent', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: mockAgentId,
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: mockAgentId,
          participantId: 'other-human',
          intimacyLevel: 'connection',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let completed = false;
      service.revokeConsent('consent-1', 'Changed my mind').subscribe(() => {
        completed = true;
      });
      tick();

      expect(completed).toBe(true);
    }));

    it('should allow participant to revoke accepted consent', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: 'other-human',
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: 'other-human',
          participantId: mockAgentId,
          intimacyLevel: 'connection',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let completed = false;
      service.revokeConsent('consent-1').subscribe(() => {
        completed = true;
      });
      tick();

      expect(completed).toBe(true);
    }));

    it('should reject if not a participant', fakeAsync(() => {
      // Create a consent where current agent is neither initiator nor participant
      // This tests that we can't revoke someone else's consent
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: 'human-a',
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: 'human-a',
          participantId: 'human-b',
          intimacyLevel: 'connection',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let error: Error | null = null;
      service.revokeConsent('consent-1').subscribe({
        error: err => {
          error = err;
        },
      });
      tick();

      // Since mockAgentId is neither initiator nor participant,
      // the consent won't be loaded during initialize(), so we get "not found"
      expect(error).toBeTruthy();
      expect(error!.message).toContain('not found');
    }));
  });

  // ===========================================================================
  // proposeElevation
  // ===========================================================================

  describe('proposeElevation', () => {
    it('should propose elevation to higher intimacy level', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: mockAgentId,
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: mockAgentId,
          participantId: 'other-human',
          intimacyLevel: 'connection',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let result: HumanConsent | null = null;
      service
        .proposeElevation({
          consentId: 'consent-1',
          newLevel: 'trusted',
          message: 'Let\'s deepen our relationship',
        })
        .subscribe(consent => {
          result = consent;
        });
      tick();

      expect(result!.intimacyLevel).toBe('trusted');
      expect(result!.consentState).toBe('pending');
      expect(result!.elevationAttempts).toBeGreaterThan(0);
    }));

    it('should reject elevation to same level', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: mockAgentId,
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: mockAgentId,
          participantId: 'other-human',
          intimacyLevel: 'connection',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let error: Error | null = null;
      service
        .proposeElevation({
          consentId: 'consent-1',
          newLevel: 'connection',
        })
        .subscribe({
          error: err => {
            error = err;
          },
        });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('same or lower');
    }));

    it('should require attestation type for intimate level', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: mockAgentId,
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: mockAgentId,
          participantId: 'other-human',
          intimacyLevel: 'trusted',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let error: Error | null = null;
      service
        .proposeElevation({
          consentId: 'consent-1',
          newLevel: 'intimate',
        })
        .subscribe({
          error: err => {
            error = err;
          },
        });
      tick();

      expect(error).toBeTruthy();
      expect(error!.message).toContain('attestation');
    }));
  });

  // ===========================================================================
  // Query Methods
  // ===========================================================================

  describe('getConsentWith', () => {
    it('should find consent with specific human', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: mockAgentId,
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: mockAgentId,
          participantId: 'target-human',
          intimacyLevel: 'connection',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      let result: HumanConsent | null = null;
      service.getConsentWith('target-human').subscribe(consent => {
        result = consent;
      });
      tick();

      expect(result).toBeTruthy();
      expect(result!.participantId).toBe('target-human');
    }));

    it('should return null if no consent exists', fakeAsync(() => {
      let result: HumanConsent | null | undefined = undefined;
      service.getConsentWith('unknown-human').subscribe(consent => {
        result = consent;
      });
      tick();

      expect(result).toBeNull();
    }));
  });

  describe('getMyConsents', () => {
    it('should return all consents', fakeAsync(() => {
      const mockEntries: any[] = [
        {
          id: 'entry-1',
          entryHash: 'hash-1',
          authorAgent: mockAgentId,
          entryType: 'human-consent',
          timestamp: '2024-01-01T00:00:00Z',
          content: {
            consentId: 'consent-1',
            initiatorId: mockAgentId,
            participantId: 'human-1',
            intimacyLevel: 'connection',
            consentState: 'accepted',
            updatedAt: '2024-01-01T00:00:00Z',
          },
        },
        {
          id: 'entry-2',
          entryHash: 'hash-2',
          authorAgent: 'human-2',
          entryType: 'human-consent',
          timestamp: '2024-01-02T00:00:00Z',
          content: {
            consentId: 'consent-2',
            initiatorId: 'human-2',
            participantId: mockAgentId,
            intimacyLevel: 'trusted',
            consentState: 'accepted',
            updatedAt: '2024-01-02T00:00:00Z',
          },
        },
      ];
      sourceChainMock.getEntriesByType.and.returnValue(mockEntries);
      service.initialize();

      let result: HumanConsent[] = [];
      service.getMyConsents().subscribe(consents => {
        result = consents;
      });
      tick();

      expect(result.length).toBe(2);
    }));

    it('should filter by intimacy level', fakeAsync(() => {
      const mockEntries: any[] = [
        {
          id: 'entry-1',
          entryHash: 'hash-1',
          authorAgent: mockAgentId,
          entryType: 'human-consent',
          timestamp: '2024-01-01T00:00:00Z',
          content: {
            consentId: 'consent-1',
            initiatorId: mockAgentId,
            participantId: 'human-1',
            intimacyLevel: 'connection',
            consentState: 'accepted',
            updatedAt: '2024-01-01T00:00:00Z',
          },
        },
        {
          id: 'entry-2',
          entryHash: 'hash-2',
          authorAgent: mockAgentId,
          entryType: 'human-consent',
          timestamp: '2024-01-02T00:00:00Z',
          content: {
            consentId: 'consent-2',
            initiatorId: mockAgentId,
            participantId: 'human-2',
            intimacyLevel: 'trusted',
            consentState: 'accepted',
            updatedAt: '2024-01-02T00:00:00Z',
          },
        },
      ];
      sourceChainMock.getEntriesByType.and.returnValue(mockEntries);
      service.initialize();

      let result: HumanConsent[] = [];
      service.getMyConsents({ level: 'trusted' }).subscribe(consents => {
        result = consents;
      });
      tick();

      expect(result.length).toBe(1);
      expect(result[0].intimacyLevel).toBe('trusted');
    }));
  });

  describe('getPendingRequests', () => {
    it('should return only pending requests for current user', fakeAsync(() => {
      const mockEntries: any[] = [
        {
          id: 'entry-1',
          entryHash: 'hash-1',
          authorAgent: 'other-human',
          entryType: 'human-consent',
          timestamp: '2024-01-01T00:00:00Z',
          content: {
            consentId: 'consent-1',
            initiatorId: 'other-human',
            participantId: mockAgentId,
            intimacyLevel: 'connection',
            consentState: 'pending',
            updatedAt: '2024-01-01T00:00:00Z',
          },
        },
        {
          id: 'entry-2',
          entryHash: 'hash-2',
          authorAgent: mockAgentId,
          entryType: 'human-consent',
          timestamp: '2024-01-02T00:00:00Z',
          content: {
            consentId: 'consent-2',
            initiatorId: mockAgentId,
            participantId: 'other-human-2',
            intimacyLevel: 'connection',
            consentState: 'pending',
            updatedAt: '2024-01-02T00:00:00Z',
          },
        },
      ];
      sourceChainMock.getEntriesByType.and.returnValue(mockEntries);
      service.initialize();

      let result: HumanConsent[] = [];
      service.getPendingRequests().subscribe(consents => {
        result = consents;
      });
      tick();

      expect(result.length).toBe(1);
      expect(result[0].participantId).toBe(mockAgentId);
    }));
  });

  describe('getHumansAtLevel', () => {
    it('should return humans at or above specified level', fakeAsync(() => {
      const mockEntries: any[] = [
        {
          id: 'entry-1',
          entryHash: 'hash-1',
          authorAgent: mockAgentId,
          entryType: 'human-consent',
          timestamp: '2024-01-01T00:00:00Z',
          content: {
            consentId: 'consent-1',
            initiatorId: mockAgentId,
            participantId: 'human-1',
            intimacyLevel: 'connection',
            consentState: 'accepted',
            updatedAt: '2024-01-01T00:00:00Z',
          },
        },
        {
          id: 'entry-2',
          entryHash: 'hash-2',
          authorAgent: 'human-2',
          entryType: 'human-consent',
          timestamp: '2024-01-02T00:00:00Z',
          content: {
            consentId: 'consent-2',
            initiatorId: 'human-2',
            participantId: mockAgentId,
            intimacyLevel: 'trusted',
            consentState: 'accepted',
            updatedAt: '2024-01-02T00:00:00Z',
          },
        },
      ];
      sourceChainMock.getEntriesByType.and.returnValue(mockEntries);
      service.initialize();

      let result: string[] = [];
      service.getHumansAtLevel('connection').subscribe(humans => {
        result = humans;
      });
      tick();

      expect(result.length).toBe(2);
      expect(result).toContain('human-1');
      expect(result).toContain('human-2');
    }));
  });

  // ===========================================================================
  // Visibility Checks
  // ===========================================================================

  describe('canViewPath', () => {
    const mockPath: LearningPath = {
      id: 'path-1',
      version: '1.0.0',
      title: 'Test Path',
      description: 'Test',
      purpose: 'Learning',
      difficulty: 'beginner',
      estimatedDuration: '2 hours',
      visibility: 'public',
      pathType: 'journey',
      tags: [],
      createdBy: 'creator-1',
      contributors: [],
      steps: [],
      createdAt: '2024-01-01T00:00:00Z',
      updatedAt: '2024-01-01T00:00:00Z',
    };

    it('should allow viewing public paths', fakeAsync(() => {
      let canView = false;
      service.canViewPath(mockPath).subscribe(result => {
        canView = result;
      });
      tick();

      expect(canView).toBe(true);
    }));

    it('should allow creator to view own paths', fakeAsync(() => {
      const privatePath = { ...mockPath, visibility: 'private' as const, createdBy: mockAgentId };

      let canView = false;
      service.canViewPath(privatePath).subscribe(result => {
        canView = result;
      });
      tick();

      expect(canView).toBe(true);
    }));

    it('should allow viewing if in participantIds', fakeAsync(() => {
      const intimatePath = {
        ...mockPath,
        visibility: 'intimate' as const,
        participantIds: [mockAgentId],
      };

      let canView = false;
      service.canViewPath(intimatePath).subscribe(result => {
        canView = result;
      });
      tick();

      expect(canView).toBe(true);
    }));

    it('should check consent level for private paths', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: mockAgentId,
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: mockAgentId,
          participantId: 'creator-1',
          intimacyLevel: 'trusted',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      const trustedPath = { ...mockPath, visibility: 'trusted' as const, createdBy: 'creator-1' };

      let canView = false;
      service.canViewPath(trustedPath).subscribe(result => {
        canView = result;
      });
      tick();

      expect(canView).toBe(true);
    }));

    it('should deny viewing if consent level insufficient', fakeAsync(() => {
      const mockEntry: any = {
        id: 'entry-1',
        entryHash: 'hash-1',
        authorAgent: mockAgentId,
        entryType: 'human-consent',
        timestamp: '2024-01-01T00:00:00Z',
        content: {
          consentId: 'consent-1',
          initiatorId: mockAgentId,
          participantId: 'creator-1',
          intimacyLevel: 'connection',
          consentState: 'accepted',
          updatedAt: '2024-01-01T00:00:00Z',
        },
      };
      sourceChainMock.getEntriesByType.and.returnValue([mockEntry]);
      service.initialize();

      const intimatePath = { ...mockPath, visibility: 'intimate' as const, createdBy: 'creator-1' };

      let canView = true;
      service.canViewPath(intimatePath).subscribe(result => {
        canView = result;
      });
      tick();

      expect(canView).toBe(false);
    }));
  });
});
