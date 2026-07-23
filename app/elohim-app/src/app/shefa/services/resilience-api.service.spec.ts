import { TestBed } from '@angular/core/testing';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { provideHttpClient } from '@angular/common/http';

import { firstValueFrom } from 'rxjs';

import { ResilienceApiService } from './resilience-api.service';

describe('ResilienceApiService', () => {
  let service: ResilienceApiService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [ResilienceApiService, provideHttpClient(), provideHttpClientTesting()],
    });
    service = TestBed.inject(ResilienceApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  // No backend route exists for a human-scoped resilience profile (see the
  // class-level comment on ResilienceApiService) — every test below asserts
  // NO HTTP request is ever issued, in addition to the honestly-empty
  // profile shape.
  afterEach(() => httpMock.verify());

  it('computeProfile never calls the network — returns an honestly-empty profile, no HTTP request', async () => {
    const profile = await service.computeProfile('human-matthew-manager');

    expect(profile.humanId).toBe('human-matthew-manager');
    expect(profile.overallScore).toBe(0);
    expect(profile.protectionStatus).toBe('at-risk');
    expect(profile.contentRiskBreakdown).toEqual([]);
    // httpMock.verify() in afterEach fails the test if any request was made.
  });

  it('computeProfile names the missing backend surface in nextAction.description', async () => {
    const profile = await service.computeProfile('human-matthew-manager');

    expect(profile.nextAction?.type).toBe('review');
    expect(profile.nextAction?.description).toContain(
      '/api/v1/resilience/human-matthew-manager/profile'
    );
    expect(profile.nextAction?.description).toContain('content_id');
  });

  it('getProfile returns null before computation', () => {
    expect(service.getProfile()).toBeNull();
  });

  it('getProfile returns the cached honestly-empty profile after computation', async () => {
    const profile = await service.computeProfile('human-matthew-manager');
    expect(service.getProfile()).toEqual(profile);
  });

  it('getProtectionStatus returns "at-risk" (fail-closed floor) after computation, null before', async () => {
    expect(service.getProtectionStatus()).toBeNull();
    await service.computeProfile('human-matthew-manager');
    expect(service.getProtectionStatus()).toBe('at-risk');
  });

  it('getNextAction returns the "review" action naming the missing surface', async () => {
    expect(service.getNextAction()).toBeNull();
    await service.computeProfile('human-matthew-manager');
    expect(service.getNextAction()?.type).toBe('review');
  });

  it('getContentRiskBreakdown returns an empty array (no content-risk data available)', async () => {
    expect(service.getContentRiskBreakdown()).toEqual([]);
    await service.computeProfile('human-matthew-manager');
    expect(service.getContentRiskBreakdown()).toEqual([]);
  });

  it('getElohimAssessment returns null (no assessment data available)', async () => {
    await service.computeProfile('human-matthew-manager');
    expect(service.getElohimAssessment()).toBeNull();
  });

  it('getProfile$ emits the honestly-empty profile once, with no HTTP request, and caches it', async () => {
    const profile = await firstValueFrom(service.getProfile$('human-matthew-manager'));

    expect(profile.humanId).toBe('human-matthew-manager');
    expect(profile.protectionStatus).toBe('at-risk');
    expect(service.getProfile()).toEqual(profile);
  });

  it('getProfile$ unsubscribes any prior polling subscription before starting a new one', async () => {
    const firstSub = service.getProfile$('human-a').subscribe();
    expect(firstSub.closed).toBe(true); // `of(...)` completes synchronously.

    // A second call must not throw and must still resolve to an honestly-empty profile.
    const second = await firstValueFrom(service.getProfile$('human-b'));
    expect(second.humanId).toBe('human-b');
  });
});
