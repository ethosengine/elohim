import { TestBed } from '@angular/core/testing';
import { of, throwError } from 'rxjs';
import { TrustBadgeService } from './trust-badge.service';
import { DataLoaderService } from '@app/elohim/services/data-loader.service';
import { ContentService } from './content.service';
import { AgentService } from '@app/elohim/services/agent.service';
import { ContentNode, ContentReach } from '../models/content-node.model';
import { ContentAttestation } from '../models/content-attestation.model';

describe('TrustBadgeService', () => {
  let service: TrustBadgeService;
  let dataLoaderSpy: jasmine.SpyObj<DataLoaderService>;
  let contentServiceSpy: jasmine.SpyObj<ContentService>;
  let agentServiceSpy: jasmine.SpyObj<AgentService>;

  const mockContent: ContentNode = {
    id: 'content-1',
    title: 'Test Content',
    description: 'Test description',
    contentType: 'concept',
    contentFormat: 'markdown',
    content: '# Test',
    tags: [],
    relatedNodeIds: [],
    metadata: {},
    reach: 'regional',
    trustScore: 0.85
  };

  const mockContentWithFlags: ContentNode = {
    ...mockContent,
    id: 'flagged-content',
    flags: [
      { type: 'disputed', flaggedAt: '2025-01-01T00:00:00.000Z', reason: 'Needs update' }
    ]
  };

  const mockAttestations: ContentAttestation[] = [
    {
      id: 'att-1',
      contentId: 'content-1',
      attestationType: 'peer-reviewed',
      reachGranted: 'bioregional',
      grantedBy: { type: 'community', grantorId: 'reviewer-1', grantorName: 'Expert Reviewer' },
      grantedAt: '2025-01-01T00:00:00.000Z',
      status: 'active',
      metadata: {}
    },
    {
      id: 'att-2',
      contentId: 'content-1',
      attestationType: 'community-endorsed',
      reachGranted: 'neighborhood',
      grantedBy: { type: 'community', grantorId: 'community', grantorName: 'Community' },
      grantedAt: '2025-01-02T00:00:00.000Z',
      status: 'active',
      metadata: {}
    }
  ];

  beforeEach(() => {
    const dataLoaderSpyObj = jasmine.createSpyObj('DataLoaderService', ['getAttestationsForContent']);
    const contentServiceSpyObj = jasmine.createSpyObj('ContentService', ['getContent']);
    const agentServiceSpyObj = jasmine.createSpyObj('AgentService', ['getCurrentAgentId', 'getAttestations']);

    TestBed.configureTestingModule({
      providers: [
        TrustBadgeService,
        { provide: DataLoaderService, useValue: dataLoaderSpyObj },
        { provide: ContentService, useValue: contentServiceSpyObj },
        { provide: AgentService, useValue: agentServiceSpyObj }
      ]
    });

    dataLoaderSpy = TestBed.inject(DataLoaderService) as jasmine.SpyObj<DataLoaderService>;
    contentServiceSpy = TestBed.inject(ContentService) as jasmine.SpyObj<ContentService>;
    agentServiceSpy = TestBed.inject(AgentService) as jasmine.SpyObj<AgentService>;

    // Default spy returns
    dataLoaderSpy.getAttestationsForContent.and.returnValue(of(mockAttestations));
    contentServiceSpy.getContent.and.returnValue(of(mockContent));
    agentServiceSpy.getCurrentAgentId.and.returnValue('user-123');
    agentServiceSpy.getAttestations.and.returnValue(['community-member']);

    service = TestBed.inject(TrustBadgeService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  // =========================================================================
  // getBadge
  // =========================================================================

  describe('getBadge', () => {
    it('should return trust badge for content', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        expect(badge.contentId).toBe('content-1');
        expect(badge.trustLevel).toBeDefined();
        expect(badge.primary).toBeDefined();
        done();
      });
    });

    it('should set primary badge to highest priority attestation', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        // peer-reviewed is higher priority than community-endorsed
        expect(badge.primary.attestationType).toBe('peer-reviewed');
        done();
      });
    });

    it('should include secondary badges', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        expect(badge.secondary.length).toBeGreaterThan(0);
        // Should include reach and community-endorsed
        const hasReach = badge.secondary.some(b => b.type === 'reach');
        const hasCommunity = badge.secondary.some(b => b.attestationType === 'community-endorsed');
        expect(hasReach || hasCommunity).toBe(true);
        done();
      });
    });

    it('should calculate trust percentage', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        expect(badge.trustPercentage).toBeGreaterThan(0);
        expect(badge.trustPercentage).toBeLessThanOrEqual(100);
        done();
      });
    });

    it('should include reach information', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        expect(badge.reach).toBe('regional');
        expect(badge.reachLabel).toBeDefined();
        done();
      });
    });

    it('should generate summary text', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        expect(badge.summary).toBeDefined();
        expect(badge.summary.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should generate aria label', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        expect(badge.ariaLabel).toBeDefined();
        expect(badge.ariaLabel.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should include available actions', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        expect(badge.actions?.length).toBeGreaterThan(0);
        // View trust profile should always be available
        const viewAction = badge.actions?.find(a => a.action === 'view-trust-profile');
        expect(viewAction).toBeDefined();
        expect(viewAction?.available).toBe(true);
        done();
      });
    });

    it('should handle content with no attestations', (done) => {
      dataLoaderSpy.getAttestationsForContent.and.returnValue(of([]));

      service.getBadge('content-1').subscribe(badge => {
        expect(badge.primary.type).toBe('reach');
        expect(badge.trustLevel).toBe('unverified');
        done();
      });
    });

    it('should handle errors gracefully', (done) => {
      contentServiceSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getBadge('missing').subscribe(badge => {
        expect(badge.contentId).toBe('missing');
        expect(badge.trustLevel).toBe('unverified');
        done();
      });
    });
  });

  // =========================================================================
  // getCompactBadge
  // =========================================================================

  describe('getCompactBadge', () => {
    it('should return compact badge', (done) => {
      service.getCompactBadge('content-1').subscribe(badge => {
        expect(badge.contentId).toBe('content-1');
        expect(badge.icon).toBeDefined();
        expect(badge.color).toBeDefined();
        expect(badge.tooltip).toBeDefined();
        done();
      });
    });

    it('should include trust level', (done) => {
      service.getCompactBadge('content-1').subscribe(badge => {
        expect(badge.trustLevel).toBeDefined();
        done();
      });
    });
  });

  // =========================================================================
  // getBadgesForContent (bulk)
  // =========================================================================

  describe('getBadgesForContent', () => {
    it('should return map of badges', (done) => {
      service.getBadgesForContent(['content-1']).subscribe(badges => {
        expect(badges.size).toBe(1);
        expect(badges.get('content-1')).toBeDefined();
        done();
      });
    });

    it('should handle empty array', (done) => {
      service.getBadgesForContent([]).subscribe(badges => {
        expect(badges.size).toBe(0);
        done();
      });
    });

    it('should load multiple badges', (done) => {
      service.getBadgesForContent(['content-1', 'content-2']).subscribe(badges => {
        expect(badges.size).toBe(2);
        done();
      });
    });
  });

  // =========================================================================
  // getCompactBadgesForContent
  // =========================================================================

  describe('getCompactBadgesForContent', () => {
    it('should return map of compact badges', (done) => {
      service.getCompactBadgesForContent(['content-1']).subscribe(badges => {
        expect(badges.size).toBe(1);
        const badge = badges.get('content-1');
        expect(badge?.icon).toBeDefined();
        done();
      });
    });
  });

  // =========================================================================
  // Warnings/Flags
  // =========================================================================

  describe('warnings', () => {
    it('should detect when content has flags', (done) => {
      contentServiceSpy.getContent.and.returnValue(of(mockContentWithFlags));

      service.getBadge('flagged-content').subscribe(badge => {
        expect(badge.hasWarnings).toBe(true);
        expect(badge.warnings.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should not have warnings for clean content', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        expect(badge.hasWarnings).toBe(false);
        expect(badge.warnings.length).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Trust Indicators
  // =========================================================================

  describe('getIndicators', () => {
    it('should return indicator set', (done) => {
      service.getIndicators('content-1').subscribe(indicators => {
        expect(indicators.contentId).toBe('content-1');
        expect(indicators.indicators.length).toBeGreaterThan(0);
        expect(indicators.primary).toBeDefined();
        done();
      });
    });

    it('should separate badges and flags', (done) => {
      contentServiceSpy.getContent.and.returnValue(of(mockContentWithFlags));

      service.getIndicators('flagged-content').subscribe(indicators => {
        expect(indicators.badges.length).toBeGreaterThan(0);
        expect(indicators.flags.length).toBeGreaterThan(0);
        done();
      });
    });

    it('should prioritize flags as primary when present', (done) => {
      contentServiceSpy.getContent.and.returnValue(of(mockContentWithFlags));

      service.getIndicators('flagged-content').subscribe(indicators => {
        expect(indicators.primary?.polarity).toBe('negative');
        done();
      });
    });

    it('should include trust percentage', (done) => {
      service.getIndicators('content-1').subscribe(indicators => {
        expect(indicators.trustPercentage).toBeDefined();
        expect(indicators.trustPercentage).toBeGreaterThanOrEqual(0);
        done();
      });
    });

    it('should handle errors', (done) => {
      contentServiceSpy.getContent.and.returnValue(throwError(() => new Error('Not found')));

      service.getIndicators('missing').subscribe(indicators => {
        expect(indicators.contentId).toBe('missing');
        expect(indicators.indicators.length).toBe(0);
        expect(indicators.trustLevel).toBe('unverified');
        done();
      });
    });
  });

  describe('getIndicatorsForContent', () => {
    it('should return map of indicator sets', (done) => {
      service.getIndicatorsForContent(['content-1']).subscribe(indicators => {
        expect(indicators.size).toBe(1);
        expect(indicators.get('content-1')?.trustLevel).toBeDefined();
        done();
      });
    });

    it('should handle empty array', (done) => {
      service.getIndicatorsForContent([]).subscribe(indicators => {
        expect(indicators.size).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Reach Helpers
  // =========================================================================

  describe('meetsReachRequirement', () => {
    it('should return true when reach meets requirement', () => {
      expect(service.meetsReachRequirement('commons', 'regional')).toBe(true);
      expect(service.meetsReachRequirement('regional', 'local')).toBe(true);
    });

    it('should return false when reach does not meet requirement', () => {
      expect(service.meetsReachRequirement('local', 'regional')).toBe(false);
      expect(service.meetsReachRequirement('private', 'commons')).toBe(false);
    });

    it('should return true when exact match', () => {
      expect(service.meetsReachRequirement('regional', 'regional')).toBe(true);
    });
  });

  describe('getNextReachLevel', () => {
    it('should return next level', () => {
      expect(service.getNextReachLevel('local')).toBe('neighborhood');
      expect(service.getNextReachLevel('regional')).toBe('commons');
    });

    it('should return null for commons', () => {
      expect(service.getNextReachLevel('commons')).toBeNull();
    });
  });

  describe('getAttestationsNeededForNextLevel', () => {
    it('should return needed attestations', () => {
      const needed = service.getAttestationsNeededForNextLevel('local', ['author-verified']);
      expect(needed.length).toBeGreaterThan(0);
    });

    it('should exclude already-held attestations', () => {
      const needed = service.getAttestationsNeededForNextLevel('local', ['author-verified', 'community-endorsed']);
      expect(needed).not.toContain('author-verified');
    });

    it('should return empty for commons', () => {
      const needed = service.getAttestationsNeededForNextLevel('commons', []);
      expect(needed.length).toBe(0);
    });
  });

  // =========================================================================
  // Trust Score Calculation
  // =========================================================================

  describe('trust score calculation', () => {
    it('should calculate higher score for more attestations', (done) => {
      const manyAttestations: ContentAttestation[] = [
        ...mockAttestations,
        {
          id: 'att-3',
          contentId: 'content-1',
          attestationType: 'steward-approved',
          reachGranted: 'municipal',
          grantedBy: { type: 'steward', grantorId: 'steward-1', grantorName: 'Steward' },
          grantedAt: '2025-01-03T00:00:00.000Z',
          status: 'active',
          metadata: {}
        }
      ];
      dataLoaderSpy.getAttestationsForContent.and.returnValue(of(manyAttestations));

      service.getBadge('content-1').subscribe(badge => {
        expect(badge.trustPercentage).toBeGreaterThan(50);
        done();
      });
    });

    it('should give zero for content without attestations', (done) => {
      dataLoaderSpy.getAttestationsForContent.and.returnValue(of([]));
      contentServiceSpy.getContent.and.returnValue(of({ ...mockContent, trustScore: undefined }));

      service.getBadge('content-1').subscribe(badge => {
        expect(badge.trustPercentage).toBe(0);
        done();
      });
    });
  });

  // =========================================================================
  // Actions Based on Agent Permissions
  // =========================================================================

  describe('available actions', () => {
    it('should include endorse action for community members', (done) => {
      agentServiceSpy.getAttestations.and.returnValue(['community-member']);

      service.getBadge('content-1').subscribe(badge => {
        const endorseAction = badge.actions?.find(a => a.action === 'endorse');
        expect(endorseAction?.available).toBe(true);
        done();
      });
    });

    it('should disable endorse for non-community-members', (done) => {
      agentServiceSpy.getAttestations.and.returnValue([]);

      service.getBadge('content-1').subscribe(badge => {
        const endorseAction = badge.actions?.find(a => a.action === 'endorse');
        expect(endorseAction?.available).toBe(false);
        done();
      });
    });

    it('should include report action for authenticated users', (done) => {
      service.getBadge('content-1').subscribe(badge => {
        const reportAction = badge.actions?.find(a => a.action === 'report');
        expect(reportAction).toBeDefined();
        expect(reportAction?.available).toBe(true);
        done();
      });
    });
  });
});
