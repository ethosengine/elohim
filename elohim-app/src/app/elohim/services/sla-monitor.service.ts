import { Injectable, OnDestroy } from '@angular/core';

// @coverage: 60.8% (2026-02-05)

import { BehaviorSubject, Observable, Subject, interval, takeUntil, map } from 'rxjs';

/**
 * SlaMonitorService - Constitutional SLA Enforcement
 *
 * Implements governance response time guarantees:
 * - Every governance action has a defined SLA
 * - Automatic escalation when deadlines approach
 * - Breach tracking and notification
 * - Escalation paths for unresolved items
 *
 * SLA Philosophy:
 * "Constitutional governance requires predictable response times.
 *  Without SLAs, feedback mechanisms can be silenced by inaction."
 *
 * Key SLA Categories:
 * - Challenge acknowledgment: 24 hours
 * - Challenge resolution: 7 days
 * - Proposal voting period: configurable (default 7 days)
 * - Mediation response: 1 hour
 * - Escalation response: 4 hours
 */
const GOVERNANCE_COUNCIL = 'governance-council';
const RESOLVED_ON_TIME = 'resolved-on-time';
const RESOLVED_LATE = 'resolved-late';

@Injectable({
  providedIn: 'root',
})
export class SlaMonitorService implements OnDestroy {
  private readonly destroy$ = new Subject<void>();

  // Active SLA tracking
  private readonly activeSlas$ = new BehaviorSubject<SlaItem[]>([]);
  private readonly slaAlerts$ = new Subject<SlaAlert>();
  private readonly slaBreaches$ = new Subject<SlaBreach>();

  // SLA configuration by entity type
  private readonly slaConfig = new Map<SlaEntityType, SlaConfiguration>([
    [
      'challenge',
      {
        acknowledgmentHours: 24,
        resolutionDays: 7,
        warningThresholdPercent: 75,
        criticalThresholdPercent: 90,
        escalationPath: ['assigned-elohim', GOVERNANCE_COUNCIL, 'network-stewards'],
      },
    ],
    [
      'proposal',
      {
        acknowledgmentHours: 4,
        resolutionDays: 7, // Default voting period
        warningThresholdPercent: 50,
        criticalThresholdPercent: 75,
        escalationPath: ['proposer', GOVERNANCE_COUNCIL],
      },
    ],
    [
      'mediation',
      {
        acknowledgmentHours: 1,
        resolutionDays: 1,
        warningThresholdPercent: 50,
        criticalThresholdPercent: 75,
        escalationPath: ['mediator-pool', GOVERNANCE_COUNCIL],
      },
    ],
    [
      'reaction-review',
      {
        acknowledgmentHours: 4,
        resolutionDays: 2,
        warningThresholdPercent: 75,
        criticalThresholdPercent: 90,
        escalationPath: ['content-steward', GOVERNANCE_COUNCIL],
      },
    ],
    [
      'feedback-aggregation',
      {
        acknowledgmentHours: 24,
        resolutionDays: 14,
        warningThresholdPercent: 80,
        criticalThresholdPercent: 95,
        escalationPath: ['content-governance', GOVERNANCE_COUNCIL],
      },
    ],
  ]);

  // Metrics
  private readonly metrics: SlaMetrics = {
    totalTracked: 0,
    currentActive: 0,
    breachCount: 0,
    averageResolutionHours: 0,
    onTimeRate: 100,
    escalationCount: 0,
  };

  constructor() {
    this.startMonitoringLoop();
    this.loadActiveSlasFromStorage();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  // ===========================================================================
  // Public API
  // ===========================================================================

  /**
   * Register a new SLA to track.
   */
  registerSla(item: SlaRegistration): SlaItem {
    const config = this.slaConfig.get(item.entityType);
    if (!config) {
      throw new Error(`Unknown SLA entity type: ${item.entityType}`);
    }

    const now = new Date();
    const deadline = item.customDeadline
      ? new Date(item.customDeadline)
      : this.calculateDeadline(now, config.resolutionDays);

    const slaItem: SlaItem = {
      id: this.generateId(),
      entityType: item.entityType,
      entityId: item.entityId,
      title: item.title,
      description: item.description,
      createdAt: now.toISOString(),
      deadline: deadline.toISOString(),
      acknowledgedAt: null,
      resolvedAt: null,
      status: 'pending',
      currentEscalationLevel: 0,
      escalationPath: config.escalationPath,
      assignedTo: item.assignedTo,
      priority: item.priority ?? 'normal',
      metadata: item.metadata ?? {},
    };

    const current = this.activeSlas$.value;
    this.activeSlas$.next([...current, slaItem]);
    this.persistToStorage();

    this.metrics.totalTracked++;
    this.metrics.currentActive++;

    return slaItem;
  }

  /**
   * Acknowledge an SLA (stops acknowledgment timer).
   */
  acknowledgeSla(slaId: string, acknowledgedBy: string): boolean {
    const current = this.activeSlas$.value;
    const index = current.findIndex(s => s.id === slaId);

    if (index === -1) return false;

    const sla = { ...current[index] };
    sla.acknowledgedAt = new Date().toISOString();
    sla.status = 'acknowledged';
    sla.assignedTo = acknowledgedBy;

    current[index] = sla;
    this.activeSlas$.next([...current]);
    this.persistToStorage();

    return true;
  }

  /**
   * Resolve an SLA (marks as completed).
   */
  resolveSla(slaId: string, resolution: SlaResolution): boolean {
    const current = this.activeSlas$.value;
    const index = current.findIndex(s => s.id === slaId);

    if (index === -1) return false;

    const sla = { ...current[index] };
    const now = new Date();

    sla.resolvedAt = now.toISOString();
    sla.status = this.isWithinDeadline(sla) ? RESOLVED_ON_TIME : RESOLVED_LATE;
    sla.metadata = {
      ...sla.metadata,
      resolution: resolution.outcome,
      resolutionNotes: resolution.notes,
      resolvedBy: resolution.resolvedBy,
    };

    // Update metrics
    const resolutionHours = this.hoursElapsed(new Date(sla.createdAt), now);
    this.updateAverageResolution(resolutionHours);
    this.metrics.currentActive--;

    if (sla.status === RESOLVED_LATE) {
      this.metrics.breachCount++;
      this.updateOnTimeRate();
    }

    current[index] = sla;
    this.activeSlas$.next([...current]);
    this.persistToStorage();

    return true;
  }

  /**
   * Manually escalate an SLA.
   */
  escalateSla(slaId: string, reason: string): boolean {
    const current = this.activeSlas$.value;
    const index = current.findIndex(s => s.id === slaId);

    if (index === -1) return false;

    const sla = { ...current[index] };

    if (sla.currentEscalationLevel >= sla.escalationPath.length - 1) {
      // Already at max escalation
      return false;
    }

    sla.currentEscalationLevel++;
    sla.status = 'escalated';
    sla.assignedTo = sla.escalationPath[sla.currentEscalationLevel];
    sla.metadata = {
      ...sla.metadata,
      escalationHistory: [
        ...((sla.metadata['escalationHistory'] as EscalationRecord[]) || []),
        {
          level: sla.currentEscalationLevel,
          reason,
          timestamp: new Date().toISOString(),
          escalatedTo: sla.assignedTo,
        },
      ],
    };

    this.metrics.escalationCount++;

    current[index] = sla;
    this.activeSlas$.next([...current]);
    this.persistToStorage();

    // Emit escalation alert
    this.slaAlerts$.next({
      type: 'escalation',
      slaId: sla.id,
      entityType: sla.entityType,
      message: `SLA escalated to ${sla.assignedTo}: ${reason}`,
      timestamp: new Date().toISOString(),
      severity: 'high',
    });

    return true;
  }

  /**
   * Get all active SLAs.
   */
  getActiveSlas(): Observable<SlaItem[]> {
    return this.activeSlas$.asObservable();
  }

  /**
   * Get SLAs by entity type.
   */
  getSlasByType(entityType: SlaEntityType): Observable<SlaItem[]> {
    return this.activeSlas$.pipe(map(slas => slas.filter(s => s.entityType === entityType)));
  }

  /**
   * Get SLAs for a specific entity.
   */
  getSlaForEntity(entityId: string): Observable<SlaItem | undefined> {
    return this.activeSlas$.pipe(
      map(slas =>
        slas.find(
          s =>
            s.entityId === entityId && s.status !== RESOLVED_ON_TIME && s.status !== RESOLVED_LATE
        )
      )
    );
  }

  /**
   * Get SLA alerts stream.
   */
  getAlerts(): Observable<SlaAlert> {
    return this.slaAlerts$.asObservable();
  }

  /**
   * Get SLA breaches stream.
   */
  getBreaches(): Observable<SlaBreach> {
    return this.slaBreaches$.asObservable();
  }

  /**
   * Get SLA at risk (approaching deadline).
   */
  getAtRiskSlas(): Observable<SlaItem[]> {
    return this.activeSlas$.pipe(
      map(slas =>
        slas.filter(s => {
          if (s.status === RESOLVED_ON_TIME || s.status === RESOLVED_LATE) return false;
          const config = this.slaConfig.get(s.entityType);
          if (!config) return false;

          const progress = this.calculateProgress(s);
          return progress >= config.warningThresholdPercent;
        })
      )
    );
  }

  /**
   * Get current SLA metrics.
   */
  getMetrics(): SlaMetrics {
    return { ...this.metrics };
  }

  /**
   * Get SLA configuration for an entity type.
   */
  getConfiguration(entityType: SlaEntityType): SlaConfiguration | undefined {
    return this.slaConfig.get(entityType);
  }

  // ===========================================================================
  // Monitoring Loop
  // ===========================================================================

  private startMonitoringLoop(): void {
    // Check every minute
    interval(60000)
      .pipe(takeUntil(this.destroy$))
      .subscribe(() => this.checkAllSlas());
  }

  private checkAllSlas(): void {
    const current = this.activeSlas$.value;
    const now = new Date();

    for (const sla of current) {
      if (sla.status === RESOLVED_ON_TIME || sla.status === RESOLVED_LATE) continue;

      const config = this.slaConfig.get(sla.entityType);
      if (!config) continue;

      const progress = this.calculateProgress(sla);
      const deadline = new Date(sla.deadline);

      // Check for breach
      if (now > deadline) {
        this.handleBreach(sla);
        continue;
      }

      // Check for critical threshold
      if (progress >= config.criticalThresholdPercent && sla.status !== 'critical') {
        this.handleCritical(sla);
        continue;
      }

      // Check for warning threshold
      if (progress >= config.warningThresholdPercent && sla.status === 'pending') {
        this.handleWarning(sla);
      }
    }
  }

  private handleWarning(sla: SlaItem): void {
    const updated = this.updateSlaStatus(sla.id, 'warning');
    if (updated) {
      this.slaAlerts$.next({
        type: 'warning',
        slaId: sla.id,
        entityType: sla.entityType,
        message: `SLA approaching deadline: ${sla.title}`,
        timestamp: new Date().toISOString(),
        severity: 'medium',
      });
    }
  }

  private handleCritical(sla: SlaItem): void {
    const updated = this.updateSlaStatus(sla.id, 'critical');
    if (updated) {
      this.slaAlerts$.next({
        type: 'critical',
        slaId: sla.id,
        entityType: sla.entityType,
        message: `SLA critically close to deadline: ${sla.title}`,
        timestamp: new Date().toISOString(),
        severity: 'high',
      });

      // Auto-escalate on critical if not already escalated
      if (sla.currentEscalationLevel === 0) {
        this.escalateSla(sla.id, 'Automatic escalation: deadline critically approaching');
      }
    }
  }

  private handleBreach(sla: SlaItem): void {
    const updated = this.updateSlaStatus(sla.id, 'breached');
    if (updated) {
      const breach: SlaBreach = {
        slaId: sla.id,
        entityType: sla.entityType,
        entityId: sla.entityId,
        title: sla.title,
        deadline: sla.deadline,
        breachedAt: new Date().toISOString(),
        assignedTo: sla.assignedTo,
        escalationLevel: sla.currentEscalationLevel,
      };

      this.slaBreaches$.next(breach);
      this.metrics.breachCount++;
      this.updateOnTimeRate();

      // Auto-escalate to max level on breach
      while (sla.currentEscalationLevel < sla.escalationPath.length - 1) {
        this.escalateSla(sla.id, 'Automatic escalation: SLA breached');
      }
    }
  }

  private updateSlaStatus(slaId: string, status: SlaStatus): boolean {
    const current = this.activeSlas$.value;
    const index = current.findIndex(s => s.id === slaId);

    if (index === -1) return false;

    const sla = { ...current[index] };
    if (sla.status === status) return false; // No change

    sla.status = status;
    current[index] = sla;
    this.activeSlas$.next([...current]);
    this.persistToStorage();

    return true;
  }

  // ===========================================================================
  // Helpers
  // ===========================================================================

  private calculateProgress(sla: SlaItem): number {
    const created = new Date(sla.createdAt);
    const deadline = new Date(sla.deadline);
    const now = new Date();

    const totalDuration = deadline.getTime() - created.getTime();
    const elapsed = now.getTime() - created.getTime();

    return Math.min(100, Math.max(0, (elapsed / totalDuration) * 100));
  }

  private calculateDeadline(from: Date, days: number): Date {
    const deadline = new Date(from);
    deadline.setDate(deadline.getDate() + days);
    return deadline;
  }

  private isWithinDeadline(sla: SlaItem): boolean {
    const deadline = new Date(sla.deadline);
    const resolved = sla.resolvedAt ? new Date(sla.resolvedAt) : new Date();
    return resolved <= deadline;
  }

  private hoursElapsed(from: Date, to: Date): number {
    return (to.getTime() - from.getTime()) / (1000 * 60 * 60);
  }

  private updateAverageResolution(hours: number): void {
    const totalResolved = this.metrics.totalTracked - this.metrics.currentActive;
    if (totalResolved === 0) {
      this.metrics.averageResolutionHours = hours;
    } else {
      // Running average
      this.metrics.averageResolutionHours =
        (this.metrics.averageResolutionHours * (totalResolved - 1) + hours) / totalResolved;
    }
  }

  private updateOnTimeRate(): void {
    const totalResolved = this.metrics.totalTracked - this.metrics.currentActive;
    if (totalResolved === 0) {
      this.metrics.onTimeRate = 100;
    } else {
      const onTime = totalResolved - this.metrics.breachCount;
      this.metrics.onTimeRate = Math.round((onTime / totalResolved) * 100);
    }
  }

  private generateId(): string {
    const randomBytes = crypto.getRandomValues(new Uint8Array(6));
    const randomStr = Array.from(randomBytes)
      .map(b => b.toString(36))
      .join('')
      .substring(0, 7);
    return `sla-${Date.now()}-${randomStr}`;
  }

  // ===========================================================================
  // Persistence (MVP: localStorage)
  // ===========================================================================

  private readonly STORAGE_KEY = 'elohim_sla_items';

  private persistToStorage(): void {
    try {
      const data = JSON.stringify(this.activeSlas$.value);
      localStorage.setItem(this.STORAGE_KEY, data);
    } catch {
      // localStorage write failure is non-critical
    }
  }

  private loadActiveSlasFromStorage(): void {
    try {
      const data = localStorage.getItem(this.STORAGE_KEY);
      if (data) {
        const items = JSON.parse(data) as SlaItem[];
        // Filter out old resolved items
        const active = items.filter(
          s => s.status !== RESOLVED_ON_TIME && s.status !== RESOLVED_LATE
        );
        this.activeSlas$.next(active);
        this.metrics.currentActive = active.length;
      }
    } catch {
      // localStorage read failure is non-critical - will start with empty state
    }
  }
}

// ===========================================================================
// Types
// ===========================================================================

export type SlaEntityType =
  | 'challenge'
  | 'proposal'
  | 'mediation'
  | 'reaction-review'
  | 'feedback-aggregation';

export type SlaStatus =
  | 'pending'
  | 'acknowledged'
  | 'warning'
  | 'critical'
  | 'escalated'
  | 'breached'
  | typeof RESOLVED_ON_TIME
  | typeof RESOLVED_LATE;

export type SlaPriority = 'low' | 'normal' | 'high' | 'critical';

export interface SlaConfiguration {
  acknowledgmentHours: number;
  resolutionDays: number;
  warningThresholdPercent: number;
  criticalThresholdPercent: number;
  escalationPath: string[];
}

export interface SlaRegistration {
  entityType: SlaEntityType;
  entityId: string;
  title: string;
  description?: string;
  customDeadline?: string;
  assignedTo?: string;
  priority?: SlaPriority;
  metadata?: Record<string, unknown>;
}

export interface SlaItem {
  id: string;
  entityType: SlaEntityType;
  entityId: string;
  title: string;
  description?: string;
  createdAt: string;
  deadline: string;
  acknowledgedAt: string | null;
  resolvedAt: string | null;
  status: SlaStatus;
  currentEscalationLevel: number;
  escalationPath: string[];
  assignedTo?: string;
  priority: SlaPriority;
  metadata: Record<string, unknown>;
}

export interface SlaResolution {
  outcome: 'resolved' | 'dismissed' | 'withdrawn';
  notes?: string;
  resolvedBy: string;
}

export interface SlaAlert {
  type: 'warning' | 'critical' | 'escalation';
  slaId: string;
  entityType: SlaEntityType;
  message: string;
  timestamp: string;
  severity: 'low' | 'medium' | 'high';
}

export interface SlaBreach {
  slaId: string;
  entityType: SlaEntityType;
  entityId: string;
  title: string;
  deadline: string;
  breachedAt: string;
  assignedTo?: string;
  escalationLevel: number;
}

export interface SlaMetrics {
  totalTracked: number;
  currentActive: number;
  breachCount: number;
  averageResolutionHours: number;
  onTimeRate: number;
  escalationCount: number;
}

interface EscalationRecord {
  level: number;
  reason: string;
  timestamp: string;
  escalatedTo: string;
}
