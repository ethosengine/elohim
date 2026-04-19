/**
 * NetworkHealthTabComponent Tests
 *
 * Coverage focus:
 * - Component creation
 * - Renders the network-posture-card testid
 * - Shows live metrics when service returns posture
 * - Shows unavailable state when service returns null
 * - data-testids present for all expected metrics
 * - formatPressure and pressureClass helpers
 */

import { ComponentFixture, TestBed } from '@angular/core/testing';
import { NO_ERRORS_SCHEMA } from '@angular/core';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { of } from 'rxjs';

import { NetworkHealthTabComponent } from './network-health-tab.component';
import { NetworkPostureService, type NetworkPostureView } from '../../services/network-posture.service';

const MOCK_POSTURE: NetworkPostureView = {
  totalPeers: 50,
  activePeers: 40,
  stalePeers: 10,
  alwaysOnPeers: 15,
  householdsReciprocating: 8,
  computeAvailable: true,
  storagePressure: 0.45,
  computedAt: '2026-04-19T10:00:00.000Z',
};

describe('NetworkHealthTabComponent', () => {
  let component: NetworkHealthTabComponent;
  let fixture: ComponentFixture<NetworkHealthTabComponent>;
  let networkPostureMock: { get: () => ReturnType<NetworkPostureService['get']> };

  function buildFixture(): Promise<void> {
    return TestBed.configureTestingModule({
      imports: [NetworkHealthTabComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: NetworkPostureService, useValue: networkPostureMock },
      ],
      schemas: [NO_ERRORS_SCHEMA],
    })
      .compileComponents()
      .then(() => {
        fixture = TestBed.createComponent(NetworkHealthTabComponent);
        component = fixture.componentInstance;
      });
  }

  // =========================================================================
  // Creation
  // =========================================================================

  describe('component creation', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      await buildFixture();
    });

    it('should create', () => {
      expect(component).toBeTruthy();
    });

    it('should render the network-posture-card testid', () => {
      fixture.detectChanges();
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="network-posture-card"]')).toBeTruthy();
    });
  });

  // =========================================================================
  // Live posture state
  // =========================================================================

  describe('when service returns posture data', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      await buildFixture();
      fixture.detectChanges();
    });

    it('should show active peers metric', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-active-peers"]');
      expect(span).toBeTruthy();
      expect(span?.textContent).toContain('40');
      expect(span?.textContent).toContain('50');
    });

    it('should show households-reciprocating metric', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-households-reciprocating"]');
      expect(span).toBeTruthy();
      expect(span?.textContent?.trim()).toBe('8');
    });

    it('should show always-on peers metric', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-always-on"]');
      expect(span).toBeTruthy();
      expect(span?.textContent?.trim()).toBe('15');
    });

    it('should show compute-available metric as Yes', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-compute"]');
      expect(span).toBeTruthy();
      expect(span?.textContent).toContain('Yes');
    });

    it('should show storage pressure metric', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-storage-pressure"]');
      expect(span).toBeTruthy();
      expect(span?.textContent).toContain('45%');
    });

    it('should NOT show the unavailable message', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).not.toContain('Network posture unavailable');
    });
  });

  // =========================================================================
  // Unavailable state (null from service)
  // =========================================================================

  describe('when service returns null', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(null) };
      await buildFixture();
      fixture.detectChanges();
    });

    it('should show the unavailable message', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('Network posture unavailable');
    });

    it('should mention the F2 endpoint', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.textContent).toContain('F2');
    });

    it('should NOT show peer metric testids', () => {
      const el: HTMLElement = fixture.nativeElement;
      expect(el.querySelector('[data-testid="posture-active-peers"]')).toBeNull();
    });
  });

  // =========================================================================
  // compute-available = false
  // =========================================================================

  describe('when compute is unavailable', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of({ ...MOCK_POSTURE, computeAvailable: false }) };
      await buildFixture();
      fixture.detectChanges();
    });

    it('should show compute-available metric as No', () => {
      const el: HTMLElement = fixture.nativeElement;
      const span = el.querySelector('[data-testid="posture-compute"]');
      expect(span?.textContent).toContain('No');
    });
  });

  // =========================================================================
  // Helper methods
  // =========================================================================

  describe('formatPressure()', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      await buildFixture();
    });

    it('should format 0 as 0%', () => {
      expect(component.formatPressure(0)).toBe('0%');
    });

    it('should format 0.5 as 50%', () => {
      expect(component.formatPressure(0.5)).toBe('50%');
    });

    it('should format 1 as 100%', () => {
      expect(component.formatPressure(1)).toBe('100%');
    });

    it('should round fractional values', () => {
      expect(component.formatPressure(0.456)).toBe('46%');
    });
  });

  describe('pressureClass()', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      await buildFixture();
    });

    it('should return pressure-ok for low pressure', () => {
      expect(component.pressureClass(0.3)).toBe('pressure-ok');
    });

    it('should return pressure-warning for medium pressure', () => {
      expect(component.pressureClass(0.7)).toBe('pressure-warning');
    });

    it('should return pressure-critical for high pressure', () => {
      expect(component.pressureClass(0.9)).toBe('pressure-critical');
    });

    it('should treat 0.6 boundary as pressure-warning', () => {
      expect(component.pressureClass(0.6)).toBe('pressure-warning');
    });

    it('should treat 0.85 boundary as pressure-critical', () => {
      expect(component.pressureClass(0.85)).toBe('pressure-critical');
    });
  });

  // =========================================================================
  // Lifecycle
  // =========================================================================

  describe('lifecycle', () => {
    beforeEach(async () => {
      networkPostureMock = { get: () => of(MOCK_POSTURE) };
      await buildFixture();
    });

    it('should handle ngOnDestroy without error', () => {
      fixture.detectChanges();
      expect(() => component.ngOnDestroy()).not.toThrow();
    });
  });
});
