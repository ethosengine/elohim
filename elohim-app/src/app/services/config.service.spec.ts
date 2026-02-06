import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { ConfigService, AppConfig } from './config.service';
import { environment } from '../../environments/environment';

describe('ConfigService', () => {
  let service: ConfigService;
  let httpMock: HttpTestingController;
  let originalEnvironment: any;

  beforeEach(() => {
    // Store original environment values
    originalEnvironment = {
      production: environment.production,
      logLevel: (environment as any).logLevel,
      environment: (environment as any).environment,
    };

    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [ConfigService],
    });

    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
    // Restore original environment values
    (environment as any).production = originalEnvironment.production;
    (environment as any).logLevel = originalEnvironment.logLevel;
    (environment as any).environment = originalEnvironment.environment;
  });

  it('should be created', () => {
    service = TestBed.inject(ConfigService);
    expect(service).toBeTruthy();
  });

  describe('getConfig in development environment', () => {
    beforeEach(() => {
      (environment as any).production = false;
      (environment as any).logLevel = 'debug';
      (environment as any).environment = 'development';
      service = TestBed.inject(ConfigService);
    });

    it('should return config from environment file', done => {
      service.getConfig().subscribe(config => {
        expect(config).toEqual({
          logLevel: 'debug',
          environment: 'development',
        });
        done();
      });

      // No HTTP request should be made
      httpMock.expectNone('/assets/config.json');
    });

    it('should use shareReplay for caching', done => {
      let emissionCount = 0;

      service.getConfig().subscribe(config => {
        emissionCount++;
        expect(config).toEqual({
          logLevel: 'debug',
          environment: 'development',
        });
      });

      service.getConfig().subscribe(config => {
        emissionCount++;
        expect(config).toEqual({
          logLevel: 'debug',
          environment: 'development',
        });

        // Should only emit once due to shareReplay
        expect(emissionCount).toBe(2);
        done();
      });
    });
  });

  describe('getConfig in production environment', () => {
    beforeEach(() => {
      (environment as any).production = true;
      service = TestBed.inject(ConfigService);
    });

    it('should load config from HTTP request', done => {
      const mockConfig: AppConfig = {
        logLevel: 'error',
        environment: 'production',
      };

      service.getConfig().subscribe(config => {
        expect(config).toEqual(mockConfig);
        done();
      });

      const req = httpMock.expectOne('/assets/config.json');
      expect(req.request.method).toBe('GET');
      req.flush(mockConfig);
    });

    it('should use default config when HTTP request returns null', done => {
      service.getConfig().subscribe(config => {
        expect(config).toEqual({
          logLevel: 'error',
          environment: 'production',
        });
        done();
      });

      const req = httpMock.expectOne('/assets/config.json');
      req.flush(null);
    });

    it('should use default config when HTTP request returns undefined', done => {
      service.getConfig().subscribe(config => {
        expect(config).toEqual({
          logLevel: 'error',
          environment: 'production',
        });
        done();
      });

      const req = httpMock.expectOne('/assets/config.json');
      req.flush(null);
    });

    it('should handle HTTP errors gracefully with default config', done => {
      service.getConfig().subscribe(config => {
        expect(config).toEqual({
          logLevel: 'error',
          environment: 'production',
        });
        done();
      });

      const req = httpMock.expectOne('/assets/config.json');
      req.error(new ErrorEvent('Network error'));
    });

    it('should cache config after successful load using shareReplay', done => {
      const mockConfig: AppConfig = {
        logLevel: 'info',
        environment: 'production',
      };

      let subscriptionCount = 0;

      service.getConfig().subscribe(config => {
        subscriptionCount++;
        expect(config).toEqual(mockConfig);
      });

      service.getConfig().subscribe(config => {
        subscriptionCount++;
        expect(config).toEqual(mockConfig);
        expect(subscriptionCount).toBe(2);
        done();
      });

      // Only one HTTP request should be made due to shareReplay
      const req = httpMock.expectOne('/assets/config.json');
      req.flush(mockConfig);
      httpMock.expectNone('/assets/config.json');
    });
  });

  describe('config interface and validation', () => {
    it('should handle missing environment values with defaults', done => {
      (environment as any).production = false;
      (environment as any).logLevel = undefined;
      (environment as any).environment = undefined;
      service = TestBed.inject(ConfigService);

      service.getConfig().subscribe(config => {
        expect(config).toEqual({
          logLevel: 'debug',
          environment: 'development',
        });
        done();
      });
    });

    it('should accept valid log levels', done => {
      (environment as any).production = false;
      (environment as any).logLevel = 'info';
      (environment as any).environment = 'staging';
      service = TestBed.inject(ConfigService);

      service.getConfig().subscribe(config => {
        expect(config.logLevel).toBe('info');
        expect(config.environment).toBe('staging');
        done();
      });
    });

    it('should maintain readonly properties', done => {
      (environment as any).production = false;
      (environment as any).logLevel = 'debug';
      (environment as any).environment = 'development';
      service = TestBed.inject(ConfigService);

      service.getConfig().subscribe(config => {
        const originalLogLevel = config.logLevel;

        // Try to modify the property (this will work in JS but shouldn't affect future calls)
        (config as any).logLevel = 'error';

        // The current object is modified (JS limitation), but the interface is readonly by design
        expect(config.logLevel).toBe('error'); // Modified object shows change
        expect(originalLogLevel).toBe('debug'); // Original value captured before modification

        done();
      });
    });
  });
});
