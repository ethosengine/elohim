/**
 * Banner Notice Model
 *
 * Provider-based aggregation pattern for banner notifications.
 * Any source (agents, services, session) can emit notices and
 * the navigator renders whatever the system produces.
 */

import { Observable } from 'rxjs';

import type { AlertAction, AlertData, AlertSeverity } from '../../shared/components/alert-banner';

/**
 * Context where a banner notice should be displayed.
 * 'global' means it appears in all contexts.
 */
export type BannerContext = 'global' | 'lamad' | 'shefa' | 'qahal' | 'doorway';

/**
 * Priority levels for banner notices, ordered by urgency.
 */
export type BannerPriority = 'system' | 'agent' | 'info';

/**
 * Sort order for priorities (lower = higher priority).
 */
export const BANNER_PRIORITY_ORDER: Record<BannerPriority, number> = {
  system: 0,
  agent: 1,
  info: 2,
};

/**
 * Sort order for severities (lower = higher priority).
 */
export const BANNER_SEVERITY_ORDER: Record<AlertSeverity, number> = {
  critical: 0,
  error: 1,
  warning: 2,
  info: 3,
  success: 4,
};

/**
 * A generic banner notice emitted by any provider.
 */
export interface BannerNotice {
  /** Unique identifier for this notice */
  id: string;

  /** ID of the provider that emitted this notice */
  providerId: string;

  /** Alert severity (reuses AlertSeverity from AlertBannerComponent) */
  severity: AlertSeverity;

  /** Priority level for sorting */
  priority: BannerPriority;

  /** Contexts where this notice should appear */
  contexts: BannerContext[];

  /** Short title */
  title: string;

  /** Longer descriptive message */
  message?: string;

  /** Action buttons (reuses AlertAction from AlertBannerComponent) */
  actions?: AlertAction[];

  /** Whether the user can dismiss this notice */
  dismissible: boolean;

  /** When this notice was created */
  createdAt: Date;

  /** Arbitrary metadata for provider-specific data */
  metadata?: Record<string, unknown>;
}

/**
 * Interface for banner notice providers.
 *
 * Any service that wants to emit banner notices implements this interface
 * and registers with BannerService.
 */
export interface BannerNoticeProvider {
  /** Unique provider identifier */
  readonly providerId: string;

  /** Stream of notices from this provider */
  readonly notices$: Observable<BannerNotice[]>;

  /** Called when a user dismisses a notice from this provider */
  dismissNotice(noticeId: string): void;

  /** Called when a user clicks an action on a notice from this provider */
  handleAction(noticeId: string, actionId: string): void;
}

/**
 * Maps a BannerNotice to AlertData for rendering via AlertBannerComponent.
 */
export function bannerNoticeToAlertData(notice: BannerNotice): AlertData {
  return {
    id: notice.id,
    severity: notice.severity,
    title: notice.title,
    message: notice.message,
    actions: notice.actions,
    dismissible: notice.dismissible,
    timestamp: notice.createdAt.toISOString(),
    metadata: {
      providerId: notice.providerId,
      priority: notice.priority,
      ...notice.metadata,
    },
  };
}
