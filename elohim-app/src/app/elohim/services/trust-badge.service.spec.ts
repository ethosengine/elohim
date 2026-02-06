import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { of, throwError } from 'rxjs';

import { TrustBadgeService } from './trust-badge.service';
import { DataLoaderService } from './data-loader.service';
import { ContentService } from '@app/lamad/services/content.service';
import { AgentService } from './agent.service';
import { ContentNode } from '@app/lamad/models/content-node.model';
import { ContentAttestation } from '@app/lamad/models/content-attestation.model';

/**
 * Comprehensive tests for TrustBadgeService
 *
 * Tests coverage:
 * - Badge computation with various attestation combinations
 * - Compact badge generation
 * - Bulk badge operations
 * - Trust indicators
 * - Reach level helpers
 * - Error handling
 * - Available actions based on agent attestations
 */
describe('TrustBadgeService', () => {
  let service: TrustBadgeService;
  let dataLoaderMock: jasmine.SpyObj<DataLoaderService>;
  let contentServiceMock: jasmine.SpyObj<ContentService>;
  let agentServiceMock: jasmine.SpyObj<AgentService>;

  const mockContent: ContentNode = {
    id: 'content-1',
    title: 'Test Content',
    description: 'Test description',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: 'Test body',
    tags: [],
    relatedNodeIds: [],
    metadata: {},
    reach: 'commons',
    trustScore: 0.75,
    authorId: 'author-1',
    createdAt: '2024-01-01T00:00:00Z',
    updatedAt: '2024-01-01T00:00:00Z',
  };

  const mockAttestation: ContentAttestation = {
    id: 'attestation-1',
    contentId: 'content-1',
    attestationType: 'peer-reviewed',
    reachGranted: 'bioregional',
    grantedBy: {
      type: 'steward',
      grantorId: 'grantor-1',
      grantorName: 'Expert Reviewer',
    },
    grantedAt: '2024-01-01T00:00:00Z',
    status: 'active',
  };

  beforeEach(() => {
    const dataLoaderSpy = jasmine.createSpyObj('DataLoaderService', [
      'getAttestationsForContent',
    ]);
    const contentServiceSpy = jasmine.createSpyObj('ContentService', ['getContent']);
    const agentServiceSpy = jasmine.createSpyObj('AgentService', [
      'getCurrentAgentId',
      'getAttestations',
    ]);

    TestBed.configureTestingModule({
      providers: [
        TrustBadgeService,
        { provide: DataLoaderService, useValue: dataLoaderSpy },
        { provide: ContentService, useValue: contentServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
      ],
    });

    service = TestBed.inject(TrustBadgeService);
    dataLoaderMock = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    contentServiceMock = TestBed.inject(ContentService) as jasmine.SpyObj<ContentService>;
    agentServiceMock = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;
  });

  // ===========================================================================
  // Service Creation
  // ===========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });
  });

  // ===========================================================================
  // getBadge - Full Badge Generation
  // ===========================================================================

  describe('getBadge', () => {
    it('should generate badge for content without attestations', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([]));
      agentServiceMock.getCurrentAgentId.and.returnValue('agent-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let badge: any = null;
      service.getBadge('content-1').subscribe(b => {
        badge = b;
      });
      tick();

      expect(badge).toBeTruthy();
      expect(badge.contentId).toBe('content-1');
      expect(badge.reach).toBe('commons');
      expect(badge.primary.type).toBe('reach');
      expect(badge.hasWarnings).toBe(false);
    }));

    it('should generate badge with peer-reviewed attestation', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([mockAttestation]));
      agentServiceMock.getCurrentAgentId.and.returnValue('agent-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let badge: any = null;
      service.getBadge('content-1').subscribe(b => {
        badge = b;
      });
      tick();

      expect(badge.primary.attestationType).toBe('peer-reviewed');
      expect(badge.primary.grantedBy).toBe('Expert Reviewer');
      expect(badge.secondary.length).toBeGreaterThan(0);
    }));

    it('should generate badge with multiple attestations', fakeAsync(() => {
      const attestations: ContentAttestation[] = [
        { ...mockAttestation, id: 'att-1', attestationType: 'peer-reviewed' },
        { ...mockAttestation, id: 'att-2', attestationType: 'community-endorsed' },
        { ...mockAttestation, id: 'att-3', attestationType: 'author-verified' },
      ];

      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of(attestations));
      agentServiceMock.getCurrentAgentId.and.returnValue('agent-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let badge: any = null;
      service.getBadge('content-1').subscribe(b => {
        badge = b;
      });
      tick();

      expect(badge.primary.attestationType).toBe('peer-reviewed');
      expect(badge.secondary.length).toBeGreaterThanOrEqual(2);
    }));

    it('should generate badge with governance-ratified as primary', fakeAsync(() => {
      const govAttestation: ContentAttestation = {
        ...mockAttestation,
        attestationType: 'governance-ratified',
      };

      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([govAttestation]));
      agentServiceMock.getCurrentAgentId.and.returnValue('agent-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let badge: any = null;
      service.getBadge('content-1').subscribe(b => {
        badge = b;
      });
      tick();

      expect(badge.primary.attestationType).toBe('governance-ratified');
      expect(badge.trustLevel).not.toBe('unverified');
    }));

    it('should include warnings for flagged content', fakeAsync(() => {
      const flaggedContent = {
        ...mockContent,
        flags: [
          {
            type: 'outdated' as const,
            reason: 'Content needs updating',
            flaggedAt: '2024-01-01T00:00:00Z',
            flaggedBy: 'system',
          },
        ],
      };

      contentServiceMock.getContent.and.returnValue(of(flaggedContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([]));
      agentServiceMock.getCurrentAgentId.and.returnValue('agent-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let badge: any = null;
      service.getBadge('content-1').subscribe(b => {
        badge = b;
      });
      tick();

      expect(badge.hasWarnings).toBe(true);
      expect(badge.warnings.length).toBe(1);
      expect(badge.warnings[0].type).toBe('outdated');
    }));

    it('should include available actions for author', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([]));
      agentServiceMock.getCurrentAgentId.and.returnValue('author-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let badge: any = null;
      service.getBadge('content-1').subscribe(b => {
        badge = b;
      });
      tick();

      const requestAction = badge.actions.find((a: any) => a.action === 'request-attestation');
      expect(requestAction).toBeTruthy();
      expect(requestAction.available).toBe(true);
    }));

    it('should handle error gracefully', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(throwError(() => new Error('Network error')));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([]));

      let badge: any = null;
      service.getBadge('content-1').subscribe(b => {
        badge = b;
      });
      tick();

      expect(badge).toBeTruthy();
      expect(badge.trustLevel).toBe('unverified');
    }));
  });

  // ===========================================================================
  // getCompactBadge
  // ===========================================================================

  describe('getCompactBadge', () => {
    it('should generate compact badge', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([mockAttestation]));
      agentServiceMock.getCurrentAgentId.and.returnValue('agent-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let compactBadge: any = null;
      service.getCompactBadge('content-1').subscribe(b => {
        compactBadge = b;
      });
      tick();

      expect(compactBadge).toBeTruthy();
      expect(compactBadge.icon).toBeDefined();
      expect(compactBadge.color).toBeDefined();
      expect(compactBadge.tooltip).toBeDefined();
    }));
  });

  // ===========================================================================
  // getBadgesForContent - Bulk Operations
  // ===========================================================================

  describe('getBadgesForContent', () => {
    it('should return empty map for empty array', fakeAsync(() => {
      let badges: any = null;
      service.getBadgesForContent([]).subscribe(b => {
        badges = b;
      });
      tick();

      expect(badges.size).toBe(0);
    }));

    it('should fetch badges for multiple content IDs', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([]));
      agentServiceMock.getCurrentAgentId.and.returnValue('agent-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let badges: any = null;
      service.getBadgesForContent(['content-1', 'content-2']).subscribe(b => {
        badges = b;
      });
      tick();

      expect(badges.size).toBe(2);
      expect(badges.get('content-1')).toBeTruthy();
      expect(badges.get('content-2')).toBeTruthy();
    }));
  });

  describe('getCompactBadgesForContent', () => {
    it('should fetch compact badges for multiple content IDs', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([mockAttestation]));
      agentServiceMock.getCurrentAgentId.and.returnValue('agent-1');
      agentServiceMock.getAttestations.and.returnValue([]);

      let badges: any = null;
      service.getCompactBadgesForContent(['content-1', 'content-2']).subscribe(b => {
        badges = b;
      });
      tick();

      expect(badges.size).toBe(2);
      expect(badges.get('content-1')?.icon).toBeDefined();
    }));
  });

  // ===========================================================================
  // getIndicators - Unified Trust Indicators
  // ===========================================================================

  describe('getIndicators', () => {
    it('should generate indicators for content', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([mockAttestation]));

      let indicators: any = null;
      service.getIndicators('content-1').subscribe(i => {
        indicators = i;
      });
      tick();

      expect(indicators).toBeTruthy();
      expect(indicators.contentId).toBe('content-1');
      expect(indicators.indicators.length).toBeGreaterThan(0);
      expect(indicators.badges.length).toBeGreaterThan(0);
    }));

    it('should include flags as negative indicators', fakeAsync(() => {
      const flaggedContent = {
        ...mockContent,
        flags: [
          {
            type: 'disputed' as const,
            reason: 'Content disputed',
            flaggedAt: '2024-01-01T00:00:00Z',
            flaggedBy: 'reviewer',
          },
        ],
      };

      contentServiceMock.getContent.and.returnValue(of(flaggedContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([mockAttestation]));

      let indicators: any = null;
      service.getIndicators('content-1').subscribe(i => {
        indicators = i;
      });
      tick();

      expect(indicators.flags.length).toBe(1);
      const negativeIndicators = indicators.indicators.filter((ind: any) => ind.polarity === 'negative');
      expect(negativeIndicators.length).toBeGreaterThan(0);
    }));

    it('should prioritize flags over badges', fakeAsync(() => {
      const flaggedContent = {
        ...mockContent,
        flags: [
          {
            type: 'outdated' as const,
            reason: 'Content needs updating',
            flaggedAt: '2024-01-01T00:00:00Z',
            flaggedBy: 'system',
          },
        ],
      };

      contentServiceMock.getContent.and.returnValue(of(flaggedContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([mockAttestation]));

      let indicators: any = null;
      service.getIndicators('content-1').subscribe(i => {
        indicators = i;
      });
      tick();

      // Primary should be the flag (negative indicator)
      expect(indicators.primary?.polarity).toBe('negative');
    }));

    it('should handle error and return empty indicator set', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(throwError(() => new Error('Error')));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([]));

      let indicators: any = null;
      service.getIndicators('content-1').subscribe(i => {
        indicators = i;
      });
      tick();

      expect(indicators.indicators.length).toBe(0);
      expect(indicators.trustLevel).toBe('unverified');
    }));
  });

  describe('getIndicatorsForContent', () => {
    it('should return empty map for empty array', fakeAsync(() => {
      let indicators: any = null;
      service.getIndicatorsForContent([]).subscribe(i => {
        indicators = i;
      });
      tick();

      expect(indicators.size).toBe(0);
    }));

    it('should fetch indicators for multiple content IDs', fakeAsync(() => {
      contentServiceMock.getContent.and.returnValue(of(mockContent));
      dataLoaderMock.getAttestationsForContent.and.returnValue(of([]));

      let indicators: any = null;
      service.getIndicatorsForContent(['content-1', 'content-2']).subscribe(i => {
        indicators = i;
      });
      tick();

      expect(indicators.size).toBe(2);
    }));
  });

  // ===========================================================================
  // Reach Level Helpers
  // ===========================================================================

  describe('meetsReachRequirement', () => {
    it('should return true when content reach meets requirement', () => {
      expect(service.meetsReachRequirement('commons', 'local')).toBe(true);
      expect(service.meetsReachRequirement('regional', 'municipal')).toBe(true);
    });

    it('should return false when content reach does not meet requirement', () => {
      expect(service.meetsReachRequirement('local', 'commons')).toBe(false);
      expect(service.meetsReachRequirement('private', 'municipal')).toBe(false);
    });

    it('should return true for same reach level', () => {
      expect(service.meetsReachRequirement('commons', 'commons')).toBe(true);
    });
  });

  describe('getNextReachLevel', () => {
    it('should return next reach level', () => {
      expect(service.getNextReachLevel('private')).toBe('invited');
      expect(service.getNextReachLevel('local')).toBe('neighborhood');
      expect(service.getNextReachLevel('regional')).toBe('commons');
    });

    it('should return null for commons (max level)', () => {
      expect(service.getNextReachLevel('commons')).toBeNull();
    });
  });

  describe('getAttestationsNeededForNextLevel', () => {
    it('should return attestations needed for next level', () => {
      const needed = service.getAttestationsNeededForNextLevel('private', []);
      expect(needed.length).toBeGreaterThan(0);
    });

    it('should filter out already obtained attestations', () => {
      const needed = service.getAttestationsNeededForNextLevel('private', ['author-verified']);
      expect(needed).not.toContain('author-verified');
    });

    it('should return empty array for commons', () => {
      const needed = service.getAttestationsNeededForNextLevel('commons', []);
      expect(needed.length).toBe(0);
    });

    it('should return empty array when all attestations obtained', () => {
      const allAttestations: import('@app/lamad/models/content-attestation.model').ContentAttestationType[] = ['author-verified'];
      const needed = service.getAttestationsNeededForNextLevel('private', allAttestations);
      expect(needed.length).toBe(0);
    });
  });
});
