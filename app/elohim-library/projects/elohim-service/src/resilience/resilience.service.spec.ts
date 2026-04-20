import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { firstValueFrom } from 'rxjs';
import { ResilienceService } from './resilience.service';
import { ResilienceSnapshotView } from '../generated/resilience-snapshot-view';

describe('ResilienceService', () => {
  let service: ResilienceService;
  let http: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [ResilienceService],
    });
    service = TestBed.inject(ResilienceService);
    http = TestBed.inject(HttpTestingController);
  });

  afterEach(() => http.verify());

  it('fetches snapshot for contentId', async () => {
    const mock: ResilienceSnapshotView = {
      contentId: 'c1',
      stewardingCollectives: 3,
      commitmentBackedCollectives: 4,
      diversityScore: 0.75,
      regionalDistribution: { local: 1, regional: 1, global: 1, unknown: 0 },
      placementGaps: [],
      protectionStatus: 'protected',
    } as ResilienceSnapshotView;

    const promise = firstValueFrom(service.getSnapshot('c1'));
    const req = http.expectOne('/api/v1/resilience/c1/household');
    expect(req.request.method).toBe('GET');
    req.flush(mock);

    const view = await promise;
    expect(view.contentId).toBe('c1');
    expect(view.diversityScore).toBe(0.75);
  });

  it('passes viewerHouseholdId as query param', async () => {
    const promise = firstValueFrom(service.getSnapshot('c2', 'hh-alpha'));
    const req = http.expectOne((r) => r.url === '/api/v1/resilience/c2/household');
    expect(req.request.params.get('viewerHouseholdId')).toBe('hh-alpha');
    req.flush({});
    await promise;
  });
});
