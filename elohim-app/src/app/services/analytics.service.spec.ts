import { TestBed } from '@angular/core/testing';
import { DOCUMENT } from '@angular/common';
import { of, throwError } from 'rxjs';
import { AnalyticsService, AnalyticsEvent, PageView } from './analytics.service';
import { ConfigService } from './config.service';

describe('AnalyticsService', () => {
  let service: AnalyticsService;
  let configService: jasmine.SpyObj<ConfigService>;
  let mockDocument: jasmine.SpyObj<Document>;
  let mockScript: jasmine.SpyObj<HTMLScriptElement>;
  let mockWindow: any;

  beforeEach(() => {
    const configServiceSpy = jasmine.createSpyObj('ConfigService', ['getConfig']);
    
    mockScript = jasmine.createSpyObj('HTMLScriptElement', [], {
      async: true,
      src: '',
      onload: null,
      onerror: null
    });
    
    mockWindow = {
      dataLayer: [],
      gtag: jasmine.createSpy('gtag')
    };
    
    mockDocument = jasmine.createSpyObj('Document', ['createElement'], {
      head: jasmine.createSpyObj('HTMLHeadElement', ['appendChild']),
      defaultView: mockWindow
    });
    
    mockDocument.createElement.and.returnValue(mockScript);
    
    TestBed.configureTestingModule({
      providers: [
        AnalyticsService,
        { provide: ConfigService, useValue: configServiceSpy },
        { provide: DOCUMENT, useValue: mockDocument }
      ]
    });
    
    service = TestBed.inject(AnalyticsService);
    configService = TestBed.inject(ConfigService) as jasmine.SpyObj<ConfigService>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should track events in production', (done) => {
    configService.getConfig.and.returnValue(of({ environment: 'production', logLevel: 'info' }));

    const event: AnalyticsEvent = {
      action: 'click',
      category: 'button',
      label: 'test',
      value: 1
    };

    service.trackEvent(event).subscribe({
      next: () => {
        expect(mockDocument.createElement).toHaveBeenCalledWith('script');
        expect(mockScript.src).toContain('G-NSL7PVP55B');
        expect(mockWindow.gtag).toHaveBeenCalledWith('event', 'click', {
          event_category: 'button',
          event_label: 'test',
          value: 1
        });
        done();
      },
      error: (err) => {
        fail('Should not error: ' + err);
      }
    });

    // Simulate script load
    setTimeout(() => {
      if (mockScript.onload) {
        mockScript.onload(new Event('load'));
      }
    }, 0);
  });

  it('should track page views in production', (done) => {
    configService.getConfig.and.returnValue(of({ environment: 'production', logLevel: 'info' }));

    const pageView: PageView = {
      path: '/home',
      title: 'Home'
    };

    service.trackPageView(pageView).subscribe({
      next: () => {
        expect(mockWindow.gtag).toHaveBeenCalledWith('config', 'G-NSL7PVP55B', {
          page_path: '/home',
          page_title: 'Home'
        });
        done();
      },
      error: (err) => {
        fail('Should not error: ' + err);
      }
    });

    // Simulate script load
    setTimeout(() => {
      if (mockScript.onload) {
        mockScript.onload(new Event('load'));
      }
    }, 0);
  });

  it('should not track in development', (done) => {
    configService.getConfig.and.returnValue(of({ environment: 'development', logLevel: 'debug' }));

    const event: AnalyticsEvent = {
      action: 'click',
      category: 'button'
    };

    service.trackEvent(event).subscribe({
      next: () => {
        fail('Should not emit in development');
      },
      error: () => {
        fail('Should not error in development');
      },
      complete: () => {
        expect(mockDocument.createElement).not.toHaveBeenCalled();
        done();
      }
    });
  });

  it('should handle script loading errors', (done) => {
    configService.getConfig.and.returnValue(of({ environment: 'production', logLevel: 'info' }));
    spyOn(console, 'error');

    const event: AnalyticsEvent = {
      action: 'click',
      category: 'button'
    };

    // When there's an error, the stream returns EMPTY which completes immediately
    service.trackEvent(event).subscribe({
      next: () => {
        fail('Should not emit on script error');
      },
      error: () => {
        fail('Error should be caught and logged');
      },
      complete: () => {
        // The stream completes immediately due to EMPTY, check error was logged
        expect(console.error).toHaveBeenCalledWith('Analytics initialization failed:', jasmine.any(Error));
        done();
      }
    });

    // Simulate script error
    setTimeout(() => {
      if (mockScript.onerror) {
        mockScript.onerror(new Event('error'));
      }
    }, 0);
  });

  it('should handle config service errors', (done) => {
    configService.getConfig.and.returnValue(throwError(() => new Error('Config error')));
    spyOn(console, 'error');

    const event: AnalyticsEvent = {
      action: 'click',
      category: 'button'
    };

    // When config service errors, the stream returns EMPTY which completes immediately
    service.trackEvent(event).subscribe({
      next: () => {
        fail('Should not emit on config error');
      },
      error: () => {
        fail('Error should be caught and logged');
      },
      complete: () => {
        // The stream completes immediately due to EMPTY, check error was logged
        expect(console.error).toHaveBeenCalledWith('Analytics initialization failed:', jasmine.any(Error));
        done();
      }
    });
  });
});