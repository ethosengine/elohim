/**
 * Notification Service
 *
 * Centralized toast/notification system replacing alert() dialogs.
 * Provides success, error, warning, and info notifications that
 * auto-dismiss after a configurable timeout.
 */

import { Injectable, signal, computed } from '@angular/core';

export type NotificationType = 'success' | 'error' | 'warning' | 'info';

export interface Notification {
  id: number;
  type: NotificationType;
  message: string;
  dismissAt: number;
}

const DEFAULT_DURATION_MS = 5000;
const ERROR_DURATION_MS = 8000;

@Injectable({ providedIn: 'root' })
export class NotificationService {
  private nextId = 0;
  private readonly _notifications = signal<Notification[]>([]);
  readonly notifications = this._notifications.asReadonly();
  readonly hasNotifications = computed(() => this._notifications().length > 0);

  success(message: string): void {
    this.add('success', message, DEFAULT_DURATION_MS);
  }

  error(message: string): void {
    this.add('error', message, ERROR_DURATION_MS);
  }

  warning(message: string): void {
    this.add('warning', message, DEFAULT_DURATION_MS);
  }

  info(message: string): void {
    this.add('info', message, DEFAULT_DURATION_MS);
  }

  dismiss(id: number): void {
    this._notifications.update(list => list.filter(n => n.id !== id));
  }

  private add(type: NotificationType, message: string, duration: number): void {
    const id = this.nextId++;
    const dismissAt = Date.now() + duration;

    this._notifications.update(list => [...list, { id, type, message, dismissAt }]);

    setTimeout(() => this.dismiss(id), duration);
  }
}
