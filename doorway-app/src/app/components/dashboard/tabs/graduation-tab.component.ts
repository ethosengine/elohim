/**
 * Graduation Tab Component
 *
 * Two tables: pending graduation (key exported, not yet steward)
 * and completed graduates. Includes force-graduate action per user.
 */

import { Component, OnInit, inject, signal } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DoorwayAdminService } from '../../../services/doorway-admin.service';
import { GraduationUser } from '../../../models/doorway.model';

@Component({
  selector: 'app-graduation-tab',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="graduation-tab">
      @if (loading()) {
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Loading graduation data...</p>
        </div>
      } @else {
        <!-- Pending Graduation -->
        <section class="graduation-section">
          <h3>Pending Graduation ({{ pending().length }})</h3>
          @if (pending().length > 0) {
            <table class="graduation-table">
              <thead>
                <tr>
                  <th>Identifier</th>
                  <th>Key Exported</th>
                  <th>Local Conductor</th>
                  <th>Key Export Date</th>
                  <th>Actions</th>
                </tr>
              </thead>
              <tbody>
                @for (user of pending(); track user.id) {
                  <tr>
                    <td class="identifier">{{ user.identifier }}</td>
                    <td>
                      <span class="bool-badge" [class.yes]="user.hasExportedKey">
                        {{ user.hasExportedKey ? 'Yes' : 'No' }}
                      </span>
                    </td>
                    <td>
                      <span class="bool-badge" [class.yes]="user.hasLocalConductor">
                        {{ user.hasLocalConductor ? 'Yes' : 'No' }}
                      </span>
                    </td>
                    <td class="date">{{ user.keyExportedAt | date:'short' }}</td>
                    <td>
                      <button
                        class="btn-graduate"
                        (click)="forceGraduate(user)"
                        [disabled]="graduating().has(user.id)">
                        @if (graduating().has(user.id)) {
                          Graduating...
                        } @else {
                          Force Graduate
                        }
                      </button>
                    </td>
                  </tr>
                }
              </tbody>
            </table>
          } @else {
            <div class="empty-state">No users pending graduation</div>
          }
        </section>

        <!-- Completed Graduation -->
        <section class="graduation-section">
          <h3>Completed ({{ completed().length }})</h3>
          @if (completed().length > 0) {
            <table class="graduation-table">
              <thead>
                <tr>
                  <th>Identifier</th>
                  <th>Graduated</th>
                  <th>Account Created</th>
                </tr>
              </thead>
              <tbody>
                @for (user of completed(); track user.id) {
                  <tr>
                    <td class="identifier">{{ user.identifier }}</td>
                    <td class="date">{{ user.graduatedAt | date:'short' }}</td>
                    <td class="date">{{ user.createdAt | date:'short' }}</td>
                  </tr>
                }
              </tbody>
            </table>
          } @else {
            <div class="empty-state">No graduated users yet</div>
          }
        </section>
      }
    </div>
  `,
  styles: [`
    .graduation-tab {
      padding: 1rem 0;
    }

    .loading-state {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 3rem;
      color: var(--text-secondary, #6b7280);
    }

    .spinner {
      width: 32px;
      height: 32px;
      border: 3px solid #e5e7eb;
      border-top-color: #3b82f6;
      border-radius: 50%;
      animation: spin 0.8s linear infinite;
      margin-bottom: 1rem;
    }

    @keyframes spin {
      to { transform: rotate(360deg); }
    }

    .graduation-section {
      margin-bottom: 2rem;

      h3 {
        font-size: 1rem;
        font-weight: 600;
        color: var(--text-primary, #111827);
        margin: 0 0 1rem;
      }
    }

    .graduation-table {
      width: 100%;
      border-collapse: collapse;
      font-size: 0.875rem;

      th, td {
        padding: 0.75rem 1rem;
        text-align: left;
        border-bottom: 1px solid var(--border-color, #e5e7eb);
      }

      th {
        background: #f9fafb;
        font-weight: 500;
        color: var(--text-secondary, #6b7280);
      }

      tbody tr:hover {
        background: #f9fafb;
      }

      .identifier {
        font-weight: 500;
      }

      .date {
        color: var(--text-secondary, #6b7280);
        font-size: 0.8125rem;
      }
    }

    .bool-badge {
      display: inline-block;
      padding: 0.25rem 0.5rem;
      border-radius: 0.25rem;
      font-size: 0.75rem;
      font-weight: 500;
      background: #fee2e2;
      color: #dc2626;

      &.yes {
        background: #d1fae5;
        color: #059669;
      }
    }

    .btn-graduate {
      padding: 0.375rem 0.75rem;
      border: 1px solid #6366f1;
      border-radius: 0.375rem;
      background: white;
      color: #6366f1;
      font-size: 0.8125rem;
      font-weight: 500;
      cursor: pointer;
      transition: background 0.15s;

      &:hover:not(:disabled) {
        background: #eef2ff;
      }

      &:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }
    }

    .empty-state {
      text-align: center;
      padding: 2rem;
      color: var(--text-secondary, #6b7280);
    }

    @media (prefers-color-scheme: dark) {
      .graduation-table th {
        background: #1f2937;
      }

      .graduation-table tbody tr:hover {
        background: #374151;
      }

      .btn-graduate {
        background: #1f2937;
        border-color: #6366f1;

        &:hover:not(:disabled) {
          background: #312e81;
        }
      }
    }
  `],
})
export class GraduationTabComponent implements OnInit {
  private readonly adminService = inject(DoorwayAdminService);

  readonly loading = signal(true);
  readonly pending = signal<GraduationUser[]>([]);
  readonly completed = signal<GraduationUser[]>([]);
  readonly graduating = signal<Set<string>>(new Set());

  ngOnInit(): void {
    this.loadData();
  }

  async loadData(): Promise<void> {
    this.loading.set(true);

    try {
      const [pendingRes, completedRes] = await Promise.all([
        this.adminService.getGraduationPending().toPromise(),
        this.adminService.getGraduationCompleted().toPromise(),
      ]);

      if (pendingRes) {
        this.pending.set(pendingRes.users);
      }
      if (completedRes) {
        this.completed.set(completedRes.users);
      }
    } catch {
      // Errors handled by service fallbacks
    } finally {
      this.loading.set(false);
    }
  }

  async forceGraduate(user: GraduationUser): Promise<void> {
    // Mark as in-progress
    this.graduating.update(set => {
      const next = new Set(set);
      next.add(user.id);
      return next;
    });

    try {
      const result = await this.adminService.forceGraduate(user.id).toPromise();
      if (result?.success) {
        // Refresh data
        await this.loadData();
      } else {
        alert('Failed to graduate user: ' + (result?.message ?? 'Unknown error'));
      }
    } catch {
      alert('Failed to graduate user');
    } finally {
      this.graduating.update(set => {
        const next = new Set(set);
        next.delete(user.id);
        return next;
      });
    }
  }
}
