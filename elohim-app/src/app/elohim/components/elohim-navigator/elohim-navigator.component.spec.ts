import { ComponentFixture, TestBed } from '@angular/core/testing';
import { Router, NavigationEnd, ActivatedRoute } from '@angular/router';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { signal } from '@angular/core';
import { Subject, of } from 'rxjs';

import { ElohimNavigatorComponent } from './elohim-navigator.component';
import { SessionHumanService } from '@app/imagodei/services/session-human.service';
import { HolochainClientService } from '@app/elohim/services/holochain-client.service';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { AuthService } from '@app/imagodei/services/auth.service';
import { RunningContextService } from '@app/doorway/services/running-context.service';

describe('ElohimNavigatorComponent', () => {
  let component: ElohimNavigatorComponent;
  let fixture: ComponentFixture<ElohimNavigatorComponent>;
  let mockSessionHumanService: jasmine.SpyObj<SessionHumanService>;
  let mockRouter: jasmine.SpyObj<Router>;
  let mockHolochainService: jasmine.SpyObj<HolochainClientService>;
  let mockIdentityService: jasmine.SpyObj<IdentityService>;
  let mockAuthService: jasmine.SpyObj<AuthService>;
  let mockRunningContext: jasmine.SpyObj<RunningContextService>;
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

    mockAuthService = jasmine.createSpyObj('AuthService', ['isAuthenticated']);
    mockAuthService.isAuthenticated.and.returnValue(false);

    mockRunningContext = jasmine.createSpyObj('RunningContextService', [
      'startPeriodicDetection',
      'stopPeriodicDetection',
      'hasDoorwayCapableNode',
    ]);
    mockRunningContext.hasDoorwayCapableNode.and.returnValue(false);

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
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ElohimNavigatorComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
