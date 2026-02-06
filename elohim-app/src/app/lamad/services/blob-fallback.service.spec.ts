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

  describe('Service Initialization and Properties', () => {
    it('should initialize with an empty URL health map', () => {
      const health = service.getUrlsHealth([]);
      expect(Array.isArray(health)).toBe(true);
      expect(health.length).toBe(0);
    });

    it('should be injectable with providedIn root', () => {
      const instance1 = TestBed.inject(BlobFallbackService);
      const instance2 = TestBed.inject(BlobFallbackService);
      expect(instance1).toBe(instance2); // Singleton
    });
  });

  describe('Method Existence Tests', () => {
    it('should have fetchWithFallback method', () => {
      expect(typeof service.fetchWithFallback).toBe('function');
    });

    it('should have getUrlHealth method', () => {
      expect(typeof service.getUrlHealth).toBe('function');
    });

    it('should have getUrlsHealth method', () => {
      expect(typeof service.getUrlsHealth).toBe('function');
    });

    it('should have clearUrlHealth method', () => {
      expect(typeof service.clearUrlHealth).toBe('function');
    });

    it('should have testFallbackUrls method', () => {
      expect(typeof service.testFallbackUrls).toBe('function');
    });

    it('should have validateUrl method', () => {
      expect(typeof service.validateUrl).toBe('function');
    });

    it('should have validateUrls method', () => {
      expect(typeof service.validateUrls).toBe('function');
    });

    it('should have getValidAndHealthyUrls method', () => {
      expect(typeof service.getValidAndHealthyUrls).toBe('function');
    });
  });

  describe('fetchWithFallback', () => {
    it('should fetch from primary URL if available', done => {
      const primaryUrl = 'https://primary.example.com/blob.mp4';
      const secondaryUrl = 'https://secondary.example.com/blob.mp4';
      const testBlob = new Blob(['test data']);

      service.fetchWithFallback([primaryUrl, secondaryUrl]).subscribe(result => {
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

    it('should fallback to secondary URL on primary failure', done => {
      const primaryUrl = 'https://primary.example.com/blob.mp4';
      const secondaryUrl = 'https://secondary.example.com/blob.mp4';
      const testBlob = new Blob(['fallback data']);

      service.fetchWithFallback([primaryUrl, secondaryUrl], 30000, 0).subscribe(result => {
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

    it('should cascade through multiple fallback URLs', done => {
      const urls = [
        'https://cdn1.example.com/blob.mp4',
        'https://cdn2.example.com/blob.mp4',
        'https://cdn3.example.com/blob.mp4',
      ];
      const testBlob = new Blob(['final data']);

      service.fetchWithFallback(urls, 30000, 0).subscribe(result => {
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

    it('should error if all URLs fail', done => {
      const urls = ['https://cdn1.example.com/blob.mp4', 'https://cdn2.example.com/blob.mp4'];

      service.fetchWithFallback(urls, 30000, 0).subscribe(
        () => fail('should have errored'),
        error => {
          expect(error.message).toContain('All fallback URLs exhausted');
          done();
        }
      );

      const req1 = httpMock.expectOne(urls[0]);
      req1.error(new ErrorEvent('Error 1'));

      const req2 = httpMock.expectOne(urls[1]);
      req2.error(new ErrorEvent('Error 2'));
    });

    it('should error if no URLs provided', done => {
      service.fetchWithFallback([]).subscribe(
        () => fail('should have errored'),
        error => {
          expect(error.message).toContain('No fallback URLs provided');
          done();
        }
      );
    });

    it('should retry individual URLs on failure', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url], 30000, 2).subscribe(result => {
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

    it('should track request duration', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url]).subscribe(result => {
        expect(result.durationMs).toBeGreaterThanOrEqual(0);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });
  });

  describe('URL Health Tracking', () => {
    it('should record successful fetch', done => {
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

    it('should record failed fetch', done => {
      const url = 'https://example.com/blob.mp4';
      const fallbackUrl = 'https://fallback.example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url, fallbackUrl], 30000, 0).subscribe(() => {
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

    it('should track error messages', done => {
      const url = 'https://example.com/blob.mp4';
      const fallbackUrl = 'https://fallback.example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url, fallbackUrl], 30000, 0).subscribe(() => {
        const health = service.getUrlHealth(url);
        expect(health.lastErrorMessage).toContain(url);
        done();
      });

      const req1 = httpMock.expectOne(url);
      req1.error(new ErrorEvent('Network error'));

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
      const urls = ['https://cdn1.example.com/blob.mp4', 'https://cdn2.example.com/blob.mp4'];

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
      expect(results.length).toBe(2);
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

  describe('Observable Return Type Tests', () => {
    it('should return Observable from fetchWithFallback', () => {
      const url = 'https://example.com/blob.mp4';
      const result = service.fetchWithFallback([url]);

      expect(result).toBeDefined();
      expect(result.subscribe).toBeDefined(); // Observable characteristic
      // TODO: Add async flow tests
      httpMock.match(url).forEach(req => req.flush(new Blob(['test'])));
    });

    it('should return BlobFetchResult with all required properties', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['test data']);

      service.fetchWithFallback([url]).subscribe(result => {
        expect(result).toBeDefined();
        expect(typeof result.blob).toBe('object');
        expect(typeof result.urlIndex).toBe('number');
        expect(typeof result.successUrl).toBe('string');
        expect(typeof result.durationMs).toBe('number');
        expect(typeof result.retryCount).toBe('number');
        expect(result.successUrl).toBe(url);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });
  });

  describe('Input Validation Tests', () => {
    it('should handle null fallback URLs array', done => {
      service.fetchWithFallback(null as any).subscribe(
        () => fail('should have errored'),
        error => {
          expect(error).toBeDefined();
          expect(error.message).toContain('No fallback URLs provided');
          done();
        }
      );
    });

    it('should handle undefined fallback URLs', done => {
      service.fetchWithFallback(undefined as any).subscribe(
        () => fail('should have errored'),
        error => {
          expect(error).toBeDefined();
          done();
        }
      );
    });

    it('should handle single URL array', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url]).subscribe(result => {
        expect(result.urlIndex).toBe(0);
        expect(result.successUrl).toBe(url);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });

    it('should accept custom timeout value', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);
      const customTimeout = 5000;

      service.fetchWithFallback([url], customTimeout).subscribe(result => {
        expect(result).toBeDefined();
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });

    it('should accept custom maxRetries value', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);
      const customRetries = 3;

      service.fetchWithFallback([url], 30000, customRetries).subscribe(result => {
        expect(result).toBeDefined();
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });
  });

  describe('URL Health Interface Tests', () => {
    it('should return UrlHealth object with correct structure', () => {
      const url = 'https://example.com/blob.mp4';
      const health = service.getUrlHealth(url);

      expect(health).toBeDefined();
      expect(health.url).toBe(url);
      expect(typeof health.successCount).toBe('number');
      expect(typeof health.failureCount).toBe('number');
      expect(typeof health.isHealthy).toBe('boolean');
    });

    it('should initialize new URL health with zero counts', () => {
      const url = 'https://new-untested.example.com/blob.mp4';
      const health = service.getUrlHealth(url);

      expect(health.successCount).toBe(0);
      expect(health.failureCount).toBe(0);
      expect(health.isHealthy).toBe(true);
    });

    it('should return array of UrlHealth objects from getUrlsHealth', () => {
      const urls = ['https://url1.example.com', 'https://url2.example.com'];
      const healthArray = service.getUrlsHealth(urls);

      expect(Array.isArray(healthArray)).toBe(true);
      expect(healthArray.length).toBe(2);
      expect(healthArray[0].url).toBe(urls[0]);
      expect(healthArray[1].url).toBe(urls[1]);
    });

    it('should update lastAccessTime on success', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url]).subscribe(() => {
        const health = service.getUrlHealth(url);
        expect(health.lastAccessTime).toBeDefined();
        expect(health.lastAccessTime instanceof Date).toBe(true);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });

    it('should store lastErrorMessage on failure', done => {
      const url = 'https://example.com/blob.mp4';
      const fallbackUrl = 'https://fallback.example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url, fallbackUrl], 30000, 0).subscribe(() => {
        const health = service.getUrlHealth(url);
        expect(health.lastErrorMessage).toBeDefined();
        expect(typeof health.lastErrorMessage).toBe('string');
        done();
      });

      const req1 = httpMock.expectOne(url);
      req1.error(new ErrorEvent('Network error'));

      const req2 = httpMock.expectOne(fallbackUrl);
      req2.flush(testBlob);
    });
  });

  describe('URL Validation', () => {
    it('should validate single URL successfully', done => {
      const url = 'https://example.com/blob.mp4';

      service.validateUrl(url).then(result => {
        expect(result.url).toBe(url);
        expect(typeof result.isValid).toBe('boolean');
        expect(result.statusCode).toBeDefined();
        expect(result.responseTimeMs).toBeGreaterThanOrEqual(0);
        done();
      });

      // Mock HTTP HEAD response
      const req = httpMock.expectOne(url);
      req.flush(null, { status: 200, statusText: 'OK' });
    });

    it('should detect URL validation failures', done => {
      const url = 'https://invalid.example.com/blob.mp4';

      service.validateUrl(url).then(result => {
        expect(result.url).toBe(url);
        expect(result.isValid).toBe(false);
        expect(result.statusCode).toBe(-1);
        expect(result.errorMessage).toBeDefined();
        done();
      });

      // Mock HTTP HEAD failure
      const req = httpMock.expectOne(url);
      req.error(new ErrorEvent('error'));
    });

    it('should extract capabilities from validation headers', done => {
      const url = 'https://example.com/blob.mp4';

      service.validateUrl(url).then(result => {
        expect(result.supportsRangeRequests).toBe(true);
        expect(result.contentLength).toBe(1024);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(null, {
        status: 200,
        statusText: 'OK',
        headers: {
          'Content-Length': '1024',
          'Accept-Ranges': 'bytes',
        },
      });
    });

    it('should detect URL types (custodian vs CDN vs standard)', () => {
      const custodianUrl = 'https://custodian-123.example.com/blob.mp4';
      const cdnUrl = 'https://cdn.example.com/blob.mp4';
      const standardUrl = 'https://example.com/blob.mp4';

      expect(service['detectUrlType'](custodianUrl)).toBe('custodian');
      expect(service['detectUrlType'](cdnUrl)).toBe('cdn');
      expect(service['detectUrlType'](standardUrl)).toBe('standard');
    });

    it('should return UrlValidationResult with all required properties', done => {
      const url = 'https://example.com/blob.mp4';

      service.validateUrl(url).then(result => {
        expect(result).toBeDefined();
        expect(result.url).toBe(url);
        expect(typeof result.isValid).toBe('boolean');
        expect(typeof result.statusCode).toBe('number');
        expect(typeof result.type).toBe('string');
        expect(typeof result.responseTimeMs).toBe('number');
        expect(['standard', 'custodian', 'cloudfront', 'cdn', 'unknown']).toContain(result.type);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(null, { status: 200, statusText: 'OK' });
    });

    it('should handle invalid URL format gracefully', done => {
      const invalidUrl = 'not-a-valid-url';

      service.validateUrl(invalidUrl).then(result => {
        expect(result.url).toBe(invalidUrl);
        expect(result.type).toBe('unknown');
        done();
      });

      const req = httpMock.expectOne(invalidUrl);
      req.error(new ErrorEvent('Invalid URL'));
    });

    it('should detect cloudfront URLs', () => {
      const cloudfrontUrl = 'https://d123.cloudfront.net/blob.mp4';
      const detectedType = service['detectUrlType'](cloudfrontUrl);
      expect(detectedType).toBe('cloudfront');
    });

    it('should detect akamai CDN URLs', () => {
      const akamaiUrl = 'https://akamai.example.com/blob.mp4';
      const detectedType = service['detectUrlType'](akamaiUrl);
      expect(detectedType).toBe('cdn');
    });

    it('should detect fastly CDN URLs', () => {
      const fastlyUrl = 'https://fastly.example.com/blob.mp4';
      const detectedType = service['detectUrlType'](fastlyUrl);
      expect(detectedType).toBe('cdn');
    });

    it('should validate multiple URLs in parallel', done => {
      const urls = ['https://example.com/blob1.mp4', 'https://example.com/blob2.mp4'];

      service.validateUrls(urls).then(results => {
        expect(results.length).toBe(2);
        expect(results[0].url).toBe(urls[0]);
        expect(results[1].url).toBe(urls[1]);
        done();
      });

      const reqs = httpMock.match(req => urls.includes(req.url));
      expect(reqs.length).toBe(2);
      reqs.forEach(req => {
        req.flush(null, { status: 200, statusText: 'OK' });
      });
    });

    it('should filter to valid and healthy URLs only', done => {
      const urls = ['https://example.com/blob1.mp4', 'https://example.com/blob2.mp4'];

      // Mark second URL as healthy in history
      service['recordUrlSuccess'](urls[1]);

      service.getValidAndHealthyUrls(urls).then(validUrls => {
        // Results should include URLs that passed validation and are healthy
        expect(Array.isArray(validUrls)).toBe(true);
        done();
      });

      const reqs = httpMock.match(req => urls.includes(req.url));
      reqs.forEach(req => {
        req.flush(null, { status: 200, statusText: 'OK' });
      });
    });
  });

  describe('Error Message Extraction Tests', () => {
    it('should extract error message from HttpErrorResponse', done => {
      const url = 'https://example.com/blob.mp4';
      const fallbackUrl = 'https://fallback.example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url, fallbackUrl], 30000, 0).subscribe(() => {
        const health = service.getUrlHealth(url);
        expect(health.lastErrorMessage).toBeDefined();
        expect(health.lastErrorMessage).toContain('Http failure');
        done();
      });

      const req1 = httpMock.expectOne(url);
      req1.flush(null, { status: 404, statusText: 'Not Found' });

      const req2 = httpMock.expectOne(fallbackUrl);
      req2.flush(testBlob);
    });

    it('should extract error message from generic Error object', done => {
      const url = 'https://example.com/blob.mp4';
      const fallbackUrl = 'https://fallback.example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url, fallbackUrl], 30000, 0).subscribe(() => {
        const health = service.getUrlHealth(url);
        expect(health.lastErrorMessage).toBeDefined();
        done();
      });

      const req1 = httpMock.expectOne(url);
      req1.error(new ErrorEvent('Network timeout'));

      const req2 = httpMock.expectOne(fallbackUrl);
      req2.flush(testBlob);
    });

    it('should extract error message from string type error', done => {
      const url = 'https://example.com/blob.mp4';
      const fallbackUrl = 'https://fallback.example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url, fallbackUrl], 30000, 0).subscribe(() => {
        const health = service.getUrlHealth(url);
        expect(health.lastErrorMessage).toBeDefined();
        done();
      });

      const req1 = httpMock.expectOne(url);
      req1.error(new ErrorEvent('Connection refused'));

      const req2 = httpMock.expectOne(fallbackUrl);
      req2.flush(testBlob);
    });
  });

  describe('Blob Property Tests', () => {
    it('should return Blob with correct type from successful fetch', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['test data'], { type: 'video/mp4' });

      service.fetchWithFallback([url]).subscribe(result => {
        expect(result.blob).toBeDefined();
        expect(result.blob instanceof Blob).toBe(true);
        expect(result.blob.size).toBeGreaterThan(0);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });

    it('should return empty Blob when server returns empty response', done => {
      const url = 'https://example.com/blob.mp4';
      const emptyBlob = new Blob([]);

      service.fetchWithFallback([url]).subscribe(result => {
        expect(result.blob).toBeDefined();
        expect(result.blob instanceof Blob).toBe(true);
        expect(result.blob.size).toBe(0);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(emptyBlob);
    });
  });

  describe('Duration and Retry Count Tests', () => {
    it('should track zero retry count on first success', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url], 30000, 0).subscribe(result => {
        expect(result.retryCount).toBe(0);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });

    it('should track non-negative duration time', done => {
      const url = 'https://example.com/blob.mp4';
      const testBlob = new Blob(['data']);

      service.fetchWithFallback([url]).subscribe(result => {
        expect(result.durationMs).toBeGreaterThanOrEqual(0);
        done();
      });

      const req = httpMock.expectOne(url);
      req.flush(testBlob);
    });

    it('should increment urlIndex for each cascaded URL', done => {
      const urls = ['https://url1.example.com', 'https://url2.example.com', 'https://url3.example.com'];
      const testBlob = new Blob(['data']);

      service.fetchWithFallback(urls, 30000, 0).subscribe(result => {
        expect(result.urlIndex).toBe(2);
        done();
      });

      const req1 = httpMock.expectOne(urls[0]);
      req1.error(new ErrorEvent('Error 1'));

      const req2 = httpMock.expectOne(urls[1]);
      req2.error(new ErrorEvent('Error 2'));

      const req3 = httpMock.expectOne(urls[2]);
      req3.flush(testBlob);
    });
  });
});
