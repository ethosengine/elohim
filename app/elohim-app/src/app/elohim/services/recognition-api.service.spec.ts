import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { RecognitionApiService } from './recognition-api.service';

describe('RecognitionApiService', () => {
  let service: RecognitionApiService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [provideHttpClient(), provideHttpClientTesting(), RecognitionApiService],
    });
    service = TestBed.inject(RecognitionApiService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => httpMock.verify());

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should POST to /api/v1/recognition/distribute', () => {
    const trigger = { contentId: 'c-1', eventType: 'mastery_completion', rawAmount: 10 };
    const mockResult = {
      contentId: 'c-1',
      triggerEventType: 'mastery_completion',
      rawAmount: 10,
      weightedAmount: 10,
      distributions: [],
      economicEventIds: [],
      limitsApplied: [],
    };

    service.distribute(trigger).subscribe(result => {
      expect(result.contentId).toBe('c-1');
      expect(result.weightedAmount).toBe(10);
    });

    const req = httpMock.expectOne('/api/v1/recognition/distribute');
    expect(req.request.method).toBe('POST');
    expect(req.request.body).toEqual(trigger);
    req.flush(mockResult);
  });
});
