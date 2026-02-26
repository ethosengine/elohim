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
  styleUrl: './graduation-tab.component.css',
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
