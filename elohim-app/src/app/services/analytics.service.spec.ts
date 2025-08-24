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
  let mockMeta: any;
  let mockWindow: any;

  beforeEach(() => {
    const configServiceSpy = jasmine.createSpyObj('ConfigService', ['getConfig']);
    
    mockScript = {
      async: false,
      src: '',
    };
    
    mockMeta = {
      name: '',
      content: ''
    };
    
    mockWindow = {
      dataLayer: []
    };
    
    mockDocument = {
      createElement: jasmine.createSpy('createElement').and.callFake((tagName: string) => {
        return tagName === 'script' ? mockScript : mockMeta;
      }),
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

  it('should inject no-indexing meta in staging environment', () => {
    configService.getConfig.and.returnValue(of({ environment: 'staging', logLevel: 'debug' }));
    service = TestBed.inject(AnalyticsService);

    expect(mockDocument.createElement).toHaveBeenCalledWith('meta');
    expect(mockMeta.name).toBe('robots');
    expect(mockMeta.content).toBe('noindex, nofollow, noarchive, nosnippet');
    expect(mockDocument.head.appendChild).toHaveBeenCalledWith(mockMeta);
  });

  it('should NOT inject no-indexing meta in production environment', () => {
    configService.getConfig.and.returnValue(of({ environment: 'production', logLevel: 'info' }));
    service = TestBed.inject(AnalyticsService);

    const createElementCalls = mockDocument.createElement.calls.all();
    const metaCall = createElementCalls.find((call: any) => call.args[0] === 'meta');
    expect(metaCall).toBeUndefined();
    
    const appendChildCalls = mockDocument.head.appendChild.calls.all();
    const metaAppendCall = appendChildCalls.find((call: any) => call.args[0] === mockMeta);
    expect(metaAppendCall).toBeUndefined();
  });
});