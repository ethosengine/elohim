import { TestBed } from '@angular/core/testing';
import { HttpClientTestingModule, HttpTestingController } from '@angular/common/http/testing';
import { ConfigService, AppConfig } from './config.service';
import { environment } from '../../environments/environment';

describe('ConfigService', () => {
  let service: ConfigService;
  let httpMock: HttpTestingController;
  let originalProduction: boolean;

  beforeEach(() => {
    originalProduction = environment.production;
    
    TestBed.configureTestingModule({
      imports: [HttpClientTestingModule],
      providers: [ConfigService]
    });
    
    service = TestBed.inject(ConfigService);
    httpMock = TestBed.inject(HttpTestingController);
  });

  afterEach(() => {
    httpMock.verify();
    // Restore original environment
    (environment as any).production = originalProduction;
    // Reset service cache for next test
    (service as any).configCache = null;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('loadConfig in development environment', () => {
    beforeEach(() => {
      (environment as any).production = false;
      (environment as any).logLevel = 'debug';
      (environment as any).environment = 'development';
    });

    it('should load config from environment file', async () => {
      const config = await service.loadConfig();

      expect(config).toEqual({
        logLevel: 'debug',
        environment: 'development'
      });
      
      // No HTTP request should be made
      httpMock.expectNone('/assets/config.json');
    });

    it('should cache config after first load', async () => {
      const config1 = await service.loadConfig();
      const config2 = await service.loadConfig();

      expect(config1).toEqual(config2);
      expect(config1).toEqual({
        logLevel: 'debug',
        environment: 'development'
      });
    });
  });

  describe('loadConfig in production environment', () => {
    beforeEach(() => {
      (environment as any).production = true;
    });

    it('should load config from HTTP request', async () => {
      const mockConfig: AppConfig = {
        logLevel: 'error',
        environment: 'production'
      };

      const configPromise = service.loadConfig();

      const req = httpMock.expectOne('/assets/config.json');
      expect(req.request.method).toBe('GET');
      req.flush(mockConfig);

      const config = await configPromise;
      expect(config).toEqual(mockConfig);
    });

    it('should use default config when HTTP request returns null', async () => {
      const configPromise = service.loadConfig();

      const req = httpMock.expectOne('/assets/config.json');
      req.flush(null);

      const config = await configPromise;
      expect(config).toEqual({
        logLevel: 'error',
        environment: 'production'
      });
    });

    it('should use default config when HTTP request returns undefined', async () => {
      const configPromise = service.loadConfig();

      const req = httpMock.expectOne('/assets/config.json');
      req.flush(null);

      const config = await configPromise;
      expect(config).toEqual({
        logLevel: 'error',
        environment: 'production'
      });
    });

    it('should handle HTTP errors gracefully', async () => {
      const configPromise = service.loadConfig();

      const req = httpMock.expectOne('/assets/config.json');
      req.error(new ErrorEvent('Network error'));

      try {
        await configPromise;
        fail('Expected promise to reject');
      } catch (error) {
        expect(error).toBeDefined();
      }
    });

    it('should cache config after successful load', async () => {
      const mockConfig: AppConfig = {
        logLevel: 'info',
        environment: 'production'
      };

      const configPromise1 = service.loadConfig();
      const req = httpMock.expectOne('/assets/config.json');
      req.flush(mockConfig);
      const config1 = await configPromise1;

      // Second call should not make HTTP request due to caching
      const config2 = await service.loadConfig();
      httpMock.expectNone('/assets/config.json');

      expect(config1).toEqual(config2);
      expect(config1).toEqual(mockConfig);
    });
  });

  describe('getConfig', () => {
    it('should return config when loaded in development', async () => {
      (environment as any).production = false;
      (environment as any).logLevel = 'debug';
      (environment as any).environment = 'development';

      await service.loadConfig();
      const config = service.getConfig();

      expect(config).toEqual({
        logLevel: 'debug',
        environment: 'development'
      });
    });

    it('should return config when loaded in production', async () => {
      (environment as any).production = true;
      const mockConfig: AppConfig = {
        logLevel: 'error',
        environment: 'production'
      };

      const configPromise = service.loadConfig();
      const req = httpMock.expectOne('/assets/config.json');
      req.flush(mockConfig);
      await configPromise;

      const config = service.getConfig();
      expect(config).toEqual(mockConfig);
    });

    it('should throw error when config not loaded', () => {
      expect(() => service.getConfig()).toThrowError(/Config not loaded/);
    });
  });

  describe('config interface', () => {
    it('should accept valid log levels', async () => {
      (environment as any).production = false;
      
      // Test debug level
      (environment as any).logLevel = 'debug';
      await service.loadConfig();
      expect(service.getConfig().logLevel).toBe('debug');

      // Test other log levels
      const logLevels = ['info', 'error'];
      for (const level of logLevels) {
        service = TestBed.inject(ConfigService);
        (environment as any).logLevel = level;
        await service.loadConfig();
        expect(service.getConfig().logLevel).toBe(level);
      }
    });

    it('should accept valid environment strings', async () => {
      (environment as any).production = false;
      (environment as any).logLevel = 'debug';
      
      const environments = ['development', 'staging', 'production'];
      for (const env of environments) {
        service = TestBed.inject(ConfigService);
        (environment as any).environment = env;
        await service.loadConfig();
        expect(service.getConfig().environment).toBe(env);
      }
    });
  });
});