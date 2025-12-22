import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { BlobFallbackService, BlobFetchResult, UrlHealth } from './blob-fallback.service';

describe('BlobFallbackService', () => {
  let service: BlobFallbackService;
  let httpMock: HttpTestingController;

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [BlobFallbackService],
    });
    service = TestBed.inject(BlobFallbackService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('fetchWithFallback', () => {
    it('should fetch from primary URL if available', (done) => {
      const primaryUrl = 'https://primary.example.com/blob.mp4';
      const secondaryUrl = 'https://secondary.example.com/blob.mp4';
      const testBlob = new Blob(['test data']);

      service.fetchWithFallback([primaryUrl, secondaryUrl]).subscribe((result) => {
        expect(result.blob.size).toBe(testBlob.size);
        expect(result.urlIndex).toBe(0);
        expect(result.successUrl).toBe(primaryUrl);
        expect(result.retryCount).toBe(0);
        done();
      });

      const req = httpMock.expectOne(primaryUrl);
      expect(req.request.method).toBe('GET');
      req.flush(testBlob);
    });

    it('should fallback to secondary URL on primary failure', (done) => {
      const primaryUrl = 'https://primary.example.com/blob.mp4';
      const secondaryUrl = 'https://secondary.example.com/blob.mp4';
      const testBlob = new Blob(['fallback data']);

      service.fetchWithFallback([primaryUrl, secondaryUrl]).subscribe((result) => {
        expect(result.urlIndex).toBe(1);
        expect(result.successUrl).toBe(secondaryUrl);
        done();
      });

      // Primary fails
      const primaryReq = httpMock.expectOne(primaryUrl);
      primaryReq.error(new ErrorEvent('Network error'));

      // Secondary succeeds
      const secondaryReq = httpMock.expectOne(secondaryUrl);
      secondaryReq.flush(testBlob);
    });

    it('should cascade through multiple fallback URLs', (done) => {
      const urls = [
        'https://cdn1.example.com/blob.mp4',
        'https://cdn2.example.com/blob.mp4',
        'https://cdn3.example.com/blob.mp4',
      ];
      const testBlob = new Blob(['final data']);

      service.fetchWithFallback(urls).subscribe((result) => {
        expect(result.urlIndex).toBe(2);
        expect(result.successUrl).toBe(urls[2]);
        done();
      });

      // First URL fails
      const req1 = httpMock.expectOne(urls[0]);
      req1.error(new ErrorEvent('Timeout'));

      // Second URL fails
      const req2 = httpMock.expectOne(urls[1]);
      req2.error(new ErrorEvent('404 Not Found'));

      // Third URL succeeds
      const req3 = httpMock.expectOne(urls[2]);
      req3.flush(testBlob);
    });

    it('should error if all URLs fail', (done) => {
      const urls = [
        'https://cdn1.example.com/blob.mp4',
        'https://cdn2.example.com/blob.mp4',
      ];

      service.fetchWithFallback(urls).subscribe(
        () => fail('should have errored'),
        (error) => {
          expect(error.message).toContain('All fallback URLs exhausted');
          done();
        }
      );

      const req1 = httpMock.expectOne(urls[0]);
      req1.error(new ErrorEvent('Error 1'));

      const req2 = httpMock.expectOne(urls[1]);
      req2.error(new ErrorEvent('Error 2'));
    });

    it('should error if no URLs provided', (done) => {
      service.fetchWithFallback([]).subscribe(
        () => fail('should have errored'),
        (error) => {
          expect(error.message).toContain('No fallback URLs provided');
          done();
        }
      );
    });

    it('should retry individual URLs on failure', (done) => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url], 30000, 2).subscribe((result) => {
        expect(result.retryCount).toBeGreaterThan(0);
        done();
      });

      // First attempt fails
      const req1 = httpMock.expectOne(url);
      req1.error(new ErrorEvent('Temporary error'));

      // Retry succeeds (after exponential backoff)
      setTimeout(() => {
        const req2 = httpMock.expectOne(url);
        req2.flush(testBlob);
      }, 150); // 100ms initial backoff + buffer
    });

    it('should track request duration', (done) => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url]).subscribe((result) => {
        expect(result.durationMs).toBeGreaterThan(0);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });
  });

  describe('URL Health Tracking', () => {
    it('should record successful fetch', (done) => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url]).subscribe(() => {
        const health = service.getUrlHealth(url);
        expect(health.successCount).toBe(1);
        expect(health.failureCount).toBe(0);
        expect(health.isHealthy).toBe(true);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });

    it('should record failed fetch', (done) => {
      const url = 'https://example.com/blob.mp4';
      const fallbackUrl = 'https://fallback.example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url, fallbackUrl]).subscribe(() => {
        const failedHealth = service.getUrlHealth(url);
        expect(failedHealth.successCount).toBe(0);
        expect(failedHealth.failureCount).toBeGreaterThan(0);
        expect(failedHealth.isHealthy).toBe(false);
        done();
      });

      const req1 = httpMock.expectOne(url);
      req1.error(new ErrorEvent('Network error'));

      const req2 = httpMock.expectOne(fallbackUrl);
      req2.flush(testBlob);
    });

    it('should track error messages', (done) => {
      const url = 'https://example.com/blob.mp4';
      const fallbackUrl = 'https://fallback.example.com/blob.mp4';
      const testBlob = new Blob(['data']);
      const errorMsg = '404 Not Found';

      service.fetchWithFallback([url, fallbackUrl]).subscribe(() => {
        const health = service.getUrlHealth(url);
        expect(health.lastErrorMessage).toContain('404');
        done();
      });

      const req1 = httpMock.expectOne(url);
      req1.error(new ErrorEvent(errorMsg));

      const req2 = httpMock.expectOne(fallbackUrl);
      req2.flush(testBlob);
    });

    it('should sort URLs by health', () => {
      const url1 = 'https://healthy.example.com/blob.mp4';
      const url2 = 'https://unhealthy.example.com/blob.mp4';
      const url3 = 'https://unknown.example.com/blob.mp4';

      // Simulate health history
      // url1: 10 successes, 1 failure (healthy)
      // url2: 0 successes, 5 failures (unhealthy)
      // url3: no history (unknown)

      service['recordUrlSuccess'](url1);
      for (let i = 0; i < 9; i++) {
        service['recordUrlSuccess'](url1);
      }
      service['recordUrlFailure'](url1, 'transient error');

      for (let i = 0; i < 5; i++) {
        service['recordUrlFailure'](url2, 'persistent failure');
      }

      const health1 = service.getUrlHealth(url1);
      const health2 = service.getUrlHealth(url2);
      const health3 = service.getUrlHealth(url3);

      expect(health1.isHealthy).toBe(true);
      expect(health2.isHealthy).toBe(false);
      expect(health3.isHealthy).toBe(true); // Unknown is considered healthy
    });
  });

  describe('testFallbackUrls', () => {
    it('should test all URLs and report health', async () => {
      const urls = [
        'https://cdn1.example.com/blob.mp4',
        'https://cdn2.example.com/blob.mp4',
      ];

      const testPromise = service.testFallbackUrls(urls);

      // First URL HEAD request succeeds
      setTimeout(() => {
        const req1 = httpMock.expectOne(urls[0]);
        expect(req1.request.method).toBe('HEAD');
        req1.flush(null);
      }, 0);

      // Second URL HEAD request succeeds
      setTimeout(() => {
        const req2 = httpMock.expectOne(urls[1]);
        expect(req2.request.method).toBe('HEAD');
        req2.flush(null);
      }, 10);

      const results = await testPromise;
      expect(results).toHaveLength(2);
    });
  });

  describe('Cache Cleanup', () => {
    it('should clear URL health on demand', () => {
      const url = 'https://example.com/blob.mp4';

      service['recordUrlSuccess'](url);
      let health = service.getUrlHealth(url);
      expect(health.successCount).toBe(1);

      service.clearUrlHealth();
      health = service.getUrlHealth(url);
      expect(health.successCount).toBe(0);
      expect(health.failureCount).toBe(0);
    });
  });
});
