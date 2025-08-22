import { TestBed } from '@angular/core/testing';
import { DOCUMENT } from '@angular/common';
import { of, throwError } from 'rxjs';
import { AnalyticsService, AnalyticsEvent, PageView } from './analytics.service';
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
      onload: null,
      onerror: null
    };
    
    mockWindow = {
      dataLayer: [],
      gtag: jasmine.createSpy('gtag')
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

  it('should track events in production environment', (done) => {
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
      error: (err) => fail('Should not error: ' + err)
    });

    // Simulate successful script load
    setTimeout(() => {
      if (mockScript.onload) {
        mockScript.onload(new Event('load'));
      }
    }, 0);
  });

  it('should track page views in production environment', (done) => {
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
      error: (err) => fail('Should not error: ' + err)
    });

    // Simulate successful script load
    setTimeout(() => {
      if (mockScript.onload) {
        mockScript.onload(new Event('load'));
      }
    }, 0);
  });

  it('should NOT track in staging environment', (done) => {
    configService.getConfig.and.returnValue(of({ environment: 'staging', logLevel: 'debug' }));

    const event: AnalyticsEvent = {
      action: 'click',
      category: 'button'
    };

    service.trackEvent(event).subscribe({
      next: () => {
        fail('Should not emit in staging environment');
      },
      error: () => {
        fail('Should not error in staging environment');
      },
      complete: () => {
        expect(mockDocument.createElement).not.toHaveBeenCalled();
        expect(mockWindow.gtag).not.toHaveBeenCalled();
        done();
      }
    });
  });

  it('should NOT track in development environment', (done) => {
    configService.getConfig.and.returnValue(of({ environment: 'development', logLevel: 'debug' }));

    const event: AnalyticsEvent = {
      action: 'click',
      category: 'button'
    };

    service.trackEvent(event).subscribe({
      next: () => {
        fail('Should not emit in development environment');
      },
      error: () => {
        fail('Should not error in development environment');
      },
      complete: () => {
        expect(mockDocument.createElement).not.toHaveBeenCalled();
        expect(mockWindow.gtag).not.toHaveBeenCalled();
        done();
      }
    });
  });

  it('should handle script loading errors gracefully', (done) => {
    configService.getConfig.and.returnValue(of({ environment: 'production', logLevel: 'info' }));
    spyOn(console, 'error');

    const event: AnalyticsEvent = {
      action: 'click',
      category: 'button'
    };

    service.trackEvent(event).subscribe({
      next: () => {
        fail('Should not emit on script error');
      },
      error: () => {
        fail('Error should be caught and logged');
      },
      complete: () => {
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

  it('should handle config service errors gracefully', (done) => {
    configService.getConfig.and.returnValue(throwError(() => new Error('Config error')));
    spyOn(console, 'error');

    const event: AnalyticsEvent = {
      action: 'click',
      category: 'button'
    };

    service.trackEvent(event).subscribe({
      next: () => {
        fail('Should not emit on config error');
      },
      error: () => {
        fail('Error should be caught and logged');
      },
      complete: () => {
        expect(console.error).toHaveBeenCalledWith('Analytics initialization failed:', jasmine.any(Error));
        done();
      }
    });
  });
});