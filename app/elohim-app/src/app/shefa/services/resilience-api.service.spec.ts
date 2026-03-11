import { TestBed } from '@angular/core/testing';
import {
  HttpTestingController,
  provideHttpClientTesting,
} from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { ResilienceApiService } from './resilience-api.service';
import type { ResilienceProfile } from '../models/resilience-profile.model';

describe('ResilienceApiService', () => {
  let service: ResilienceApiService;
  let httpMock: HttpTestingController;

  const stubProfile: ResilienceProfile = {
    humanId: 'human-matthew-manager',
    overallScore: 0.65,
    protectionStatus: 'partial',
    shardHealth: {
      totalBlobs: 42,
      totalShards: 120,
      distinctPeers: 3,
      averageShardsPerBlob: 2.86,
      encodingBreakdown: { single: 10, chunked: 12, reedSolomon: 20 },
      singlePointOfFailureCount: 10,
      lastAccessVerifiedAt: '2026-03-11T10:00:00Z',
    },
    commitmentHealth: {
      activeCommitments: 2,
      reciprocatedCommitments: 1,
      expiringSoon: 0,
      totalPeersCommitted: 2,
      commitmentCoverage: 0.6,
    },
    trustCircleDepth: {
      householdPeers: 1,
      friendPeers: 0,
      communityPeers: 1,
      institutionalPeers: 0,
      totalCircles: 2,
    },
    contentRiskBreakdown: [
      {
        reach: 'private',
        contentCount: 5,
        shardDistribution: 1,
        adequacy: 0.4,
        exemplar: 'medical records',
      },
      {
        reach: 'neighborhood',
        contentCount: 30,
        shardDistribution: 3,
        adequacy: 0.8,
        exemplar: 'faith community content',
      },
    ],
    nextAction: {
      type: 'connect',
      description:
        'Connect with a friend or community peer to diversify personal-reach backup',
      urgency: 'soon',
    },
    lastComputedAt: '2026-03-11T10:00:00Z',
  };

  /** Fetch and flush in one call, returning the result. */
  async function fetchAndFlush(
    humanId = 'human-matthew-manager',
    data: ResilienceProfile = stubProfile
  ): Promise<ResilienceProfile> {
    const promise = service.computeProfile(humanId);
    httpMock.expectOne(`/api/v1/resilience/${humanId}/profile`).flush(data);
    return promise;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [
        ResilienceApiService,
        provideHttpClient(),
        provideHttpClientTesting(),
      ],
    });
    service = TestBed.inject(ResilienceApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('computeProfile calls GET /api/v1/resilience/{humanId}/profile', async () => {
    const promise = service.computeProfile('human-matthew-manager');
    const req = httpMock.expectOne(
      '/api/v1/resilience/human-matthew-manager/profile'
    );
    expect(req.request.method).toBe('GET');
    req.flush(stubProfile);
    expect(await promise).toEqual(stubProfile);
  });

  it('getProfile returns null before computation', () => {
    expect(service.getProfile()).toBeNull();
  });

  it('getProfile returns cached profile after computation', async () => {
    await fetchAndFlush();
    expect(service.getProfile()).toEqual(stubProfile);
  });

  it('getProtectionStatus returns status from cached profile', async () => {
    expect(service.getProtectionStatus()).toBeNull();
    await fetchAndFlush();
    expect(service.getProtectionStatus()).toBe('partial');
  });

  it('getNextAction returns action from cached profile', async () => {
    expect(service.getNextAction()).toBeNull();
    await fetchAndFlush();
    expect(service.getNextAction()).toEqual(stubProfile.nextAction!);
  });

  it('getContentRiskBreakdown returns buckets from cached profile', async () => {
    expect(service.getContentRiskBreakdown()).toEqual([]);
    await fetchAndFlush();
    expect(service.getContentRiskBreakdown()).toEqual(
      stubProfile.contentRiskBreakdown
    );
  });

  it('getElohimAssessment returns null when no assessment', async () => {
    await fetchAndFlush();
    expect(service.getElohimAssessment()).toBeNull();
  });

  it('getElohimAssessment returns assessment when present', async () => {
    const assessment = {
      assessedAt: '2026-03-11T10:00:00Z',
      assessedBy: { agentId: 'elohim-1' },
      overallAdequacy: 0.7,
      narrative: 'Your data protection is improving but personal records need attention.',
      memories: [],
      concerns: [
        {
          severity: 'concerning' as const,
          description: 'Medical records have single-point-of-failure risk',
        },
      ],
      attestations: ['constitutional-data-sovereignty-1'],
    };
    const profileWithAssessment: ResilienceProfile = {
      ...stubProfile,
      elohimAssessment: assessment,
    };
    await fetchAndFlush('human-matthew-manager', profileWithAssessment);
    expect(service.getElohimAssessment()).toEqual(assessment);
  });
});
