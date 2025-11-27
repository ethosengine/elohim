import { TestBed } from '@angular/core/testing';
import { of } from 'rxjs';
import { TrustBadgeService } from './trust-badge.service';
import { DataLoaderService } from './data-loader.service';
import { TrustLevel, TrustBadge } from '../models/trust-badge.model';

describe('TrustBadgeService', () => {
  let service: TrustBadgeService;
  let dataLoaderMock: jasmine.SpyObj<DataLoaderService>;

  beforeEach(() => {
    dataLoaderMock = jasmine.createSpyObj('DataLoaderService', [
      'getContent',
      'getAttestations'
    ]);

    dataLoaderMock.getContent.and.returnValue(of({
      id: 'node-1',
      title: 'Test',
      trustScore: 0.8,
      activeAttestationIds: ['att-1']
    }));
    dataLoaderMock.getAttestations.and.returnValue(of([]));

    TestBed.configureTestingModule({
      providers: [
        TrustBadgeService,
        { provide: DataLoaderService, useValue: dataLoaderMock }
      ]
    });
    service = TestBed.inject(TrustBadgeService);
  });

  it('should be created', () => {
    expect(service).toBeTruthy();
  });

  describe('getTrustBadge', () => {
    it('should return trust badge for content', (done) => {
      service.getTrustBadge('node-1').subscribe(badge => {
        expect(badge).toBeDefined();
        expect(badge.level).toBeDefined();
        expect(badge.score).toBeDefined();
        done();
      });
    });

    it('should calculate correct trust level', (done) => {
      service.getTrustBadge('node-1').subscribe(badge => {
        expect(badge.score).toBe(0.8);
        expect(badge.level).toBe('high');
        done();
      });
    });
  });

  describe('getTrustBadgeForScore', () => {
    it('should return unknown for 0 score', () => {
      const badge = service.getTrustBadgeForScore(0);
      expect(badge.level).toBe('unknown');
    });

    it('should return low for scores 0.01-0.39', () => {
      const badge = service.getTrustBadgeForScore(0.3);
      expect(badge.level).toBe('low');
    });

    it('should return medium for scores 0.4-0.69', () => {
      const badge = service.getTrustBadgeForScore(0.5);
      expect(badge.level).toBe('medium');
    });

    it('should return high for scores 0.7-0.89', () => {
      const badge = service.getTrustBadgeForScore(0.8);
      expect(badge.level).toBe('high');
    });

    it('should return verified for scores 0.9+', () => {
      const badge = service.getTrustBadgeForScore(0.95);
      expect(badge.level).toBe('verified');
    });

    it('should include appropriate label', () => {
      const badge = service.getTrustBadgeForScore(0.8);
      expect(badge.label).toBeDefined();
      expect(badge.label.length).toBeGreaterThan(0);
    });

    it('should include color', () => {
      const badge = service.getTrustBadgeForScore(0.8);
      expect(badge.color).toBeDefined();
    });

    it('should include icon', () => {
      const badge = service.getTrustBadgeForScore(0.8);
      expect(badge.icon).toBeDefined();
    });
  });

  describe('getTrustColor', () => {
    it('should return gray for unknown', () => {
      expect(service.getTrustColor('unknown')).toBe('gray');
    });

    it('should return red for low', () => {
      expect(service.getTrustColor('low')).toBe('red');
    });

    it('should return yellow for medium', () => {
      expect(service.getTrustColor('medium')).toBe('yellow');
    });

    it('should return green for high', () => {
      expect(service.getTrustColor('high')).toBe('green');
    });

    it('should return blue for verified', () => {
      expect(service.getTrustColor('verified')).toBe('blue');
    });
  });

  describe('getTrustIcon', () => {
    it('should return appropriate icon for each level', () => {
      const levels: TrustLevel[] = ['unknown', 'low', 'medium', 'high', 'verified'];

      levels.forEach(level => {
        const icon = service.getTrustIcon(level);
        expect(icon).toBeDefined(`Expected icon for ${level}`);
        expect(icon.length).toBeGreaterThan(0);
      });
    });
  });

  describe('getTrustLabel', () => {
    it('should return human-readable label for each level', () => {
      const levels: TrustLevel[] = ['unknown', 'low', 'medium', 'high', 'verified'];

      levels.forEach(level => {
        const label = service.getTrustLabel(level);
        expect(label).toBeDefined(`Expected label for ${level}`);
        expect(label.length).toBeGreaterThan(0);
      });
    });
  });

  describe('formatTrustScore', () => {
    it('should format score as percentage', () => {
      expect(service.formatTrustScore(0.85)).toBe('85%');
      expect(service.formatTrustScore(0.5)).toBe('50%');
      expect(service.formatTrustScore(1.0)).toBe('100%');
    });

    it('should handle edge cases', () => {
      expect(service.formatTrustScore(0)).toBe('0%');
      expect(service.formatTrustScore(0.999)).toBe('100%');
    });
  });
});
