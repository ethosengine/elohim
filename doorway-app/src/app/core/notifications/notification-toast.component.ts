/**
 * Notification Toast Component
 *
 * Renders stacked toast notifications from NotificationService.
 * Include once in the app root template.
 */

import { Component, inject } from '@angular/core';

import { NotificationService } from './notification.service';

@Component({
  selector: 'app-notification-toast',
  standalone: true,
  template: `
    @if (notificationService.hasNotifications()) {
      <div class="toast-container">
        @for (n of notificationService.notifications(); track n.id) {
          <div class="toast" [class]="'toast--' + n.type" role="alert">
            <span class="toast__message">{{ n.message }}</span>
            <button
              type="button"
              class="toast__dismiss"
              aria-label="Dismiss"
              (click)="notificationService.dismiss(n.id)"
            >
              &times;
            </button>
          </div>
        }
      </div>
    }
  `,
  styles: [
    `
      .toast-container {
        position: fixed;
        top: 1rem;
        right: 1rem;
        z-index: 10000;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
        max-width: 400px;
      }

      .toast {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        padding: 0.75rem 1rem;
        border-radius: 6px;
        color: #fff;
        font-size: 0.875rem;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
        animation: toast-in 0.2s ease-out;
      }

      .toast--success {
        background: #2e7d32;
      }
      .toast--error {
        background: #c62828;
      }
      .toast--warning {
        background: #e65100;
      }
      .toast--info {
        background: #1565c0;
      }

      .toast__message {
        flex: 1;
      }

      .toast__dismiss {
        background: none;
        border: none;
        color: inherit;
        font-size: 1.25rem;
        cursor: pointer;
        padding: 0 0.25rem;
        opacity: 0.7;
      }

      .toast__dismiss:hover {
        opacity: 1;
      }

      @keyframes toast-in {
        from {
          opacity: 0;
          transform: translateX(1rem);
        }
        to {
          opacity: 1;
          transform: translateX(0);
        }
      }
    `,
  ],
})
export class NotificationToastComponent {
  readonly notificationService = inject(NotificationService);
}
