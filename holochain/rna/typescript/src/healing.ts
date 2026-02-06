/**
 * Healing Signal Monitoring
 *
 * Apps subscribe to healing signals to show progress and status in the UI.
 * This enables real-time awareness of which entries are degraded, being healed,
 * or have been successfully repaired.
 */

/**
 * Signals emitted during the healing process
 *
 * Apps can listen for these to update UI and show healing progress.
 */
export enum HealingSignalType {
  DegradedEntryFound = "DegradedEntryFound",
  HealingStarted = "HealingStarted",
  HealingSucceeded = "HealingSucceeded",
  HealingRetrying = "HealingRetrying",
  HealingFailed = "HealingFailed",
  HealingBatchComplete = "HealingBatchComplete",
  SystemFullyHealed = "SystemFullyHealed",
}

/**
 * Base healing signal (all signals have these fields)
 */
export interface BaseHealingSignal {
  type: HealingSignalType;
  timestamp: number;
}

/**
 * A degraded entry was discovered
 */
export interface DegradedEntryFoundSignal extends BaseHealingSignal {
  type: HealingSignalType.DegradedEntryFound;
  entry_id: string;
  entry_type: string;
  reason: string;
}

/**
 * Healing started for an entry
 */
export interface HealingStartedSignal extends BaseHealingSignal {
  type: HealingSignalType.HealingStarted;
  entry_id: string;
  attempt: number;
}

/**
 * An entry was successfully healed
 */
export interface HealingSucceededSignal extends BaseHealingSignal {
  type: HealingSignalType.HealingSucceeded;
  entry_id: string;
  entry_type: string;
  was_migrated_from_v1: boolean;
}

/**
 * Healing failed but will be retried
 */
export interface HealingRetryingSignal extends BaseHealingSignal {
  type: HealingSignalType.HealingRetrying;
  entry_id: string;
  reason: string;
  next_attempt_in_seconds: number;
}

/**
 * Healing permanently failed for an entry
 */
export interface HealingFailedSignal extends BaseHealingSignal {
  type: HealingSignalType.HealingFailed;
  entry_id: string;
  entry_type: string;
  final_error: string;
}

/**
 * A batch of entries was healed
 */
export interface HealingBatchCompleteSignal extends BaseHealingSignal {
  type: HealingSignalType.HealingBatchComplete;
  entry_type: string;
  total_found: number;
  healed: number;
  failed: number;
}

/**
 * System has fully healed (no more degraded entries)
 */
export interface SystemFullyHealedSignal extends BaseHealingSignal {
  type: HealingSignalType.SystemFullyHealed;
  time_taken_seconds: number;
  total_entries_healed: number;
}

export type HealingSignal =
  | DegradedEntryFoundSignal
  | HealingStartedSignal
  | HealingSucceededSignal
  | HealingRetryingSignal
  | HealingFailedSignal
  | HealingBatchCompleteSignal
  | SystemFullyHealedSignal;

/**
 * Status of a specific entry being healed
 */
export interface HealingEntryStatus {
  entry_id: string;
  entry_type: string;
  status: "healthy" | "degraded" | "healing" | "healed" | "failed";
  attempts: number;
  last_error?: string;
  healed_at?: number;
  was_migrated_from_v1: boolean;
}

/**
 * Overall system healing status
 */
export interface SystemHealingStatus {
  is_healthy: boolean;
  total_entries_checked: number;
  degraded_count: number;
  healed_count: number;
  failed_count: number;
  healing_in_progress_count: number;
  entries: Map<string, HealingEntryStatus>;
  healing_started_at?: number;
  healing_completed_at?: number;
  total_time_seconds?: number;
}

/**
 * Monitor healing signals and track system health
 */
export class HealingMonitor {
  private signals: HealingSignal[] = [];
  private entryStatus = new Map<string, HealingEntryStatus>();
  private systemStatus: SystemHealingStatus;
  private signalListeners: ((signal: HealingSignal) => void)[] = [];
  private statusChangeListeners: (() => void)[] = [];

  constructor() {
    this.systemStatus = {
      is_healthy: true,
      total_entries_checked: 0,
      degraded_count: 0,
      healed_count: 0,
      failed_count: 0,
      healing_in_progress_count: 0,
      entries: this.entryStatus,
    };
  }

  /**
   * Process a healing signal
   */
  processSignal(signal: HealingSignal): void {
    // Add timestamp if not present
    if (!signal.timestamp) {
      signal.timestamp = Date.now();
    }

    this.signals.push(signal);

    switch (signal.type) {
      case HealingSignalType.DegradedEntryFound:
        this.handleDegradedFound(signal as DegradedEntryFoundSignal);
        break;
      case HealingSignalType.HealingStarted:
        this.handleHealingStarted(signal as HealingStartedSignal);
        break;
      case HealingSignalType.HealingSucceeded:
        this.handleHealingSucceeded(signal as HealingSucceededSignal);
        break;
      case HealingSignalType.HealingFailed:
        this.handleHealingFailed(signal as HealingFailedSignal);
        break;
      case HealingSignalType.SystemFullyHealed:
        this.handleSystemFullyHealed(signal as SystemFullyHealedSignal);
        break;
    }

    // Notify listeners
    this.signalListeners.forEach((listener) => listener(signal));
    this.statusChangeListeners.forEach((listener) => listener());
  }

  /**
   * Subscribe to healing signals
   */
  onSignal(listener: (signal: HealingSignal) => void): void {
    this.signalListeners.push(listener);
  }

  /**
   * Subscribe to healing status changes
   */
  onStatusChange(listener: () => void): void {
    this.statusChangeListeners.push(listener);
  }

  /**
   * Get current system healing status
   */
  getStatus(): SystemHealingStatus {
    return {
      ...this.systemStatus,
      entries: new Map(this.entryStatus),
    };
  }

  /**
   * Get status of a specific entry
   */
  getEntryStatus(entryId: string): HealingEntryStatus | undefined {
    return this.entryStatus.get(entryId);
  }

  /**
   * Get all degraded entries
   */
  getDegradedEntries(): HealingEntryStatus[] {
    return Array.from(this.entryStatus.values()).filter(
      (s) => s.status === "degraded"
    );
  }

  /**
   * Get all entries being healed
   */
  getHealingEntries(): HealingEntryStatus[] {
    return Array.from(this.entryStatus.values()).filter(
      (s) => s.status === "healing"
    );
  }

  /**
   * Get signal history
   */
  getSignalHistory(limit?: number): HealingSignal[] {
    if (limit) {
      return this.signals.slice(-limit);
    }
    return [...this.signals];
  }

  /**
   * Clear all tracking data
   */
  reset(): void {
    this.signals = [];
    this.entryStatus.clear();
    this.systemStatus = {
      is_healthy: true,
      total_entries_checked: 0,
      degraded_count: 0,
      healed_count: 0,
      failed_count: 0,
      healing_in_progress_count: 0,
      entries: this.entryStatus,
    };
  }

  // Private helpers

  private handleDegradedFound(signal: DegradedEntryFoundSignal): void {
    const status: HealingEntryStatus = {
      entry_id: signal.entry_id,
      entry_type: signal.entry_type,
      status: "degraded",
      attempts: 0,
      last_error: signal.reason,
      was_migrated_from_v1: false,
    };

    this.entryStatus.set(signal.entry_id, status);
    this.systemStatus.degraded_count += 1;
    this.systemStatus.is_healthy = false;
    this.systemStatus.total_entries_checked += 1;
  }

  private handleHealingStarted(signal: HealingStartedSignal): void {
    const status = this.entryStatus.get(signal.entry_id);
    if (status) {
      status.status = "healing";
      status.attempts = signal.attempt;
      this.systemStatus.healing_in_progress_count += 1;
      if (!this.systemStatus.healing_started_at) {
        this.systemStatus.healing_started_at = Date.now();
      }
    }
  }

  private handleHealingSucceeded(signal: HealingSucceededSignal): void {
    const status = this.entryStatus.get(signal.entry_id);
    if (status) {
      status.status = "healed";
      status.healed_at = Date.now();
      status.was_migrated_from_v1 = signal.was_migrated_from_v1;
      this.systemStatus.healed_count += 1;
      this.systemStatus.healing_in_progress_count -= 1;
    }
  }

  private handleHealingFailed(signal: HealingFailedSignal): void {
    const status = this.entryStatus.get(signal.entry_id);
    if (status) {
      status.status = "failed";
      status.last_error = signal.final_error;
      this.systemStatus.failed_count += 1;
      this.systemStatus.healing_in_progress_count -= 1;
    }
  }

  private handleSystemFullyHealed(signal: SystemFullyHealedSignal): void {
    this.systemStatus.is_healthy = true;
    this.systemStatus.healing_completed_at = Date.now();
    this.systemStatus.total_time_seconds = signal.time_taken_seconds;
    this.systemStatus.healing_in_progress_count = 0;

    // Update all entries to healthy if they were healed
    for (const status of this.entryStatus.values()) {
      if (status.status === "healed" || status.status === "healthy") {
        status.status = "healthy";
      }
    }
  }
}

/**
 * Format a healing signal for display
 */
export function formatHealingSignal(signal: HealingSignal): string {
  switch (signal.type) {
    case HealingSignalType.DegradedEntryFound:
      return `Entry ${signal.entry_id} is degraded: ${signal.reason}`;
    case HealingSignalType.HealingStarted:
      return `Healing ${signal.entry_id} (attempt ${signal.attempt})`;
    case HealingSignalType.HealingSucceeded:
      return `Successfully healed ${signal.entry_type} ${signal.entry_id}${
        signal.was_migrated_from_v1 ? " (from v1)" : ""
      }`;
    case HealingSignalType.HealingRetrying:
      return `Retrying ${signal.entry_id} in ${signal.next_attempt_in_seconds}s`;
    case HealingSignalType.HealingFailed:
      return `Failed to heal ${signal.entry_id}: ${signal.final_error}`;
    case HealingSignalType.HealingBatchComplete:
      return `Healed ${signal.healed}/${signal.total_found} ${signal.entry_type} entries`;
    case HealingSignalType.SystemFullyHealed:
      return `System fully healed: ${signal.total_entries_healed} entries in ${signal.time_taken_seconds}s`;
    default:
      return "Unknown healing signal";
  }
}

/**
 * Helper to create a healing monitor for Holochain apps
 */
export function createHealingMonitor(): HealingMonitor {
  const monitor = new HealingMonitor();

  // In a real app, subscribe to Holochain zome signals here:
  // appWebsocket.on('signal', (signal) => {
  //   if (isHealingSignal(signal)) {
  //     monitor.processSignal(signal);
  //   }
  // });

  return monitor;
}

/**
 * Check if a signal is a healing signal
 */
export function isHealingSignal(signal: any): signal is HealingSignal {
  return (
    signal &&
    typeof signal === "object" &&
    Object.values(HealingSignalType).includes(signal.type)
  );
}
