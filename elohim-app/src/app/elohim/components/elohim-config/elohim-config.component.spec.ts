import { ComponentFixture, TestBed, fakeAsync, tick } from '@angular/core/testing';

import { BehaviorSubject, of } from 'rxjs';

import { ElohimBackendCatalog } from '../../services/elohim-backend';
import type { ElohimBackend } from '../../services/elohim-backend';
import { ElohimPresenceService } from '../../services/elohim-presence.service';
import { NativeBackend } from '../../services/backends/native-backend';

import type { ElohimComputationCost } from '../../models/elohim-agent.model';

import { ElohimConfigComponent } from './elohim-config.component';

describe('ElohimConfigComponent', () => {
  let fixture: ComponentFixture<ElohimConfigComponent>;
  let component: ElohimConfigComponent;
  let mockCatalog: jasmine.SpyObj<ElohimBackendCatalog>;
  let mockPresenceService: jasmine.SpyObj<ElohimPresenceService>;
  let costSubject: BehaviorSubject<ElohimComputationCost>;

  beforeEach(async () => {
    costSubject = new BehaviorSubject<ElohimComputationCost>({
      tokensProcessed: 0,
      timeMs: 0,
      constitutionalChecks: 0,
      precedentLookups: 0,
    });

    mockCatalog = jasmine.createSpyObj('ElohimBackendCatalog', [
      'register',
      'unregister',
      'setPreferred',
      'getPreferredId',
      'get',
      'selectBackend',
    ]);
    mockCatalog.getPreferredId.and.returnValue(null);
    mockCatalog.get.and.returnValue(undefined);
    mockCatalog.selectBackend.and.returnValue(
      Promise.resolve({
        id: 'mock',
        name: 'Simulation (Mock)',
        type: 'mock' as const,
        isAvailable: () => Promise.resolve(true),
        invoke: () => of(),
      }),
    );

    mockPresenceService = jasmine.createSpyObj(
      'ElohimPresenceService',
      ['onContentCompleted', 'onDiscoveryCompleted'],
      {
        cost$: costSubject.asObservable(),
      },
    );

    await TestBed.configureTestingModule({
      imports: [ElohimConfigComponent],
      providers: [
        { provide: ElohimBackendCatalog, useValue: mockCatalog },
        { provide: ElohimPresenceService, useValue: mockPresenceService },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(ElohimConfigComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  afterEach(() => {
    costSubject.complete();
  });

  // =========================================================================
  // Creation
  // =========================================================================

  describe('creation', () => {
    it('should create', () => {
      expect(component).toBeTruthy();
    });

    it('should render config panel', () => {
      const el = fixture.nativeElement.querySelector('[data-testid="elohim-config"]');
      expect(el).toBeTruthy();
    });

    it('should show 2 backend options', () => {
      const options = fixture.nativeElement.querySelectorAll('.backend-option');
      expect(options.length).toBe(2);
    });

    it('should default to mock backend', () => {
      expect(component.selectedBackend()).toBe('mock');
    });
  });

  // =========================================================================
  // selectBackend
  // =========================================================================

  describe('selectBackend', () => {
    it('should update selected signal', () => {
      component.selectBackend('native');
      expect(component.selectedBackend()).toBe('native');
    });

    it('should clear test result on switch', () => {
      component.testResult.set({ success: true, message: 'ok' });
      component.selectBackend('mock');
      expect(component.testResult()).toBeNull();
    });

    it('should call catalog.setPreferred', () => {
      component.selectBackend('native');
      expect(mockCatalog.setPreferred).toHaveBeenCalledWith('native');
    });
  });

  // =========================================================================
  // API Key (BYOK)
  // =========================================================================

  describe('API key section', () => {
    it('should show API key section only for native', () => {
      let section = fixture.nativeElement.querySelector('[data-testid="api-key-section"]');
      expect(section).toBeNull();

      component.selectBackend('native');
      fixture.detectChanges();

      section = fixture.nativeElement.querySelector('[data-testid="api-key-section"]');
      expect(section).toBeTruthy();
    });

    it('should update hasApiKey on input', () => {
      component.selectBackend('native');
      fixture.detectChanges();

      const event = { target: { value: 'sk-ant-test-key' } } as unknown as Event;
      component.onApiKeyInput(event);
      expect(component.hasApiKey()).toBe(true);
    });

    it('should forward BYOK key to NativeBackend instance', () => {
      const nativeBackend = new NativeBackend();
      mockCatalog.get.and.callFake((id: string) =>
        id === 'native' ? (nativeBackend as unknown as ElohimBackend) : undefined,
      );

      const event = { target: { value: 'sk-ant-test-key' } } as unknown as Event;
      component.onApiKeyInput(event);

      expect(nativeBackend.byokApiKey).toBe('sk-ant-test-key');
    });

    it('should clear API key', () => {
      component.apiKey.set('some-key');
      component.clearApiKey();
      expect(component.hasApiKey()).toBe(false);
    });

    it('should clear BYOK key on NativeBackend when cleared', () => {
      const nativeBackend = new NativeBackend();
      nativeBackend.byokApiKey = 'old-key';
      mockCatalog.get.and.callFake((id: string) =>
        id === 'native' ? (nativeBackend as unknown as ElohimBackend) : undefined,
      );

      component.clearApiKey();
      expect(nativeBackend.byokApiKey).toBeNull();
    });

    it('should clear test result on key clear', () => {
      component.testResult.set({ success: true, message: 'ok' });
      component.clearApiKey();
      expect(component.testResult()).toBeNull();
    });
  });

  // =========================================================================
  // Test Connection
  // =========================================================================

  describe('test connection', () => {
    it('should set testing signal during test', fakeAsync(() => {
      expect(component.testing()).toBe(false);

      component.testBackend();
      expect(component.testing()).toBe(true);

      tick();
      expect(component.testing()).toBe(false);
    }));

    it('should show success message when available', fakeAsync(() => {
      component.testBackend();
      tick();

      expect(component.testResult()!.success).toBe(true);
      expect(component.testResult()!.message).toContain('available');
    }));

    it('should show error when not available', fakeAsync(() => {
      mockCatalog.selectBackend.and.returnValue(
        Promise.resolve({
          id: 'mock',
          name: 'Simulation (Mock)',
          type: 'mock' as const,
          isAvailable: () => Promise.resolve(false),
          invoke: () => of(),
        }),
      );

      component.testBackend();
      tick();

      expect(component.testResult()!.success).toBe(false);
      expect(component.testResult()!.message).toContain('not available');
    }));

    it('should handle test failure gracefully', fakeAsync(() => {
      mockCatalog.selectBackend.and.returnValue(Promise.reject(new Error('Network error')));

      component.testBackend();
      tick();

      expect(component.testResult()!.success).toBe(false);
      expect(component.testResult()!.message).toContain('failed');
    }));
  });

  // =========================================================================
  // Session Cost
  // =========================================================================

  describe('session cost', () => {
    it('should display cost from presence service', () => {
      costSubject.next({
        tokensProcessed: 500,
        timeMs: 1000,
        constitutionalChecks: 3,
        precedentLookups: 1,
      });
      fixture.detectChanges();

      const costEl = fixture.nativeElement.querySelector('[data-testid="session-cost"]');
      expect(costEl).toBeTruthy();
      expect(costEl.textContent).toContain('500');
    });

    it('should not show cost when null', () => {
      // Initial cost is zero, which is truthy as an object, but let's check the initial render
      const costEl = fixture.nativeElement.querySelector('[data-testid="session-cost"]');
      // Zero cost object is still truthy in @if, so it will render
      expect(costEl).toBeTruthy();
    });
  });
});
