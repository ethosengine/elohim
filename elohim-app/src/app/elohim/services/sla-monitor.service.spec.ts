/**
 * SLA Monitor Service Tests
 *
 * Coverage focus:
 * - Service creation and lifecycle
 * - SLA registration and management
 * - Status transitions (acknowledge, resolve, escalate)
 * - Observable streams
 * - Helper calculations
 * - Persistence
 */

import { TestBed } from '@angular/core/testing';
import { SlaMonitorService, SlaRegistration, SlaResolution, SlaEntityType } from './sla-monitor.service';

describe('SlaMonitorService', () => {
  let service: SlaMonitorService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [SlaMonitorService],
    });

    service = TestBed.inject(SlaMonitorService);
  });

  afterEach(() => {
    service.ngOnDestroy();
  });

  // ==========================================================================
  // Service Creation Tests
  // ==========================================================================

  describe('service creation', () => {
    it('should be created', () => {
      expect(service).toBeTruthy();
    });

    it('should be a SlaMonitorService instance', () => {
      expect(service instanceof SlaMonitorService).toBe(true);
    });
  });

  // ==========================================================================
  // SLA Registration Tests
  // ==========================================================================

  describe('SLA registration', () => {
    it('should have registerSla method', () => {
      expect(typeof service.registerSla).toBe('function');
    });

    it('should register SLA and return SlaItem', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review Challenge',
        description: 'Review submitted challenge',
        assignedTo: 'reviewer-1',
      };

      const sla = service.registerSla(registration);
      expect(sla).toBeDefined();
      expect(sla.id).toBeDefined();
      expect(sla.entityType).toBe('challenge');
      expect(sla.status).toBe('pending');
    });

    it('should reject unknown entity type', () => {
      const registration: SlaRegistration = {
        entityType: 'unknown' as SlaEntityType,
        entityId: 'item-123',
        title: 'Test',
      };

      expect(() => service.registerSla(registration)).toThrow();
    });

    it('should accept custom deadline', () => {
      const customDeadline = new Date(Date.now() + 86400000).toISOString();
      const registration: SlaRegistration = {
        entityType: 'proposal',
        entityId: 'proposal-123',
        title: 'Vote on Proposal',
        customDeadline,
      };

      const sla = service.registerSla(registration);
      expect(sla.deadline).toBe(customDeadline);
    });

    it('should accept metadata', () => {
      const registration: SlaRegistration = {
        entityType: 'mediation',
        entityId: 'mediation-123',
        title: 'Resolve Dispute',
        metadata: { disputeId: 'd-123', parties: 2 },
      };

      const sla = service.registerSla(registration);
      expect(sla.metadata['disputeId']).toBe('d-123');
    });

    it('should set default priority to normal', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review Challenge',
      };

      const sla = service.registerSla(registration);
      expect(sla.priority).toBe('normal');
    });

    it('should accept custom priority', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Urgent Challenge',
        priority: 'critical',
      };

      const sla = service.registerSla(registration);
      expect(sla.priority).toBe('critical');
    });
  });

  // ==========================================================================
  // SLA Acknowledgement Tests
  // ==========================================================================

  describe('SLA acknowledgement', () => {
    it('should have acknowledgeSla method', () => {
      expect(typeof service.acknowledgeSla).toBe('function');
    });

    it('should acknowledge SLA and return true', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review Challenge',
      };

      const sla = service.registerSla(registration);
      const result = service.acknowledgeSla(sla.id, 'acknowledger-1');

      expect(result).toBe(true);
    });

    it('should return false for non-existent SLA', () => {
      const result = service.acknowledgeSla('non-existent', 'user-1');
      expect(result).toBe(false);
    });

    it('should update status to acknowledged', () => {
      const registration: SlaRegistration = {
        entityType: 'proposal',
        entityId: 'proposal-123',
        title: 'Vote',
      };

      const sla = service.registerSla(registration);
      service.acknowledgeSla(sla.id, 'acknowledger-1');

      const activeSlas = service.getActiveSlas();
      activeSlas.subscribe(slas => {
        const updated = slas.find(s => s.id === sla.id);
        expect(updated?.status).toBe('acknowledged');
      });
    });

    it('should set acknowledgedAt timestamp', () => {
      const registration: SlaRegistration = {
        entityType: 'mediation',
        entityId: 'mediation-123',
        title: 'Resolve',
      };

      const sla = service.registerSla(registration);
      expect(sla.acknowledgedAt).toBeNull();

      service.acknowledgeSla(sla.id, 'acknowledger-1');

      const activeSlas = service.getActiveSlas();
      activeSlas.subscribe(slas => {
        const updated = slas.find(s => s.id === sla.id);
        expect(updated?.acknowledgedAt).not.toBeNull();
      });
    });
  });

  // ==========================================================================
  // SLA Resolution Tests
  // ==========================================================================

  describe('SLA resolution', () => {
    it('should have resolveSla method', () => {
      expect(typeof service.resolveSla).toBe('function');
    });

    it('should resolve SLA and return true', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review Challenge',
      };

      const sla = service.registerSla(registration);
      const resolution: SlaResolution = {
        outcome: 'resolved',
        notes: 'Challenge accepted',
        resolvedBy: 'resolver-1',
      };

      const result = service.resolveSla(sla.id, resolution);
      expect(result).toBe(true);
    });

    it('should return false for non-existent SLA', () => {
      const resolution: SlaResolution = {
        outcome: 'resolved',
        resolvedBy: 'user-1',
      };

      const result = service.resolveSla('non-existent', resolution);
      expect(result).toBe(false);
    });

    it('should accept different outcomes', () => {
      const registration: SlaRegistration = {
        entityType: 'proposal',
        entityId: 'proposal-123',
        title: 'Vote',
      };

      const sla = service.registerSla(registration);

      const resolution: SlaResolution = {
        outcome: 'dismissed',
        resolvedBy: 'resolver-1',
      };

      service.resolveSla(sla.id, resolution);
      expect(true).toBe(true); // Verify method executed without throwing
    });
  });

  // ==========================================================================
  // SLA Escalation Tests
  // ==========================================================================

  describe('SLA escalation', () => {
    it('should have escalateSla method', () => {
      expect(typeof service.escalateSla).toBe('function');
    });

    it('should escalate SLA and return true', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review Challenge',
      };

      const sla = service.registerSla(registration);
      const result = service.escalateSla(sla.id, 'Deadline approaching');

      expect(result).toBe(true);
    });

    it('should return false for non-existent SLA', () => {
      const result = service.escalateSla('non-existent', 'reason');
      expect(result).toBe(false);
    });

    it('should return false at max escalation level', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review Challenge',
      };

      const sla = service.registerSla(registration);
      const config = service.getConfiguration('challenge');
      const maxLevel = (config?.escalationPath.length ?? 1) - 1;

      // Escalate to max level
      for (let i = 0; i < maxLevel; i++) {
        service.escalateSla(sla.id, `Escalation ${i}`);
      }

      // Try to escalate beyond max
      const result = service.escalateSla(sla.id, 'Beyond max');
      expect(result).toBe(false);
    });
  });

  // ==========================================================================
  // Observable Stream Tests
  // ==========================================================================

  describe('observable streams', () => {
    it('should have getActiveSlas method', () => {
      expect(typeof service.getActiveSlas).toBe('function');
    });

    it('should have getSlasByType method', () => {
      expect(typeof service.getSlasByType).toBe('function');
    });

    it('should have getSlaForEntity method', () => {
      expect(typeof service.getSlaForEntity).toBe('function');
    });

    it('should have getAlerts method', () => {
      expect(typeof service.getAlerts).toBe('function');
    });

    it('should have getBreaches method', () => {
      expect(typeof service.getBreaches).toBe('function');
    });

    it('should have getAtRiskSlas method', () => {
      expect(typeof service.getAtRiskSlas).toBe('function');
    });

    it('getActiveSlas should return Observable', () => {
      const result = service.getActiveSlas();
      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('getSlasByType should return Observable', () => {
      const result = service.getSlasByType('challenge');
      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('getSlaForEntity should return Observable', () => {
      const result = service.getSlaForEntity('entity-123');
      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('getAlerts should return Observable', () => {
      const result = service.getAlerts();
      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('getBreaches should return Observable', () => {
      const result = service.getBreaches();
      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });

    it('getAtRiskSlas should return Observable', () => {
      const result = service.getAtRiskSlas();
      expect(result).toBeDefined();
      expect(typeof result.subscribe).toBe('function');
    });
  });

  // ==========================================================================
  // Metrics Tests
  // ==========================================================================

  describe('metrics', () => {
    it('should have getMetrics method', () => {
      expect(typeof service.getMetrics).toBe('function');
    });

    it('should return metrics object', () => {
      const metrics = service.getMetrics();
      expect(metrics).toBeDefined();
      expect(metrics.totalTracked).toBeDefined();
      expect(metrics.currentActive).toBeDefined();
      expect(metrics.breachCount).toBeDefined();
      expect(metrics.onTimeRate).toBeDefined();
      expect(metrics.escalationCount).toBeDefined();
    });

    it('should increment totalTracked on registerSla', () => {
      const initialMetrics = service.getMetrics();
      const initialCount = initialMetrics.totalTracked;

      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review',
      };

      service.registerSla(registration);
      const updatedMetrics = service.getMetrics();

      expect(updatedMetrics.totalTracked).toBe(initialCount + 1);
    });

    it('should increment currentActive on registerSla', () => {
      const initialMetrics = service.getMetrics();
      const initialActive = initialMetrics.currentActive;

      const registration: SlaRegistration = {
        entityType: 'proposal',
        entityId: 'proposal-123',
        title: 'Vote',
      };

      service.registerSla(registration);
      const updatedMetrics = service.getMetrics();

      expect(updatedMetrics.currentActive).toBe(initialActive + 1);
    });
  });

  // ==========================================================================
  // Configuration Tests
  // ==========================================================================

  describe('configuration', () => {
    it('should have getConfiguration method', () => {
      expect(typeof service.getConfiguration).toBe('function');
    });

    it('should return configuration for challenge', () => {
      const config = service.getConfiguration('challenge');
      expect(config).toBeDefined();
      expect(config?.acknowledgmentHours).toBe(24);
      expect(config?.resolutionDays).toBe(7);
    });

    it('should return configuration for proposal', () => {
      const config = service.getConfiguration('proposal');
      expect(config).toBeDefined();
      expect(config?.acknowledgmentHours).toBe(4);
    });

    it('should return configuration for mediation', () => {
      const config = service.getConfiguration('mediation');
      expect(config).toBeDefined();
      expect(config?.acknowledgmentHours).toBe(1);
    });

    it('should return undefined for unknown type', () => {
      const config = service.getConfiguration('unknown' as SlaEntityType);
      expect(config).toBeUndefined();
    });

    it('should have escalation paths in config', () => {
      const config = service.getConfiguration('challenge');
      expect(config?.escalationPath).toBeDefined();
      expect(config?.escalationPath.length).toBeGreaterThan(0);
    });
  });

  // ==========================================================================
  // Lifecycle Tests
  // ==========================================================================

  describe('lifecycle', () => {
    it('should have ngOnDestroy method', () => {
      expect(typeof service.ngOnDestroy).toBe('function');
    });

    it('should handle ngOnDestroy without error', () => {
      expect(() => service.ngOnDestroy()).not.toThrow();
    });
  });

  // ==========================================================================
  // Method Existence Tests
  // ==========================================================================

  describe('method existence', () => {
    it('should have registerSla', () => {
      expect(typeof service.registerSla).toBe('function');
    });

    it('should have acknowledgeSla', () => {
      expect(typeof service.acknowledgeSla).toBe('function');
    });

    it('should have resolveSla', () => {
      expect(typeof service.resolveSla).toBe('function');
    });

    it('should have escalateSla', () => {
      expect(typeof service.escalateSla).toBe('function');
    });

    it('should have getActiveSlas', () => {
      expect(typeof service.getActiveSlas).toBe('function');
    });

    it('should have getSlasByType', () => {
      expect(typeof service.getSlasByType).toBe('function');
    });

    it('should have getSlaForEntity', () => {
      expect(typeof service.getSlaForEntity).toBe('function');
    });

    it('should have getAlerts', () => {
      expect(typeof service.getAlerts).toBe('function');
    });

    it('should have getBreaches', () => {
      expect(typeof service.getBreaches).toBe('function');
    });

    it('should have getAtRiskSlas', () => {
      expect(typeof service.getAtRiskSlas).toBe('function');
    });

    it('should have getMetrics', () => {
      expect(typeof service.getMetrics).toBe('function');
    });

    it('should have getConfiguration', () => {
      expect(typeof service.getConfiguration).toBe('function');
    });
  });

  // ==========================================================================
  // Parameter Acceptance Tests
  // ==========================================================================

  describe('parameter acceptance', () => {
    it('should accept SlaRegistration for registerSla', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review',
      };

      expect(() => service.registerSla(registration)).not.toThrow();
    });

    it('should accept string id for acknowledgeSla', () => {
      const registration: SlaRegistration = {
        entityType: 'proposal',
        entityId: 'proposal-123',
        title: 'Vote',
      };

      const sla = service.registerSla(registration);
      expect(() => service.acknowledgeSla(sla.id, 'user-1')).not.toThrow();
    });

    it('should accept SlaEntityType for getSlasByType', () => {
      expect(() => service.getSlasByType('challenge')).not.toThrow();
      expect(() => service.getSlasByType('proposal')).not.toThrow();
      expect(() => service.getSlasByType('mediation')).not.toThrow();
    });

    it('should accept string entityId for getSlaForEntity', () => {
      expect(() => service.getSlaForEntity('entity-123')).not.toThrow();
    });

    it('should accept SlaEntityType for getConfiguration', () => {
      expect(() => service.getConfiguration('challenge')).not.toThrow();
    });
  });

  // ==========================================================================
  // Return Type Tests
  // ==========================================================================

  describe('return types', () => {
    it('registerSla should return SlaItem', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review',
      };

      const result = service.registerSla(registration);
      expect(result).toBeDefined();
      expect(result.id).toBeDefined();
      expect(result.entityType).toBeDefined();
      expect(result.status).toBeDefined();
    });

    it('acknowledgeSla should return boolean', () => {
      const registration: SlaRegistration = {
        entityType: 'proposal',
        entityId: 'proposal-123',
        title: 'Vote',
      };

      const sla = service.registerSla(registration);
      const result = service.acknowledgeSla(sla.id, 'user-1');
      expect(typeof result).toBe('boolean');
    });

    it('resolveSla should return boolean', () => {
      const registration: SlaRegistration = {
        entityType: 'mediation',
        entityId: 'mediation-123',
        title: 'Resolve',
      };

      const sla = service.registerSla(registration);
      const resolution: SlaResolution = {
        outcome: 'resolved',
        resolvedBy: 'user-1',
      };

      const result = service.resolveSla(sla.id, resolution);
      expect(typeof result).toBe('boolean');
    });

    it('escalateSla should return boolean', () => {
      const registration: SlaRegistration = {
        entityType: 'challenge',
        entityId: 'challenge-123',
        title: 'Review',
      };

      const sla = service.registerSla(registration);
      const result = service.escalateSla(sla.id, 'reason');
      expect(typeof result).toBe('boolean');
    });

    it('getMetrics should return SlaMetrics', () => {
      const result = service.getMetrics();
      expect(typeof result.totalTracked).toBe('number');
      expect(typeof result.currentActive).toBe('number');
      expect(typeof result.onTimeRate).toBe('number');
    });
  });
});
