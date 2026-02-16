import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, NavigationEnd, ActivatedRoute } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { signal } from '@angular/core';

import { Subject, of } from 'rxjs';

import { ElohimNavigatorComponent } from './elohim-navigator.component';
import { BannerService } from '@app/elohim/services/banner.service';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { AuthService } from '@app/imagodei/services/auth.service';
import { RunningContextService } from '@app/doorway/services/running-context.service';
import { UpgradeBannerProvider } from '@app/imagodei/services/providers/upgrade-banner.provider';

describe('ElohimNavigatorComponent', () => {
  let component: ElohimNavigatorComponent;
  let fixture: ComponentFixture<ElohimNavigatorComponent>;
  let mockSessionHumanService: jasmine.SpyObj<SessionHumanService>;
  let mockRouter: jasmine.SpyObj<Router>;
  let mockHolochainService: jasmine.SpyObj<HolochainClientService>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockAuthService: jasmine.SpyObj<AuthService>;
  let mockRunningContext: jasmine.SpyObj<RunningContextService>;
  let mockBannerService: jasmine.SpyObj<BannerService>;
  let mockUpgradeBannerProvider: jasmine.SpyObj<UpgradeBannerProvider>;
  let routerEventsSubject: Subject<unknown>;

  beforeEach(async () => {
    routerEventsSubject = new Subject();

    mockSessionHumanService = jasmine.createSpyObj('SessionHumanService', [], {
      session$: new Subject(),
      upgradePrompts$: new Subject(),
    });

    mockRouter = jasmine.createSpyObj('Router', ['navigate', 'createUrlTree', 'serializeUrl'], {
      events: routerEventsSubject.asObservable(),
      url: '/lamad',
    });
    mockRouter.createUrlTree.and.returnValue({
      root: {},
      queryParams: {},
      fragment: null,
      queryParamMap: new Map(),
    } as any);
    mockRouter.serializeUrl.and.returnValue('/lamad');

    mockHolochainService = jasmine.createSpyObj(
      'HolochainClientService',
      ['disconnect', 'connect', 'getDisplayInfo'],
      {
        state: signal('disconnected'),
      }
    );
    mockHolochainService.getDisplayInfo.and.returnValue({
      state: 'disconnected',
      mode: 'doorway',
      adminUrl: 'ws://localhost:8888',
      appUrl: 'ws://localhost:8888',
      agentPubKey: null,
      cellId: null,
      appId: 'elohim',
      dnaHash: null,
      connectedAt: null,
      hasStoredCredentials: false,
      networkSeed: null,
      error: null,
    });

    mockIdentityService = jasmine.createSpyObj('IdentityService', ['logout'], {
      mode: signal('session'),
      displayName: signal('Test User'),
    });

    mockAuthService = jasmine.createSpyObj('AuthService', ['isAuthenticated'], {
      identifier: signal(null),
      doorwayUrl: signal(null),
    });
    mockAuthService.isAuthenticated.and.returnValue(false);

    mockRunningContext = jasmine.createSpyObj('RunningContextService', [
      'startPeriodicDetection',
      'stopPeriodicDetection',
      'hasDoorwayCapableNode',
    ]);
    mockRunningContext.hasDoorwayCapableNode.and.returnValue(false);

    mockBannerService = jasmine.createSpyObj('BannerService', [
      'registerProvider',
      'unregisterProvider',
      'noticesForContext$',
      'dismissNotice',
      'handleAction',
    ]);
    mockBannerService.noticesForContext$.and.returnValue(of([]));

    mockUpgradeBannerProvider = jasmine.createSpyObj('UpgradeBannerProvider', [], {
      upgradeModalRequested$: new Subject<void>(),
    });

    await TestBed.configureTestingModule({
      imports: [ElohimNavigatorComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: SessionHumanService, useValue: mockSessionHumanService },
        { provide: Router, useValue: mockRouter },
        { provide: ActivatedRoute, useValue: { queryParams: of({}) } },
        { provide: HolochainClientService, useValue: mockHolochainService },
        { provide: IdentityService, useValue: mockIdentityService },
        { provide: AuthService, useValue: mockAuthService },
        { provide: RunningContextService, useValue: mockRunningContext },
        { provide: BannerService, useValue: mockBannerService },
        { provide: UpgradeBannerProvider, useValue: mockUpgradeBannerProvider },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ElohimNavigatorComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should subscribe to banner notices for context', () => {
    expect(mockBannerService.noticesForContext$).toHaveBeenCalledWith('lamad');
  });

  it('should delegate banner dismiss to BannerService', () => {
    const notice = {
      id: 'test-notice',
      providerId: 'test',
      severity: 'info' as const,
      priority: 'info' as const,
      contexts: ['global' as const],
      title: 'Test',
      dismissible: true,
      createdAt: new Date(),
    };

    // Simulate banner notices being set
    (component as any).bannerNotices = [notice];

    component.onBannerDismissed({ id: 'test-notice', severity: 'info', title: 'Test' });

    expect(mockBannerService.dismissNotice).toHaveBeenCalledWith(notice);
  });

  it('should delegate banner action to BannerService', () => {
    const notice = {
      id: 'test-notice',
      providerId: 'test',
      severity: 'info' as const,
      priority: 'info' as const,
      contexts: ['global' as const],
      title: 'Test',
      dismissible: true,
      createdAt: new Date(),
    };

    (component as any).bannerNotices = [notice];

    component.onBannerAction({
      alert: { id: 'test-notice', severity: 'info', title: 'Test' },
      action: { id: 'learn-more', label: 'Learn More' },
    });

    expect(mockBannerService.handleAction).toHaveBeenCalledWith(notice, 'learn-more');
  });

  it('should open upgrade modal when provider emits', () => {
    expect(component.showUpgradeModal).toBe(false);

    (mockUpgradeBannerProvider.upgradeModalRequested$ as Subject<void>).next();

    expect(component.showUpgradeModal).toBe(true);
  });
});
