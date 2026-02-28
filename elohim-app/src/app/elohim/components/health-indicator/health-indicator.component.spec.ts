import { vi } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';

import { HealthIndicatorComponent } from './health-indicator.component';
import { HealthCheckService, type HealthStatus } from '../../services/health-check.service';

describe('HealthIndicatorComponent', () => {
  let component: HealthIndicatorComponent;
  let fixture: ComponentFixture<HealthIndicatorComponent>;
  let mockHealthService: any;

  const mockHealthStatus: HealthStatus = {
    status: 'healthy',
    summary: 'All systems operational',
    lastChecked: new Date().toISOString(),
    isChecking: false,
    checks: {
      holochain: {
        name: 'holochain',
        status: 'healthy',
        message: 'Connected',
        lastChecked: new Date().toISOString(),
      },
      indexedDb: {
        name: 'indexedDb',
        status: 'healthy',
        message: 'Cache operational',
        lastChecked: new Date().toISOString(),
      },
      blobCache: {
        name: 'blobCache',
        status: 'healthy',
        message: 'Blob cache ready',
        lastChecked: new Date().toISOString(),
      },
      network: {
        name: 'network',
        status: 'healthy',
        message: 'Online',
        lastChecked: new Date().toISOString(),
      },
    },
  };

  beforeEach(async () => {
    mockHealthService = { refresh: vi.fn(), getQuickStatus: vi.fn(), status: signal(mockHealthStatus),
        isChecking: signal(false), };
    mockHealthService.refresh.mockReturnValue(Promise.resolve(mockHealthStatus));
    mockHealthService.getQuickStatus.mockReturnValue({
      icon: '✓',
      label: 'All systems operational',
      color: '#34a853',
    });

    await TestBed.configureTestingModule({
      imports: [HealthIndicatorComponent],
      providers: [{ provide: HealthCheckService, useValue: mockHealthService }],
    }).compileComponents();

    fixture = TestBed.createComponent(HealthIndicatorComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
