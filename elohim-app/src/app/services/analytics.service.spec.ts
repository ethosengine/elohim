import { TestBed } from '@angular/core/testing';
import { AnalyticsService } from './analytics.service';
import { ConfigService } from './config.service';

describe('AnalyticsService', () => {
  let service: AnalyticsService;
  let configService: jasmine.SpyObj<ConfigService>;
  let mockScript: HTMLScriptElement;
  let gtagSpy: jasmine.Spy;

  beforeEach(() => {
    const configServiceSpy = jasmine.createSpyObj('ConfigService', ['getConfig']);
    
    TestBed.configureTestingModule({
      providers: [
        AnalyticsService,
        { provide: ConfigService, useValue: configServiceSpy }
      ]
    });
    
    service = TestBed.inject(AnalyticsService);
    configService = TestBed.inject(ConfigService) as jasmine.SpyObj<ConfigService>;
    
    gtagSpy = jasmine.createSpy('gtag');
    (window as any).gtag = gtagSpy;
    mockScript = document.createElement('script');
    spyOn(document, 'createElement').and.returnValue(mockScript);
    spyOn(document.head, 'appendChild');
  });

  afterEach(() => {
    delete (window as any).gtag;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should initialize in production', async () => {
    configService.getConfig.and.returnValue({ environment: 'production', logLevel: 'info' });

    const promise = service.initialize();
    setTimeout(() => mockScript.onload?.({} as any), 0);
    await promise;

    expect(document.createElement).toHaveBeenCalledWith('script');
    expect(mockScript.src).toContain('G-NSL7PVP55B');
  });

  it('should not initialize in development', async () => {
    configService.getConfig.and.returnValue({ environment: 'development', logLevel: 'debug' });
    await service.initialize();

    expect(document.createElement).not.toHaveBeenCalled();
  });

  it('should handle initialization errors', async () => {
    configService.getConfig.and.throwError('Config error');
    spyOn(console, 'error');

    await service.initialize();

    expect(console.error).toHaveBeenCalled();
  });

  it('should track events when initialized', async () => {
    configService.getConfig.and.returnValue({ environment: 'production', logLevel: 'info' });
    const promise = service.initialize();
    setTimeout(() => mockScript.onload?.({} as any), 0);
    await promise;

    service.trackEvent('click', 'button', 'test', 1);

    expect(gtagSpy).toHaveBeenCalledWith('event', 'click', {
      event_category: 'button',
      event_label: 'test',
      value: 1
    });
  });

  it('should track page views when initialized', async () => {
    configService.getConfig.and.returnValue({ environment: 'production', logLevel: 'info' });
    const promise = service.initialize();
    setTimeout(() => mockScript.onload?.({} as any), 0);
    await promise;

    service.trackPageView('/home', 'Home');

    expect(gtagSpy).toHaveBeenCalledWith('config', 'G-NSL7PVP55B', {
      page_path: '/home',
      page_title: 'Home'
    });
  });

  it('should not track when not initialized', () => {
    service.trackEvent('click', 'button');
    service.trackPageView('/home');

    expect(gtagSpy).not.toHaveBeenCalled();
  });
});