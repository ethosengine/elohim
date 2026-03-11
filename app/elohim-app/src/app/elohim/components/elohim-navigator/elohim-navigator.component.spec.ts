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
import { vi } from 'vitest';

describe('ElohimNavigatorComponent', () => {
  let component: ElohimNavigatorComponent;
  let fixture: ComponentFixture<ElohimNavigatorComponent>;
  let mockSessionHumanService: any;
  let mockRouter: any;
  let mockHolochainService: any;
  let mockIdentityService: any;
  let mockAuthService: any;
  let mockRunningContext: any;
  let mockBannerService: any;
  let mockUpgradeBannerProvider: any;
  let routerEventsSubject: Subject<unknown>;

  beforeEach(async () => {
    routerEventsSubject = new Subject();

    mockSessionHumanService = {
      session$: new Subject(),
      upgradePrompts$: new Subject(),
    };

    mockRouter = {
      navigate: vi.fn(),
      createUrlTree: vi.fn(),
      serializeUrl: vi.fn(),
      events: routerEventsSubject.asObservable(),
      url: '/lamad',
    };
    mockRouter.createUrlTree.mockReturnValue({
      root: {},
      queryParams: {},
      fragment: null,
      queryParamMap: new Map(),
    } as any);
    mockRouter.serializeUrl.mockReturnValue('/lamad');

    mockHolochainService = {
      disconnect: vi.fn(),
      connect: vi.fn(),
      getDisplayInfo: vi.fn(),
    };
    mockHolochainService.getDisplayInfo.mockReturnValue({
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

    mockIdentityService = {
      logout: vi.fn(),
      mode: signal<'anonymous' | 'hosted' | 'steward'>('anonymous'),
      displayName: signal<string | null>(null),
      humanId: signal<string | null>(null),
    };

    mockAuthService = {
      isAuthenticated: vi.fn(),
      identifier: signal<string | null>(null),
      doorwayUrl: signal<string | null>(null),
    };
    mockAuthService.isAuthenticated.mockReturnValue(false);

    mockRunningContext = {
      startPeriodicDetection: vi.fn(),
      stopPeriodicDetection: vi.fn(),
      hasDoorwayCapableNode: vi.fn(),
    };
    mockRunningContext.hasDoorwayCapableNode.mockReturnValue(false);

    mockUpgradeBannerProvider = {
      upgradeModalRequested$: new Subject<void>(),
      providerId: 'upgrade-banner',
      notices$: of([]),
      dismissNotice: vi.fn(),
      handleAction: vi.fn(),
    };

    mockBannerService = {
      registerProvider: vi.fn(),
      unregisterProvider: vi.fn(),
      noticesForContext$: vi.fn(),
      dismissNotice: vi.fn(),
      handleAction: vi.fn(),
    };
    mockBannerService.noticesForContext$.mockReturnValue(of([]));

    await TestBed.configureTestingModule({
      imports: [ElohimNavigatorComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: Router, useValue: mockRouter },
        { provide: ActivatedRoute, useValue: { snapshot: {}, params: of({}) } },
        { provide: SessionHumanService, useValue: mockSessionHumanService },
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
