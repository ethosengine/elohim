import { TestBed } from '@angular/core/testing';

import { CustodianMetricsReporterService } from './custodian-metrics-reporter.service';
import { PerformanceMetricsService } from './performance-metrics.service';
import { ShefaService } from './shefa.service';

describe('CustodianMetricsReporterService', () => {
  let service: CustodianMetricsReporterService;
  let metricsMock: jasmine.SpyObj<PerformanceMetricsService>;
  let shefaMock: jasmine.SpyObj<ShefaService>;

  beforeEach(() => {
    const metricsSpy = jasmine.createSpyObj('PerformanceMetricsService', [
      'getMetricsForReport',
    ]);
    const shefaSpy = jasmine.createSpyObj('ShefaService', ['reportMetrics']);

    TestBed.configureTestingModule({
      providers: [
        CustodianMetricsReporterService,
        { provide: PerformanceMetricsService, useValue: metricsSpy },
        { provide: ShefaService, useValue: shefaSpy },
      ],
    });

    service = TestBed.inject(CustodianMetricsReporterService);
    metricsMock = TestBed.inject(PerformanceMetricsService) as jasmine.SpyObj<PerformanceMetricsService>;
    shefaMock = TestBed.inject(ShefaService) as jasmine.SpyObj<ShefaService>;
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  it('should have enableReporting method', () => {
    expect(service.enableReporting).toBeDefined();
    expect(typeof service.enableReporting).toBe('function');
  });

  it('should have disableReporting method', () => {
    expect(service.disableReporting).toBeDefined();
    expect(typeof service.disableReporting).toBe('function');
  });

  it('should have isReportingEnabled method', () => {
    expect(service.isReportingEnabled).toBeDefined();
    expect(typeof service.isReportingEnabled).toBe('function');
  });

  it('should have reportMetrics method', () => {
    expect(service.reportMetrics).toBeDefined();
    expect(typeof service.reportMetrics).toBe('function');
  });

  it('should have getStatistics method', () => {
    expect(service.getStatistics).toBeDefined();
    expect(typeof service.getStatistics).toBe('function');
  });

  it('should have reportingStats readonly signal', () => {
    expect(service.reportingStats).toBeDefined();
  });

  it('should have successRate computed signal', () => {
    expect(service.successRate).toBeDefined();
  });
});
