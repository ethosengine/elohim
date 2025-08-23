import { TestBed } from '@angular/core/testing';
import { DOCUMENT } from '@angular/common';
import { of } from 'rxjs';
import { AnalyticsService } from './analytics.service';
import { ConfigService } from './config.service';

describe('AnalyticsService', () => {
  let service: AnalyticsService;
  let configService: jasmine.SpyObj<ConfigService>;
  let mockDocument: any;
  let mockScript: any;
  let mockWindow: any;

  beforeEach(() => {
    const configServiceSpy = jasmine.createSpyObj('ConfigService', ['getConfig']);
    
    mockScript = {
      async: false,
      src: '',
    };
    
    mockWindow = {
      dataLayer: []
    };
    
    mockDocument = {
      createElement: jasmine.createSpy('createElement').and.returnValue(mockScript),
      head: {
        appendChild: jasmine.createSpy('appendChild')
      },
      defaultView: mockWindow
    };
    
    TestBed.configureTestingModule({
      providers: [
        { provide: ConfigService, useValue: configServiceSpy },
        { provide: DOCUMENT, useValue: mockDocument }
      ]
    });
    
    configService = TestBed.inject(ConfigService) as jasmine.SpyObj<ConfigService>;
  });

  it('should be created', () => {
    configService.getConfig.and.returnValue(of({ environment: 'development', logLevel: 'debug' }));
    service = TestBed.inject(AnalyticsService);
    expect(service).toBeTruthy();
  });

  it('should inject Google Analytics script in production environment', () => {
    configService.getConfig.and.returnValue(of({ environment: 'production', logLevel: 'info' }));
    service = TestBed.inject(AnalyticsService);

    expect(mockDocument.createElement).toHaveBeenCalledWith('script');
    expect(mockScript.src).toBe('https://www.googletagmanager.com/gtag/js?id=G-NSL7PVP55B');
    expect(mockScript.async).toBe(true);
    expect(mockDocument.head.appendChild).toHaveBeenCalledWith(mockScript);
  });

  it('should NOT inject Google Analytics script in staging environment', () => {
    configService.getConfig.and.returnValue(of({ environment: 'staging', logLevel: 'debug' }));
    service = TestBed.inject(AnalyticsService);

    expect(mockDocument.createElement).not.toHaveBeenCalled();
    expect(mockDocument.head.appendChild).not.toHaveBeenCalled();
  });

  it('should NOT inject Google Analytics script in development environment', () => {
    configService.getConfig.and.returnValue(of({ environment: 'development', logLevel: 'debug' }));
    service = TestBed.inject(AnalyticsService);

    expect(mockDocument.createElement).not.toHaveBeenCalled();
    expect(mockDocument.head.appendChild).not.toHaveBeenCalled();
  });

  it('should handle missing window gracefully', () => {
    mockDocument.defaultView = null;
    configService.getConfig.and.returnValue(of({ environment: 'production', logLevel: 'info' }));
    service = TestBed.inject(AnalyticsService);

    expect(mockDocument.createElement).not.toHaveBeenCalled();
    expect(mockDocument.head.appendChild).not.toHaveBeenCalled();
  });
});