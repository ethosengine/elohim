/**
 * Shefa-compute Service Tests
 */

import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';

import { ShefaComputeService } from './shefa-compute.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { EconomicService } from './economic.service';
import { StewardedResourceService } from './stewarded-resources.service';
import type { ComputeGap } from '../models/shefa-dashboard.model';
import { REAAction } from '@app/elohim/models/rea-bridge.model';

describe('ShefaComputeService', () => {
  let service: ShefaComputeService;
  let mockHolochain: jasmine.SpyObj<HolochainClientService>;
  let mockEconomic: jasmine.SpyObj<EconomicService>;
  let mockStewardedResources: jasmine.SpyObj<StewardedResourceService>;

  beforeEach(() => {
    mockHolochain = jasmine.createSpyObj('HolochainClientService', ['callZome']);
    mockEconomic = jasmine.createSpyObj('EconomicService', [
      'getEventsForAgent',
      'getEventsByLamadType',
    ]);
    mockStewardedResources = jasmine.createSpyObj('StewardedResourceService', [
      'getResourceById',
    ]);

    TestBed.configureTestingModule({
      providers: [
        ShefaComputeService,
        { provide: HolochainClientService, useValue: mockHolochain },
        { provide: EconomicService, useValue: mockEconomic },
        { provide: StewardedResourceService, useValue: mockStewardedResources },
      ],
    });
    service = TestBed.inject(ShefaComputeService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have service instance', () => {
    expect(service).toBeDefined();
  });

  // ==========================================================================
  // Helper Method Tests - Extracted from nested ternaries
  // ==========================================================================

  describe('determineRedundancyStrategy', () => {
    it('should return erasure_coded for avgRedundancy >= 3', () => {
      const result = (service as any).determineRedundancyStrategy(3.5);
      expect(result).toBe('erasure_coded');
    });

    it('should return threshold_split for avgRedundancy >= 2 but < 3', () => {
      const result = (service as any).determineRedundancyStrategy(2.5);
      expect(result).toBe('threshold_split');
    });

    it('should return full_replica for avgRedundancy < 2', () => {
      const result = (service as any).determineRedundancyStrategy(1.5);
      expect(result).toBe('full_replica');
    });

    it('should handle boundary value 3 as erasure_coded', () => {
      const result = (service as any).determineRedundancyStrategy(3);
      expect(result).toBe('erasure_coded');
    });

    it('should handle boundary value 2 as threshold_split', () => {
      const result = (service as any).determineRedundancyStrategy(2);
      expect(result).toBe('threshold_split');
    });

    it('should handle zero redundancy', () => {
      const result = (service as any).determineRedundancyStrategy(0);
      expect(result).toBe('full_replica');
    });
  });

  describe('determineRiskProfile', () => {
    it('should return geo-redundant for 3+ regions', () => {
      const result = (service as any).determineRiskProfile(5);
      expect(result).toBe('geo-redundant');
    });

    it('should return distributed for 2 regions', () => {
      const result = (service as any).determineRiskProfile(2);
      expect(result).toBe('distributed');
    });

    it('should return centralized for 1 region', () => {
      const result = (service as any).determineRiskProfile(1);
      expect(result).toBe('centralized');
    });

    it('should handle boundary value 3 as geo-redundant', () => {
      const result = (service as any).determineRiskProfile(3);
      expect(result).toBe('geo-redundant');
    });

    it('should handle zero regions as centralized', () => {
      const result = (service as any).determineRiskProfile(0);
      expect(result).toBe('centralized');
    });
  });

  describe('determineProtectionLevel', () => {
    it('should return highly-protected for 3+ custodians with high redundancy', () => {
      const result = (service as any).determineProtectionLevel(3, 2.5);
      expect(result).toBe('highly-protected');
    });

    it('should return protected for 2+ custodians regardless of redundancy', () => {
      const result = (service as any).determineProtectionLevel(2, 1.0);
      expect(result).toBe('protected');
    });

    it('should return vulnerable for fewer than 2 custodians', () => {
      const result = (service as any).determineProtectionLevel(1, 3.0);
      expect(result).toBe('vulnerable');
    });

    it('should require both conditions for highly-protected', () => {
      // High redundancy but not enough custodians
      const result1 = (service as any).determineProtectionLevel(2, 3.0);
      expect(result1).toBe('protected');

      // Enough custodians but low redundancy
      const result2 = (service as any).determineProtectionLevel(4, 2.0);
      expect(result2).toBe('protected');
    });

    it('should handle boundary values', () => {
      // Exactly at threshold
      const result = (service as any).determineProtectionLevel(3, 2.5);
      expect(result).toBe('highly-protected');
    });
  });

  describe('estimateRecoveryTime', () => {
    it('should return < 1 hour for 3+ custodians', () => {
      const result = (service as any).estimateRecoveryTime(5);
      expect(result).toBe('< 1 hour');
    });

    it('should return < 4 hours for 2 custodians', () => {
      const result = (service as any).estimateRecoveryTime(2);
      expect(result).toBe('< 4 hours');
    });

    it('should return > 24 hours for fewer than 2 custodians', () => {
      const result = (service as any).estimateRecoveryTime(1);
      expect(result).toBe('> 24 hours');
    });

    it('should handle boundary value 3', () => {
      const result = (service as any).estimateRecoveryTime(3);
      expect(result).toBe('< 1 hour');
    });

    it('should handle zero custodians', () => {
      const result = (service as any).estimateRecoveryTime(0);
      expect(result).toBe('> 24 hours');
    });
  });

  describe('mapEventTypeToTransactionType', () => {
    it('should map infrastructure-token-issued to earned', () => {
      const result = (service as any).mapEventTypeToTransactionType('infrastructure-token-issued');
      expect(result).toBe('earned');
    });

    it('should map token-transferred to transferred', () => {
      const result = (service as any).mapEventTypeToTransactionType('token-transferred');
      expect(result).toBe('transferred');
    });

    it('should map token-decayed to decayed', () => {
      const result = (service as any).mapEventTypeToTransactionType('token-decayed');
      expect(result).toBe('decayed');
    });

    it('should default to earned for unknown types', () => {
      const result = (service as any).mapEventTypeToTransactionType('unknown-type');
      expect(result).toBe('earned');
    });

    it('should handle empty string', () => {
      const result = (service as any).mapEventTypeToTransactionType('');
      expect(result).toBe('earned');
    });
  });

  describe('determineGapSeverity', () => {
    it('should return critical for gap > 50%', () => {
      const result = (service as any).determineGapSeverity(75);
      expect(result).toBe('critical');
    });

    it('should return moderate for gap > 25% but <= 50%', () => {
      const result = (service as any).determineGapSeverity(40);
      expect(result).toBe('moderate');
    });

    it('should return minor for gap <= 25%', () => {
      const result = (service as any).determineGapSeverity(15);
      expect(result).toBe('minor');
    });

    it('should handle boundary values', () => {
      expect((service as any).determineGapSeverity(50)).toBe('moderate');
      expect((service as any).determineGapSeverity(51)).toBe('critical');
      expect((service as any).determineGapSeverity(25)).toBe('minor');
      expect((service as any).determineGapSeverity(26)).toBe('moderate');
    });

    it('should handle zero gap', () => {
      const result = (service as any).determineGapSeverity(0);
      expect(result).toBe('minor');
    });

    it('should handle 100% gap', () => {
      const result = (service as any).determineGapSeverity(100);
      expect(result).toBe('critical');
    });
  });

  describe('determineStorageGapSeverity', () => {
    it('should return critical for gap > 80%', () => {
      const result = (service as any).determineStorageGapSeverity(90);
      expect(result).toBe('critical');
    });

    it('should return moderate for gap > 50% but <= 80%', () => {
      const result = (service as any).determineStorageGapSeverity(65);
      expect(result).toBe('moderate');
    });

    it('should return minor for gap <= 50%', () => {
      const result = (service as any).determineStorageGapSeverity(30);
      expect(result).toBe('minor');
    });

    it('should handle boundary values', () => {
      expect((service as any).determineStorageGapSeverity(80)).toBe('moderate');
      expect((service as any).determineStorageGapSeverity(81)).toBe('critical');
      expect((service as any).determineStorageGapSeverity(50)).toBe('minor');
      expect((service as any).determineStorageGapSeverity(51)).toBe('moderate');
    });
  });

  describe('determineAlertSeverity', () => {
    it('should return critical for primary node regardless of status', () => {
      const result = (service as any).determineAlertSeverity(true, 'offline');
      expect(result).toBe('critical');
    });

    it('should return warning for non-primary offline node', () => {
      const result = (service as any).determineAlertSeverity(false, 'offline');
      expect(result).toBe('warning');
    });

    it('should return info for non-primary degraded node', () => {
      const result = (service as any).determineAlertSeverity(false, 'degraded');
      expect(result).toBe('info');
    });

    it('should prioritize primary status over node status', () => {
      const result = (service as any).determineAlertSeverity(true, 'degraded');
      expect(result).toBe('critical');
    });
  });

  describe('determineCeilingStatus', () => {
    it('should return warning for high CPU usage', () => {
      const result = (service as any).determineCeilingStatus(95, 50000, 100000);
      expect(result).toBe('warning');
    });

    it('should return warning for tokens > 80% of ceiling', () => {
      const result = (service as any).determineCeilingStatus(50, 85000, 100000);
      expect(result).toBe('warning');
    });

    it('should return warning for tokens > ceiling', () => {
      const result = (service as any).determineCeilingStatus(50, 110000, 100000);
      expect(result).toBe('warning');
    });

    it('should return safe for normal conditions', () => {
      const result = (service as any).determineCeilingStatus(50, 50000, 100000);
      expect(result).toBe('safe');
    });

    it('should handle boundary conditions', () => {
      // Just under 80%
      const result1 = (service as any).determineCeilingStatus(50, 79999, 100000);
      expect(result1).toBe('safe');

      // Exactly at 80% - implementation uses > not >=, so still safe
      const result2 = (service as any).determineCeilingStatus(50, 80000, 100000);
      expect(result2).toBe('safe');

      // Just over 80%
      const result3 = (service as any).determineCeilingStatus(50, 80001, 100000);
      expect(result3).toBe('warning');
    });
  });

  describe('calculateMutualAidRatio', () => {
    it('should calculate ratio when both values are positive', () => {
      const result = (service as any).calculateMutualAidRatio(100, 50);
      expect(result).toBe(2);
    });

    it('should return 2 when helping but not being helped', () => {
      const result = (service as any).calculateMutualAidRatio(100, 0);
      expect(result).toBe(2);
    });

    it('should return 1 when neither helping nor being helped', () => {
      const result = (service as any).calculateMutualAidRatio(0, 0);
      expect(result).toBe(1);
    });

    it('should handle balanced mutual aid', () => {
      const result = (service as any).calculateMutualAidRatio(75, 75);
      expect(result).toBe(1);
    });

    it('should handle receiving more than giving', () => {
      const result = (service as any).calculateMutualAidRatio(25, 100);
      expect(result).toBe(0.25);
    });
  });

  describe('determineMutualAidStatus', () => {
    it('should return giving-more for ratio > 1.2', () => {
      const result = (service as any).determineMutualAidStatus(1.5);
      expect(result).toBe('giving-more');
    });

    it('should return receiving-more for ratio < 0.8', () => {
      const result = (service as any).determineMutualAidStatus(0.5);
      expect(result).toBe('receiving-more');
    });

    it('should return balanced for ratio between 0.8 and 1.2', () => {
      const result = (service as any).determineMutualAidStatus(1.0);
      expect(result).toBe('balanced');
    });

    it('should handle boundary values', () => {
      expect((service as any).determineMutualAidStatus(1.2)).toBe('balanced');
      expect((service as any).determineMutualAidStatus(1.21)).toBe('giving-more');
      expect((service as any).determineMutualAidStatus(0.8)).toBe('balanced');
      expect((service as any).determineMutualAidStatus(0.79)).toBe('receiving-more');
    });
  });

  describe('determineOverallGapSeverity', () => {
    it('should return critical if any gap is critical', () => {
      const gaps: ComputeGap[] = [
        { resource: 'cpu', severity: 'minor' } as ComputeGap,
        { resource: 'storage', severity: 'critical' } as ComputeGap,
      ];
      const result = (service as any).determineOverallGapSeverity(gaps, true);
      expect(result).toBe('critical');
    });

    it('should return moderate if any gap is moderate but none critical', () => {
      const gaps: ComputeGap[] = [
        { resource: 'cpu', severity: 'minor' } as ComputeGap,
        { resource: 'storage', severity: 'moderate' } as ComputeGap,
      ];
      const result = (service as any).determineOverallGapSeverity(gaps, true);
      expect(result).toBe('moderate');
    });

    it('should return minor if hasGaps but all are minor', () => {
      const gaps: ComputeGap[] = [
        { resource: 'cpu', severity: 'minor' } as ComputeGap,
        { resource: 'storage', severity: 'minor' } as ComputeGap,
      ];
      const result = (service as any).determineOverallGapSeverity(gaps, true);
      expect(result).toBe('minor');
    });

    it('should return none if no gaps', () => {
      const gaps: ComputeGap[] = [];
      const result = (service as any).determineOverallGapSeverity(gaps, false);
      expect(result).toBe('none');
    });

    it('should prioritize critical over all other severities', () => {
      const gaps: ComputeGap[] = [
        { resource: 'cpu', severity: 'minor' } as ComputeGap,
        { resource: 'memory', severity: 'moderate' } as ComputeGap,
        { resource: 'storage', severity: 'critical' } as ComputeGap,
      ];
      const result = (service as any).determineOverallGapSeverity(gaps, true);
      expect(result).toBe('critical');
    });
  });

  describe('generateHelpFlowCTA', () => {
    it('should return healthy message when no gaps', () => {
      const result = (service as any).generateHelpFlowCTA(false, 'none');
      expect(result).toBe('Your compute is healthy');
    });

    it('should return urgent message for critical gaps', () => {
      const result = (service as any).generateHelpFlowCTA(true, 'critical');
      expect(result).toBe('Restore protection now');
    });

    it('should return improvement message for non-critical gaps', () => {
      const result = (service as any).generateHelpFlowCTA(true, 'moderate');
      expect(result).toBe('Improve your compute capacity');
    });

    it('should return improvement message for minor gaps', () => {
      const result = (service as any).generateHelpFlowCTA(true, 'minor');
      expect(result).toBe('Improve your compute capacity');
    });
  });

  describe('mapCustodianTypeToRelationship', () => {
    it('should map family to family-member', () => {
      const result = (service as any).mapCustodianTypeToRelationship('family');
      expect(result).toBe('family-member');
    });

    it('should map friend to friend', () => {
      const result = (service as any).mapCustodianTypeToRelationship('friend');
      expect(result).toBe('friend');
    });

    it('should map professional to professional', () => {
      const result = (service as any).mapCustodianTypeToRelationship('professional');
      expect(result).toBe('professional');
    });

    it('should map institution to institution', () => {
      const result = (service as any).mapCustodianTypeToRelationship('institution');
      expect(result).toBe('institution');
    });

    it('should map community to community-peer', () => {
      const result = (service as any).mapCustodianTypeToRelationship('community');
      expect(result).toBe('community-peer');
    });

    it('should default to community-peer for unknown types', () => {
      const result = (service as any).mapCustodianTypeToRelationship('unknown' as any);
      expect(result).toBe('community-peer');
    });
  });

  // ==========================================================================
  // Integration Tests - Complex Observable Flows
  // ==========================================================================

  describe('getInfrastructureTokenBalance', () => {
    it('should aggregate token events correctly', done => {
      const mockEvents = [
        {
          id: 'evt1',
          action: 'produce' as REAAction,
          state: 'validated' as 'validated',
          hasPointInTime: '2024-01-01T00:00:00Z',
          provider: 'operator1',
          receiver: 'operator1',
          resourceQuantity: { hasNumericalValue: 100, hasUnit: 'infrastructure-token' },
          metadata: { type: 'infrastructure-token-issued' },
          note: 'Earned tokens',
        },
        {
          id: 'evt2',
          action: 'transfer' as REAAction,
          state: 'validated' as 'validated',
          hasPointInTime: '2024-01-02T00:00:00Z',
          provider: 'operator1',
          receiver: 'other',
          resourceQuantity: { hasNumericalValue: 20, hasUnit: 'infrastructure-token' },
          metadata: { type: 'token-transferred' },
          note: 'Transferred tokens',
        },
      ];

      mockEconomic.getEventsForAgent.and.returnValue(of(mockEvents));

      (service as any).getInfrastructureTokenBalance('operator1').subscribe((result: any) => {
        expect(result.balance.tokens).toBe(80); // 100 - 20
        expect(result.transactions.length).toBe(2);
        expect(result.transactions[0].type).toBe('transferred'); // Most recent first
        expect(result.transactions[1].type).toBe('earned');
        done();
      });
    });

    it('should handle empty events', done => {
      mockEconomic.getEventsForAgent.and.returnValue(of([]));

      (service as any).getInfrastructureTokenBalance('operator1').subscribe((result: any) => {
        expect(result.balance.tokens).toBe(0);
        expect(result.transactions.length).toBe(0);
        done();
      });
    });

    it('should filter only token-related events', done => {
      const mockEvents = [
        {
          id: 'evt1',
          action: 'produce' as REAAction,
          state: 'validated' as 'validated',
          hasPointInTime: '2024-01-01T00:00:00Z',
          provider: 'operator1',
          receiver: 'operator1',
          resourceQuantity: { hasNumericalValue: 100, hasUnit: 'infrastructure-token' },
          metadata: { type: 'infrastructure-token-issued' },
          note: '',
        },
        {
          id: 'evt2',
          action: 'produce' as REAAction,
          state: 'validated' as 'validated',
          hasPointInTime: '2024-01-02T00:00:00Z',
          provider: 'operator1',
          receiver: 'operator1',
          resourceQuantity: { hasNumericalValue: 50, hasUnit: 'infrastructure-token' },
          metadata: { type: 'some-other-event' },
          note: '',
        },
      ];

      mockEconomic.getEventsForAgent.and.returnValue(of(mockEvents));

      (service as any).getInfrastructureTokenBalance('operator1').subscribe((result: any) => {
        expect(result.balance.tokens).toBe(100); // Only infrastructure-token event counted
        expect(result.transactions.length).toBe(1);
        done();
      });
    });
  });
});
