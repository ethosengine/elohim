import { provideHttpClient } from '@angular/common/http';
import { TestBed } from '@angular/core/testing';
import { provideRouter, Router, NavigationEnd } from '@angular/router';

import { Observable, Subject } from 'rxjs';

import { AppComponent } from './app.component';
import { HolochainClientService } from './elohim/services/holochain-client.service';
import { HolochainContentService } from './elohim/services/holochain-content.service';
import { TauriAuthService } from './imagodei/services/tauri-auth.service';
import { BlobBootstrapService } from './lamad/services/blob-bootstrap.service';

describe('AppComponent', () => {
  let routerEventsSubject: Subject<NavigationEnd>;
  let mockRouter: { events: Observable<NavigationEnd>; url: string; navigate: jasmine.Spy };
  let mockHolochainClient: jasmine.SpyObj<HolochainClientService>;
  let mockHolochainContent: jasmine.SpyObj<HolochainContentService>;
  let mockTauriAuth: jasmine.SpyObj<TauriAuthService>;
  let mockBlobBootstrap: jasmine.SpyObj<BlobBootstrapService>;

  beforeEach(async () => {
    routerEventsSubject = new Subject();
    mockRouter = {
      events: routerEventsSubject.asObservable(),
      url: '/',
      navigate: jasmine.createSpy('navigate').and.returnValue(Promise.resolve(true)),
    };

    // Create mock services
    mockHolochainClient = jasmine.createSpyObj('HolochainClientService', ['connect', 'isConnected']);
    mockHolochainContent = jasmine.createSpyObj('HolochainContentService', ['testAvailability']);
    mockTauriAuth = jasmine.createSpyObj('TauriAuthService', [
      'isTauriEnvironment',
      'initialize',
      'needsLogin',
      'destroy',
    ]);
    mockBlobBootstrap = jasmine.createSpyObj('BlobBootstrapService', ['startBootstrap']);

    // Default mock behavior
    mockHolochainClient.connect.and.returnValue(Promise.resolve());
    mockHolochainClient.isConnected.and.returnValue(true);
    mockHolochainContent.testAvailability.and.returnValue(Promise.resolve(true));
    mockTauriAuth.isTauriEnvironment.and.returnValue(false);
    mockTauriAuth.initialize.and.returnValue(Promise.resolve());
    mockTauriAuth.needsLogin.and.returnValue(false);
    mockBlobBootstrap.startBootstrap.and.returnValue(Promise.resolve());

    await TestBed.configureTestingModule({
      imports: [AppComponent],
      providers: [
        provideHttpClient(),
        provideRouter([]),
        { provide: Router, useValue: mockRouter },
        { provide: HolochainClientService, useValue: mockHolochainClient },
        { provide: HolochainContentService, useValue: mockHolochainContent },
        { provide: TauriAuthService, useValue: mockTauriAuth },
        { provide: BlobBootstrapService, useValue: mockBlobBootstrap },
      ],
    }).compileComponents();
  });

  it('should create the app', () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;
    expect(app).toBeTruthy();
  });

  it(`should have the 'elohim-app' title`, () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;
    expect(app.title).toEqual('elohim-app');
  });

  it('should render router outlet', () => {
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('router-outlet')).toBeTruthy();
  });

  it('should show floating toggle on root landing page (/)', () => {
    mockRouter.url = '/';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.showFloatingToggle).toBe(true);
  });

  it('should hide floating toggle on lamad routes', () => {
    mockRouter.url = '/lamad/home';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.showFloatingToggle).toBe(false);
  });

  it('should hide floating toggle on shefa routes', () => {
    mockRouter.url = '/shefa';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.showFloatingToggle).toBe(false);
  });

  it('should hide floating toggle on qahal routes', () => {
    mockRouter.url = '/qahal';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.showFloatingToggle).toBe(false);
  });

  it('should update showFloatingToggle when navigating away from root', () => {
    mockRouter.url = '/';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();
    expect(app.showFloatingToggle).toBe(true);

    routerEventsSubject.next(new NavigationEnd(1, '/lamad/search', '/lamad/search'));

    expect(app.showFloatingToggle).toBe(false);
  });

  it('should update showFloatingToggle when navigating to root', () => {
    mockRouter.url = '/lamad/home';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();
    expect(app.showFloatingToggle).toBe(false);

    routerEventsSubject.next(new NavigationEnd(2, '/', '/'));

    expect(app.showFloatingToggle).toBe(true);
  });

  it('should render theme toggle component on root page', () => {
    mockRouter.url = '/';
    const fixture = TestBed.createComponent(AppComponent);
    fixture.componentInstance.ngOnInit();
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('app-theme-toggle')).toBeTruthy();
  });

  it('should not render theme toggle component on non-root pages', () => {
    mockRouter.url = '/lamad';
    const fixture = TestBed.createComponent(AppComponent);
    fixture.componentInstance.ngOnInit();
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('app-theme-toggle')).toBeFalsy();
  });

  describe('ngOnDestroy', () => {
    it('should clear retry timeout on destroy', () => {
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;
      fixture.detectChanges();

      // Simulate a retry timeout being set
      (app as any).retryTimeoutId = setTimeout(() => {}, 5000);
      const timeoutId = (app as any).retryTimeoutId;

      app.ngOnDestroy();

      expect((app as any).retryTimeoutId).toBeNull();
      expect((app as any).isDestroyed).toBe(true);
    });

    it('should call tauriAuth.destroy on cleanup', () => {
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;
      fixture.detectChanges();

      app.ngOnDestroy();

      expect(mockTauriAuth.destroy).toHaveBeenCalled();
    });

    it('should handle destroy when no retry timeout exists', () => {
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;
      fixture.detectChanges();

      (app as any).retryTimeoutId = null;

      expect(() => app.ngOnDestroy()).not.toThrow();
      expect((app as any).isDestroyed).toBe(true);
    });
  });

  describe('retryConnection', () => {
    it('should reset connection attempt counter', () => {
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;
      fixture.detectChanges();

      // Simulate failed attempts
      (app as any).connectionAttempt = 3;

      app.retryConnection();

      expect((app as any).connectionAttempt).toBe(0);
    });

    it('should clear pending retry timeout', () => {
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;
      fixture.detectChanges();

      // Simulate a pending retry
      (app as any).retryTimeoutId = setTimeout(() => {}, 5000);

      app.retryConnection();

      expect((app as any).retryTimeoutId).toBeNull();
    });

    it('should call blob bootstrap service', async () => {
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;
      fixture.detectChanges();

      app.retryConnection();
      await fixture.whenStable();

      expect(mockBlobBootstrap.startBootstrap).toHaveBeenCalled();
    });

    it('should attempt holochain connection', async () => {
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;
      fixture.detectChanges();

      mockHolochainClient.connect.calls.reset();

      app.retryConnection();
      await fixture.whenStable();

      expect(mockHolochainClient.connect).toHaveBeenCalled();
    });
  });

  describe('Tauri environment initialization', () => {
    it('should check for Tauri session when in Tauri environment', async () => {
      mockTauriAuth.isTauriEnvironment.and.returnValue(true);
      mockTauriAuth.needsLogin.and.returnValue(false);

      const fixture = TestBed.createComponent(AppComponent);
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockTauriAuth.initialize).toHaveBeenCalled();
    });

    it('should navigate to login when Tauri session needs login', async () => {
      mockTauriAuth.isTauriEnvironment.and.returnValue(true);
      mockTauriAuth.needsLogin.and.returnValue(true);

      const fixture = TestBed.createComponent(AppComponent);
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockRouter.navigate).toHaveBeenCalledWith(['/identity/login']);
    });

    it('should continue initialization when Tauri session exists', async () => {
      mockTauriAuth.isTauriEnvironment.and.returnValue(true);
      mockTauriAuth.needsLogin.and.returnValue(false);

      const fixture = TestBed.createComponent(AppComponent);
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockBlobBootstrap.startBootstrap).toHaveBeenCalled();
    });

    it('should skip Tauri check in non-Tauri environment', async () => {
      mockTauriAuth.isTauriEnvironment.and.returnValue(false);

      const fixture = TestBed.createComponent(AppComponent);
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockTauriAuth.initialize).not.toHaveBeenCalled();
      expect(mockBlobBootstrap.startBootstrap).toHaveBeenCalled();
    });
  });

  describe('Holochain connection initialization', () => {
    it('should start blob bootstrap on initialization', async () => {
      const fixture = TestBed.createComponent(AppComponent);
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockBlobBootstrap.startBootstrap).toHaveBeenCalled();
    });

    it('should connect to Holochain when config is available', async () => {
      const fixture = TestBed.createComponent(AppComponent);
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockHolochainClient.connect).toHaveBeenCalled();
    });

    it('should test holochain content availability after connection', async () => {
      const fixture = TestBed.createComponent(AppComponent);
      fixture.detectChanges();
      await fixture.whenStable();

      expect(mockHolochainContent.testAvailability).toHaveBeenCalled();
    });

    it('should handle connection failure gracefully', async () => {
      mockHolochainClient.connect.and.returnValue(Promise.reject(new Error('Connection failed')));

      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      // Test attemptConnection directly without triggering infinite retry
      await (app as any).attemptConnection();

      // Should have incremented attempt counter
      expect((app as any).connectionAttempt).toBe(1);

      // Clean up scheduled timeout
      app.ngOnDestroy();
    });

    it('should handle content availability test failure', async () => {
      mockHolochainContent.testAvailability.and.returnValue(
        Promise.reject(new Error('Zome unavailable'))
      );

      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      // Call attemptConnection directly to test error handling
      await (app as any).attemptConnection();

      // Should still increment counter and schedule retry
      expect((app as any).connectionAttempt).toBe(1);

      // Clean up
      app.ngOnDestroy();
    });

    it('should reset connection attempt counter on successful connection', async () => {
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      // Simulate previous failed attempts
      (app as any).connectionAttempt = 2;

      fixture.detectChanges();
      await fixture.whenStable();

      expect((app as any).connectionAttempt).toBe(0);
    });
  });

  describe('Connection retry logic', () => {
    it('should increment attempt counter on connection failure', async () => {
      mockHolochainClient.connect.and.returnValue(Promise.reject(new Error('Connection failed')));

      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      // Don't call detectChanges - manually test attemptConnection
      (app as any).connectionAttempt = 0;

      // Manually call attemptConnection to test retry logic
      await (app as any).attemptConnection();

      // After first failed attempt, counter should increment
      expect((app as any).connectionAttempt).toBe(1);
    });

    it('should schedule retry timeout on connection failure', async () => {
      mockHolochainClient.connect.and.returnValue(Promise.reject(new Error('Connection failed')));

      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      // Manually call attemptConnection
      await (app as any).attemptConnection();

      // Retry timeout should be scheduled
      expect((app as any).retryTimeoutId).not.toBeNull();

      // Clean up
      app.ngOnDestroy();
    });

    it('should not schedule retry if component is destroyed', async () => {
      mockHolochainClient.connect.and.returnValue(Promise.reject(new Error('Connection failed')));

      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      // Mark as destroyed first
      (app as any).isDestroyed = true;

      await (app as any).attemptConnection();

      // Should not schedule retry when destroyed
      expect((app as any).retryTimeoutId).toBeNull();
    });

    it('should handle max retry attempts gracefully', async () => {
      mockHolochainClient.connect.and.returnValue(Promise.reject(new Error('Connection failed')));

      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      // Simulate being at max attempts (5)
      (app as any).connectionAttempt = 4;

      // Manually call attemptConnection - this will be attempt 5
      await (app as any).attemptConnection();

      // At max attempts, should not schedule retry
      // Counter should reset after max attempts
      expect((app as any).connectionAttempt).toBe(0);
    });
  });

  describe('isRootLandingPage URL parsing', () => {
    it('should handle URLs with query parameters', () => {
      mockRouter.url = '/?theme=dark';
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      app.ngOnInit();

      expect(app.showFloatingToggle).toBe(true);
    });

    it('should handle URLs with fragments', () => {
      mockRouter.url = '/#section1';
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      app.ngOnInit();

      expect(app.showFloatingToggle).toBe(true);
    });

    it('should handle URLs with both query params and fragments', () => {
      mockRouter.url = '/?tab=intro#top';
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      app.ngOnInit();

      expect(app.showFloatingToggle).toBe(true);
    });

    it('should handle empty string as root page', () => {
      mockRouter.url = '';
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      app.ngOnInit();

      expect(app.showFloatingToggle).toBe(true);
    });

    it('should not treat /lamad with query params as root', () => {
      mockRouter.url = '/lamad?filter=new';
      const fixture = TestBed.createComponent(AppComponent);
      const app = fixture.componentInstance;

      app.ngOnInit();

      expect(app.showFloatingToggle).toBe(false);
    });
  });
});
