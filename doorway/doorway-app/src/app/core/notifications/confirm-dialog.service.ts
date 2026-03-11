/**
 * Confirm Dialog Service
 *
 * Provides a Promise-based confirmation dialog replacing window.confirm().
 * Uses a signal-driven dialog that can be styled and rendered in the app.
 *
 * For now, wraps window.confirm() to unblock the refactor. The dialog
 * component can be swapped in later without changing callers.
 */

import { Injectable } from '@angular/core';

@Injectable({ providedIn: 'root' })
export class ConfirmDialogService {
  /**
   * Show a confirmation dialog and return the user's choice.
   */
  async confirm(message: string): Promise<boolean> {
    // TODO: Replace with custom dialog component
    return window.confirm(message);
  }
}
