import { provideHttpClient } from '@angular/common/http';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { of } from 'rxjs';

import { AnalyticsService } from '../../services/analytics.service';
import { ConfigService } from '../../services/config.service';
import { DomInteractionService } from '../../services/dom-interaction.service';

import { HomeComponent } from './home.component';

describe('HomeComponent', () => {
  let component: HomeComponent;
  let fixture: ComponentFixture<HomeComponent>;
  let mockConfigService: jasmine.SpyObj<ConfigService>;
  let mockAnalyticsService: jasmine.SpyObj<AnalyticsService>;
  let mockDomInteractionService: jasmine.SpyObj<DomInteractionService>;

  beforeEach(async () => {
    mockConfigService = jasmine.createSpyObj('ConfigService', ['getConfig']);
    mockAnalyticsService = jasmine.createSpyObj('AnalyticsService', ['trackEvent']);
    mockDomInteractionService = jasmine.createSpyObj('DomInteractionService', [
      'setupScrollIndicator',
      'setupHeroTitleAnimation',
    ]);

    mockConfigService.getConfig.and.returnValue(
      of({
        logLevel: 'info' as const,
        environment: 'test',
      })
    );

    await TestBed.configureTestingModule({
      imports: [HomeComponent],
      providers: [
        { provide: ConfigService, useValue: mockConfigService },
        { provide: AnalyticsService, useValue: mockAnalyticsService },
        { provide: DomInteractionService, useValue: mockDomInteractionService },
        provideRouter([]),
        provideHttpClient(),
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(HomeComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should call config service on init', () => {
    fixture.detectChanges();
    expect(mockConfigService.getConfig).toHaveBeenCalled();
  });

  it('should setup scroll listeners on init', done => {
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- spying on private method
    const scrollSpy = spyOn<any>(component, 'setupParallaxScrolling');
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- spying on private method
    const observerSpy = spyOn<any>(component, 'setupIntersectionObserver');

    fixture.detectChanges();

    // Wait for async operations
    setTimeout(() => {
      expect(scrollSpy).toHaveBeenCalled();
      expect(observerSpy).toHaveBeenCalled();
      expect(mockDomInteractionService.setupScrollIndicator).toHaveBeenCalled();
      expect(mockDomInteractionService.setupHeroTitleAnimation).toHaveBeenCalled();
      done();
    }, 100);
  });

  it('should cleanup on destroy', () => {
    fixture.detectChanges();

    // Simulate having listeners - accessing private properties for testing
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- testing private property
    (component as any).scrollListener = () => {};
    const mockObserver = {
      disconnect: jasmine.createSpy('disconnect'),
    };
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- testing private property
    (component as any).intersectionObserver = mockObserver;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any -- testing private property
    (component as any).rafId = 123;

    const cancelSpy = spyOn(window, 'cancelAnimationFrame');

    component.ngOnDestroy();

    expect(mockObserver.disconnect).toHaveBeenCalled();
    expect(cancelSpy).toHaveBeenCalledWith(123);
  });

  it('should render all main sections', () => {
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;

    expect(compiled.querySelector('app-hero')).toBeTruthy();
    expect(compiled.querySelector('app-crisis')).toBeTruthy();
    expect(compiled.querySelector('app-vision')).toBeTruthy();
    expect(compiled.querySelector('app-elohim-host')).toBeTruthy();
    expect(compiled.querySelector('app-design-principles')).toBeTruthy();
    expect(compiled.querySelector('app-learning-success')).toBeTruthy();
    expect(compiled.querySelector('app-path-forward')).toBeTruthy();
    expect(compiled.querySelector('app-call-to-action')).toBeTruthy();
    expect(compiled.querySelector('app-footer')).toBeTruthy();
  });

  it('should render debug bar', () => {
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('app-debug-bar')).toBeTruthy();
  });
});
