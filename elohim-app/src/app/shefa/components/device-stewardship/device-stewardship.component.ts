import { CommonModule } from '@angular/common';
import { Component, OnInit, computed, inject, signal } from '@angular/core';
import { RouterModule } from '@angular/router';

import { AGENCY_STAGES, getNextStage } from '@app/imagodei/models/agency.model';
import { IdentityService } from '@app/imagodei/services/identity.service';
import { getTriggerLabel, formatPoints } from '@app/lamad/models/learning-points.model';
import { MasteryService } from '@app/lamad/services/mastery.service';
import { PointsService } from '@app/lamad/services/points.service';

import {
  getDeviceStatusDisplay,
  getDeviceCategoryDisplay,
  getDevicePlatformDisplay,
} from '../../models/device-stewardship.model';
import { getNodeTypeDisplay } from '../../models/shefa-dashboard.model';
import { DeviceStewardshipService } from '../../services/device-stewardship.service';

import type { StewardedDevice, DeviceStatus } from '../../models/device-stewardship.model';
import type { LamadPointTrigger } from '@app/lamad/models/learning-points.model';

/**
 * DeviceStewardshipComponent - Tabbed stewardship view.
 *
 * Activity tab: participation points & mastery stats (all authenticated users)
 * Devices tab: stewarded device infrastructure (stewards) or upgrade prompt (hosted/visitors)
 */
@Component({
  selector: 'app-device-stewardship',
  standalone: true,
  imports: [CommonModule, RouterModule],
  template: `
    <div class="device-stewardship">
      <!-- Header -->
      <div class="page-header">
        <div class="header-content">
          <h1>Your Stewardship</h1>
          <p class="subtitle">Track your participation and resources</p>
        </div>
        <button class="refresh-btn" (click)="refresh()" [disabled]="service.isLoading()">
          <span class="material-icons">refresh</span>
          Refresh
        </button>
      </div>

      <!-- Tab Bar -->
      <div class="tab-bar">
        <button
          class="tab-btn"
          [class.active]="activeTab() === 'activity'"
          (click)="selectTab('activity')"
        >
          <span class="material-icons tab-icon">trending_up</span>
          Activity
        </button>
        <button
          class="tab-btn"
          [class.active]="activeTab() === 'devices'"
          (click)="selectTab('devices')"
        >
          <span class="material-icons tab-icon">devices</span>
          Devices
          <span class="lock-icon" *ngIf="!isSteward()">
            <span class="material-icons">lock</span>
          </span>
        </button>
      </div>

      <!-- ============================================================= -->
      <!-- ACTIVITY TAB                                                   -->
      <!-- ============================================================= -->
      <div class="tab-content" *ngIf="activeTab() === 'activity'">
        <!-- Points Summary Strip -->
        <div class="points-strip" *ngIf="pointsBalance() as balance">
          <div class="points-stat">
            <span class="stat-value">{{ balance.total_points }}</span>
            <span class="stat-label">Total Points</span>
          </div>
          <div class="points-stat">
            <span class="stat-value">{{ weeklyPoints() }}</span>
            <span class="stat-label">This Week</span>
          </div>
          <div class="points-stat">
            <span class="stat-value">{{ balance.total_earned }}</span>
            <span class="stat-label">Lifetime Earned</span>
          </div>
        </div>

        <!-- Points Breakdown -->
        <div class="section" *ngIf="triggerEntries().length > 0">
          <div class="section-header">
            <span class="material-icons">star</span>
            <h2>Points Breakdown</h2>
          </div>
          <div class="trigger-grid">
            <div class="trigger-card" *ngFor="let entry of triggerEntries()">
              <div class="trigger-label">{{ entry.label }}</div>
              <div class="trigger-points">{{ formatPointsDisplay(entry.points) }}</div>
            </div>
          </div>
        </div>

        <!-- Mastery Summary -->
        <div class="section" *ngIf="masteryStats() as stats">
          <div class="section-header">
            <span class="material-icons">school</span>
            <h2>Mastery Progress</h2>
          </div>
          <div class="mastery-strip">
            <div class="mastery-stat">
              <span class="stat-value">{{ stats.totalRecords }}</span>
              <span class="stat-label">Content Engaged</span>
            </div>
            <div class="mastery-stat">
              <span class="stat-value">{{ stats.masteredCount }}</span>
              <span class="stat-label">Mastered</span>
            </div>
            <div class="mastery-stat">
              <span class="stat-value">{{ stats.freshnessPercent }}%</span>
              <span class="stat-label">Fresh</span>
            </div>
          </div>
        </div>

        <!-- Activity Empty State -->
        <div class="empty-state" *ngIf="!pointsBalance() && !service.isLoading()">
          <span class="material-icons empty-icon">emoji_events</span>
          <h2>Start Learning to Earn Points</h2>
          <p>
            Engage with content, practice assessments, and complete paths to earn participation
            points. Your learning activity fuels the network.
          </p>
          <a routerLink="/lamad" class="action-link">
            <span class="material-icons">arrow_forward</span>
            Explore Learning Paths
          </a>
        </div>
      </div>

      <!-- ============================================================= -->
      <!-- DEVICES TAB                                                    -->
      <!-- ============================================================= -->
      <div class="tab-content" *ngIf="activeTab() === 'devices'">
        <!-- Upgrade Prompt (non-steward users) -->
        <div class="upgrade-prompt" *ngIf="!isSteward()">
          <div class="upgrade-icon-wrap">
            <span class="material-icons upgrade-icon">rocket_launch</span>
          </div>
          <h2>Become an App Steward</h2>
          <p class="upgrade-desc">
            App Stewards run a local Holochain conductor on their device, giving them true data
            ownership and offline access. Your keys live on your device, not on a server.
          </p>

          <!-- Stage Progression Visual -->
          <div class="stage-progression">
            <div
              class="stage-chip"
              *ngFor="let stage of stageList"
              [class.current]="stage.stage === currentStage()"
              [class.next]="stage.stage === nextStage()"
              [class.past]="stage.order < currentStageOrder()"
            >
              <span class="material-icons stage-icon">{{ stage.icon }}</span>
              <span class="stage-label">{{ stage.label }}</span>
            </div>
          </div>

          <div class="upgrade-benefits" *ngIf="nextStageInfo() as next">
            <h3>{{ next.label }} Benefits</h3>
            <ul>
              <li *ngFor="let benefit of next.benefits">{{ benefit }}</li>
            </ul>
          </div>

          <button class="upgrade-cta" disabled>
            <span class="material-icons">download</span>
            Download the Desktop App
          </button>
          <p class="cta-note">Coming soon</p>
        </div>

        <!-- Steward Device Content -->
        <ng-container *ngIf="isSteward()">
          <!-- Loading State -->
          <div class="loading-overlay" *ngIf="service.isLoading()">
            <div class="spinner"></div>
            <p>Discovering devices...</p>
          </div>

          <!-- Summary Strip -->
          <div class="summary-strip" *ngIf="!service.isLoading() && service.totalDevices() > 0">
            <div class="summary-badge connected">
              <span class="badge-dot"></span>
              {{ service.connectedCount() }} Connected
            </div>
            <div class="summary-badge seen" *ngIf="service.seenCount() > 0">
              <span class="badge-dot"></span>
              {{ service.seenCount() }} Seen
            </div>
            <div class="summary-badge offline" *ngIf="service.offlineCount() > 0">
              <span class="badge-dot"></span>
              {{ service.offlineCount() }} Offline
            </div>
            <div class="summary-total">{{ service.totalDevices() }} total</div>
          </div>

          <!-- Device Content -->
          <div class="device-content" *ngIf="!service.isLoading()">
            <!-- Current Device (highlighted) -->
            <div class="current-device-section" *ngIf="service.currentDevice() as current">
              <div class="device-card current">
                <div class="card-header">
                  <span class="material-icons device-icon">
                    {{ getPlatformDisplay(current).icon }}
                  </span>
                  <div class="card-title">
                    <h3>{{ current.displayName }}</h3>
                    <span class="current-badge">You are here</span>
                  </div>
                  <span
                    class="status-indicator"
                    [style.background]="getStatusDisplay(current.status).color"
                  ></span>
                </div>
                <div class="card-details">
                  <div class="detail-row">
                    <span class="detail-label">Platform</span>
                    <span class="detail-value">{{ getPlatformDisplay(current).label }}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">Category</span>
                    <span class="detail-value">{{ getCategoryDisplay(current).label }}</span>
                  </div>
                  <div class="detail-row">
                    <span class="detail-label">Agency</span>
                    <span class="detail-value agency-badge">{{ agencyLabel() }}</span>
                  </div>
                  <div class="detail-row" *ngIf="current.doorwayUrl">
                    <span class="detail-label">Conductor</span>
                    <span class="detail-value mono">{{ current.doorwayUrl }}</span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Node Steward Section -->
            <div class="section" *ngIf="service.nodeStewardDevices().length > 0">
              <div class="section-header">
                <span class="material-icons">dns</span>
                <h2>Infrastructure Nodes</h2>
                <span class="section-count">{{ service.nodeStewardDevices().length }}</span>
              </div>

              <div class="device-grid">
                <div
                  class="device-card"
                  *ngFor="let device of service.nodeStewardDevices()"
                  [class.current]="device.isCurrentDevice"
                >
                  <div class="card-header">
                    <span class="material-icons device-icon">
                      {{ getNodeIcon(device) }}
                    </span>
                    <div class="card-title">
                      <h3>{{ device.displayName }}</h3>
                      <span class="node-type">{{ getNodeLabel(device) }}</span>
                    </div>
                    <span
                      class="status-indicator"
                      [style.background]="getStatusDisplay(device.status).color"
                      [title]="getStatusDisplay(device.status).label"
                    ></span>
                  </div>

                  <div class="card-details">
                    <div class="detail-row" *ngIf="device.location">
                      <span class="detail-label">Location</span>
                      <span class="detail-value">{{ device.location!.label }}</span>
                    </div>

                    <!-- Resource Bars -->
                    <div class="resource-bars" *ngIf="device.resources">
                      <div class="resource-bar">
                        <span class="bar-label">CPU</span>
                        <div class="bar-track">
                          <div
                            class="bar-fill"
                            [style.width.%]="device.resources!.cpuPercent"
                            [class.high]="device.resources!.cpuPercent > 80"
                          ></div>
                        </div>
                        <span class="bar-value">{{ device.resources!.cpuPercent }}%</span>
                      </div>
                      <div class="resource-bar">
                        <span class="bar-label">Mem</span>
                        <div class="bar-track">
                          <div
                            class="bar-fill"
                            [style.width.%]="device.resources!.memoryPercent"
                            [class.high]="device.resources!.memoryPercent > 80"
                          ></div>
                        </div>
                        <span class="bar-value">{{ device.resources!.memoryPercent }}%</span>
                      </div>
                      <div class="resource-bar">
                        <span class="bar-label">Disk</span>
                        <div class="bar-track">
                          <div
                            class="bar-fill"
                            [style.width.%]="storagePercent(device)"
                            [class.high]="storagePercent(device) > 80"
                          ></div>
                        </div>
                        <span class="bar-value">
                          {{ device.resources!.storageUsedGB }}/{{
                            device.resources!.storageTotalGB
                          }}GB
                        </span>
                      </div>
                    </div>

                    <!-- Roles -->
                    <div class="roles-row" *ngIf="device.roles && device.roles.length > 0">
                      <span class="role-chip" *ngFor="let role of device.roles">
                        {{ role.role }}
                      </span>
                    </div>

                    <div class="detail-row" *ngIf="device.isPrimaryNode">
                      <span class="primary-badge">Primary Node</span>
                    </div>
                  </div>
                </div>
              </div>
            </div>

            <!-- App Steward Section (non-current devices, for future multi-device) -->
            <div class="section" *ngIf="nonCurrentAppDevices().length > 0">
              <div class="section-header">
                <span class="material-icons">smartphone</span>
                <h2>Personal Devices</h2>
                <span class="section-count">{{ nonCurrentAppDevices().length }}</span>
              </div>
              <div class="device-grid">
                <div class="device-card" *ngFor="let device of nonCurrentAppDevices()">
                  <div class="card-header">
                    <span class="material-icons device-icon">
                      {{ getPlatformDisplay(device).icon }}
                    </span>
                    <div class="card-title">
                      <h3>{{ device.displayName }}</h3>
                    </div>
                    <span
                      class="status-indicator"
                      [style.background]="getStatusDisplay(device.status).color"
                    ></span>
                  </div>
                </div>
              </div>
            </div>

            <!-- Empty State -->
            <div class="empty-state" *ngIf="service.totalDevices() === 0 && !service.error()">
              <span class="material-icons empty-icon">devices</span>
              <h2>No Stewarded Devices</h2>
              <p>
                Devices appear here once you progress beyond hosted access. As an
                <strong>App Steward</strong>
                , your desktop app runs a local Holochain conductor. As a
                <strong>Node Steward</strong>
                , you run always-on infrastructure for the network.
              </p>
              <a routerLink="/imagodei/agency" class="action-link">
                <span class="material-icons">arrow_forward</span>
                View Agency Progression
              </a>
            </div>
          </div>
        </ng-container>
      </div>

      <!-- Error Banner -->
      <div class="error-banner" *ngIf="service.error()">
        <span class="material-icons">warning</span>
        {{ service.error() }}
        <button class="dismiss-btn" (click)="refresh()">Retry</button>
      </div>
    </div>
  `,
  styles: [
    `
      .device-stewardship {
        max-width: 1200px;
        margin: 0 auto;
        padding: 2rem;
      }

      /* Header */
      .page-header {
        display: flex;
        justify-content: space-between;
        align-items: flex-start;
        margin-bottom: 1.5rem;
        flex-wrap: wrap;
        gap: 1rem;
      }

      .header-content h1 {
        font-size: 2rem;
        margin: 0;
        color: var(--lamad-text-primary, #f8fafc);
      }

      .subtitle {
        color: var(--lamad-text-muted, #64748b);
        margin: 0.25rem 0 0 0;
      }

      .refresh-btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.5rem 1rem;
        background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.2));
        color: var(--lamad-text-secondary, #e2e8f0);
        border-radius: 8px;
        font-size: 0.875rem;
        cursor: pointer;
        transition: all 0.2s;
      }

      .refresh-btn:hover:not(:disabled) {
        background: var(--lamad-accent-primary, #6366f1);
        border-color: var(--lamad-accent-primary, #6366f1);
        color: white;
      }

      .refresh-btn:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }

      .refresh-btn .material-icons {
        font-size: 1.125rem;
      }

      /* Tab Bar */
      .tab-bar {
        display: flex;
        gap: 0;
        border-bottom: 2px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        margin-bottom: 1.5rem;
      }

      .tab-btn {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.25rem;
        background: none;
        border: none;
        border-bottom: 2px solid transparent;
        margin-bottom: -2px;
        color: var(--lamad-text-muted, #64748b);
        font-size: 0.9375rem;
        font-weight: 500;
        cursor: pointer;
        transition: all 0.2s;
      }

      .tab-btn:hover {
        color: var(--lamad-text-secondary, #e2e8f0);
      }

      .tab-btn.active {
        color: var(--lamad-accent-primary, #6366f1);
        border-bottom-color: var(--lamad-accent-primary, #6366f1);
      }

      .tab-icon {
        font-size: 1.125rem;
      }

      .lock-icon {
        display: inline-flex;
        align-items: center;
      }

      .lock-icon .material-icons {
        font-size: 0.875rem;
        opacity: 0.5;
      }

      .tab-content {
        min-height: 200px;
      }

      /* Points Strip */
      .points-strip,
      .mastery-strip {
        display: flex;
        gap: 1.5rem;
        margin-bottom: 2rem;
        flex-wrap: wrap;
      }

      .points-stat,
      .mastery-stat {
        display: flex;
        flex-direction: column;
        align-items: center;
        background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        border-radius: 12px;
        padding: 1.25rem 2rem;
        flex: 1;
        min-width: 140px;
      }

      .stat-value {
        font-size: 1.75rem;
        font-weight: 700;
        color: var(--lamad-accent-primary, #6366f1);
        line-height: 1;
        margin-bottom: 0.375rem;
      }

      .stat-label {
        font-size: 0.8125rem;
        color: var(--lamad-text-muted, #64748b);
      }

      /* Trigger Grid */
      .trigger-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(180px, 1fr));
        gap: 0.75rem;
      }

      .trigger-card {
        background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
        border-radius: 8px;
        padding: 0.875rem 1rem;
        display: flex;
        justify-content: space-between;
        align-items: center;
      }

      .trigger-label {
        font-size: 0.8125rem;
        color: var(--lamad-text-secondary, #e2e8f0);
      }

      .trigger-points {
        font-size: 0.9375rem;
        font-weight: 600;
        color: var(--lamad-accent-primary, #6366f1);
      }

      /* Upgrade Prompt */
      .upgrade-prompt {
        text-align: center;
        padding: 3rem 2rem;
        max-width: 560px;
        margin: 0 auto;
      }

      .upgrade-icon-wrap {
        margin-bottom: 1rem;
      }

      .upgrade-icon {
        font-size: 3rem;
        color: var(--lamad-accent-primary, #6366f1);
        opacity: 0.8;
      }

      .upgrade-prompt h2 {
        color: var(--lamad-text-primary, #f8fafc);
        font-size: 1.375rem;
        margin: 0 0 0.75rem;
      }

      .upgrade-desc {
        color: var(--lamad-text-muted, #64748b);
        font-size: 0.875rem;
        line-height: 1.6;
        margin: 0 0 1.5rem;
      }

      /* Stage Progression */
      .stage-progression {
        display: flex;
        gap: 0.5rem;
        justify-content: center;
        margin-bottom: 1.5rem;
        flex-wrap: wrap;
      }

      .stage-chip {
        display: flex;
        align-items: center;
        gap: 0.375rem;
        padding: 0.375rem 0.75rem;
        border-radius: 20px;
        font-size: 0.8125rem;
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        color: var(--lamad-text-muted, #64748b);
        background: var(--lamad-surface, rgba(30, 30, 46, 0.5));
      }

      .stage-chip .stage-icon {
        font-size: 1rem;
      }

      .stage-chip.current {
        border-color: var(--lamad-accent-primary, #6366f1);
        color: var(--lamad-accent-primary, #6366f1);
        background: rgba(99, 102, 241, 0.1);
      }

      .stage-chip.next {
        border-color: #22c55e;
        color: #22c55e;
        background: rgba(34, 197, 94, 0.08);
      }

      .stage-chip.past {
        opacity: 0.4;
      }

      .upgrade-benefits {
        text-align: left;
        background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.15));
        border-radius: 12px;
        padding: 1rem 1.25rem;
        margin-bottom: 1.5rem;
      }

      .upgrade-benefits h3 {
        margin: 0 0 0.5rem;
        font-size: 0.9375rem;
        color: var(--lamad-text-primary, #f8fafc);
      }

      .upgrade-benefits ul {
        margin: 0;
        padding-left: 1.25rem;
      }

      .upgrade-benefits li {
        font-size: 0.8125rem;
        color: var(--lamad-text-secondary, #e2e8f0);
        margin-bottom: 0.25rem;
        line-height: 1.5;
      }

      .upgrade-cta {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.75rem 1.5rem;
        background: var(--lamad-accent-primary, #6366f1);
        color: white;
        border: none;
        border-radius: 8px;
        font-size: 0.9375rem;
        font-weight: 600;
        cursor: pointer;
      }

      .upgrade-cta:disabled {
        opacity: 0.5;
        cursor: not-allowed;
      }

      .cta-note {
        font-size: 0.75rem;
        color: var(--lamad-text-muted, #64748b);
        margin: 0.5rem 0 0;
      }

      /* Loading */
      .loading-overlay {
        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;
        padding: 4rem;
        color: var(--lamad-text-muted, #64748b);
      }

      .spinner {
        width: 40px;
        height: 40px;
        border: 3px solid var(--lamad-border, rgba(99, 102, 241, 0.2));
        border-top-color: var(--lamad-accent-primary, #6366f1);
        border-radius: 50%;
        animation: spin 1s linear infinite;
        margin-bottom: 1rem;
      }

      @keyframes spin {
        to {
          transform: rotate(360deg);
        }
      }

      /* Summary Strip */
      .summary-strip {
        display: flex;
        align-items: center;
        gap: 1rem;
        margin-bottom: 2rem;
        flex-wrap: wrap;
      }

      .summary-badge {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        padding: 0.375rem 0.75rem;
        border-radius: 20px;
        font-size: 0.875rem;
        font-weight: 500;
      }

      .summary-badge.connected {
        background: rgba(34, 197, 94, 0.1);
        color: #22c55e;
      }

      .summary-badge.seen {
        background: rgba(245, 158, 11, 0.1);
        color: #f59e0b;
      }

      .summary-badge.offline {
        background: rgba(239, 68, 68, 0.1);
        color: #ef4444;
      }

      .badge-dot {
        width: 8px;
        height: 8px;
        border-radius: 50%;
        background: currentColor;
      }

      .summary-total {
        margin-left: auto;
        font-size: 0.875rem;
        color: var(--lamad-text-muted, #64748b);
      }

      /* Sections */
      .section {
        margin-bottom: 2rem;
      }

      .section-header {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        margin-bottom: 1rem;
      }

      .section-header .material-icons {
        color: var(--lamad-accent-primary, #6366f1);
        font-size: 1.25rem;
      }

      .section-header h2 {
        margin: 0;
        font-size: 1.125rem;
        font-weight: 600;
        color: var(--lamad-text-primary, #f8fafc);
      }

      .section-count {
        font-size: 0.75rem;
        color: var(--lamad-text-muted, #64748b);
        background: var(--lamad-bg-primary, #0f0f1a);
        padding: 0.125rem 0.5rem;
        border-radius: 10px;
      }

      /* Device Grid */
      .device-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(340px, 1fr));
        gap: 1rem;
      }

      /* Device Card */
      .device-card {
        background: var(--lamad-surface, rgba(30, 30, 46, 0.8));
        border: 1px solid var(--lamad-border, rgba(99, 102, 241, 0.1));
        border-radius: 12px;
        padding: 1.25rem;
        transition:
          transform 0.2s,
          border-color 0.2s;
      }

      .device-card:hover {
        transform: translateY(-2px);
        border-color: var(--lamad-accent-primary, #6366f1);
      }

      .device-card.current {
        border-color: var(--lamad-accent-primary, #6366f1);
        background: linear-gradient(
          135deg,
          rgba(99, 102, 241, 0.08) 0%,
          var(--lamad-surface, rgba(30, 30, 46, 0.8)) 100%
        );
      }

      .current-device-section {
        margin-bottom: 2rem;
      }

      .card-header {
        display: flex;
        align-items: center;
        gap: 0.75rem;
        margin-bottom: 1rem;
      }

      .device-icon {
        font-size: 1.5rem;
        color: var(--lamad-accent-primary, #6366f1);
        background: var(--lamad-bg-primary, rgba(15, 15, 26, 0.5));
        width: 40px;
        height: 40px;
        display: flex;
        align-items: center;
        justify-content: center;
        border-radius: 10px;
        flex-shrink: 0;
      }

      .card-title {
        flex: 1;
        min-width: 0;
      }

      .card-title h3 {
        margin: 0;
        font-size: 1rem;
        font-weight: 600;
        color: var(--lamad-text-primary, #f8fafc);
      }

      .current-badge {
        display: inline-block;
        font-size: 0.6875rem;
        color: var(--lamad-accent-primary, #6366f1);
        background: rgba(99, 102, 241, 0.15);
        padding: 0.125rem 0.5rem;
        border-radius: 4px;
        margin-top: 0.25rem;
      }

      .node-type {
        font-size: 0.75rem;
        color: var(--lamad-text-muted, #64748b);
      }

      .status-indicator {
        width: 10px;
        height: 10px;
        border-radius: 50%;
        flex-shrink: 0;
      }

      /* Card Details */
      .card-details {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      .detail-row {
        display: flex;
        justify-content: space-between;
        align-items: center;
        font-size: 0.8125rem;
      }

      .detail-label {
        color: var(--lamad-text-muted, #64748b);
      }

      .detail-value {
        color: var(--lamad-text-secondary, #e2e8f0);
        font-weight: 500;
      }

      .detail-value.mono {
        font-family: monospace;
        font-size: 0.75rem;
      }

      .agency-badge {
        color: var(--lamad-accent-primary, #6366f1);
      }

      .primary-badge {
        font-size: 0.75rem;
        color: #f59e0b;
        background: rgba(245, 158, 11, 0.1);
        padding: 0.125rem 0.5rem;
        border-radius: 4px;
      }

      /* Resource Bars */
      .resource-bars {
        display: flex;
        flex-direction: column;
        gap: 0.375rem;
        margin-top: 0.25rem;
      }

      .resource-bar {
        display: flex;
        align-items: center;
        gap: 0.5rem;
        font-size: 0.75rem;
      }

      .bar-label {
        width: 28px;
        color: var(--lamad-text-muted, #64748b);
        flex-shrink: 0;
      }

      .bar-track {
        flex: 1;
        height: 6px;
        background: var(--lamad-bg-primary, rgba(15, 15, 26, 0.5));
        border-radius: 3px;
        overflow: hidden;
      }

      .bar-fill {
        height: 100%;
        background: var(--lamad-accent-primary, #6366f1);
        border-radius: 3px;
        transition: width 0.3s ease;
      }

      .bar-fill.high {
        background: #ef4444;
      }

      .bar-value {
        width: 60px;
        text-align: right;
        color: var(--lamad-text-muted, #64748b);
        flex-shrink: 0;
      }

      /* Roles */
      .roles-row {
        display: flex;
        gap: 0.375rem;
        flex-wrap: wrap;
        margin-top: 0.25rem;
      }

      .role-chip {
        font-size: 0.6875rem;
        color: var(--lamad-text-secondary, #e2e8f0);
        background: var(--lamad-bg-primary, rgba(15, 15, 26, 0.5));
        padding: 0.125rem 0.5rem;
        border-radius: 4px;
        text-transform: capitalize;
      }

      /* Empty State */
      .empty-state {
        text-align: center;
        padding: 4rem 2rem;
        color: var(--lamad-text-muted, #64748b);
      }

      .empty-state .empty-icon {
        font-size: 4rem;
        opacity: 0.3;
        display: block;
        margin-bottom: 1rem;
      }

      .empty-state h2 {
        color: var(--lamad-text-primary, #f8fafc);
        margin: 0 0 0.75rem;
        font-size: 1.25rem;
      }

      .empty-state p {
        max-width: 480px;
        margin: 0 auto 1.5rem;
        line-height: 1.6;
        font-size: 0.875rem;
      }

      .action-link {
        display: inline-flex;
        align-items: center;
        gap: 0.5rem;
        color: var(--lamad-accent-primary, #6366f1);
        text-decoration: none;
        font-weight: 500;
        font-size: 0.875rem;
        transition: opacity 0.2s;
      }

      .action-link:hover {
        opacity: 0.8;
      }

      .action-link .material-icons {
        font-size: 1.125rem;
      }

      /* Error Banner */
      .error-banner {
        position: fixed;
        bottom: 2rem;
        left: 50%;
        transform: translateX(-50%);
        background: rgba(239, 68, 68, 0.9);
        color: white;
        padding: 0.75rem 1.25rem;
        border-radius: 8px;
        display: flex;
        align-items: center;
        gap: 0.75rem;
        box-shadow: 0 4px 12px rgba(0, 0, 0, 0.3);
        z-index: 1000;
        font-size: 0.875rem;
      }

      .error-banner .material-icons {
        font-size: 1.25rem;
      }

      .dismiss-btn {
        background: rgba(255, 255, 255, 0.2);
        border: none;
        color: white;
        padding: 0.25rem 0.75rem;
        border-radius: 4px;
        cursor: pointer;
        font-size: 0.75rem;
      }

      .dismiss-btn:hover {
        background: rgba(255, 255, 255, 0.3);
      }

      @media (max-width: 600px) {
        .device-stewardship {
          padding: 1rem;
        }

        .device-grid {
          grid-template-columns: 1fr;
        }

        .points-strip,
        .mastery-strip {
          flex-direction: column;
        }

        .trigger-grid {
          grid-template-columns: 1fr;
        }
      }
    `,
  ],
})
export class DeviceStewardshipComponent implements OnInit {
  readonly service = inject(DeviceStewardshipService);
  private readonly identityService = inject(IdentityService);
  private readonly pointsService = inject(PointsService);
  private readonly masteryService = inject(MasteryService);

  // Tab state
  readonly activeTab = signal<'activity' | 'devices'>('activity');

  // Agency
  readonly isSteward = computed(() => {
    const stage = this.identityService.identity().agencyStage;
    return stage === 'app-steward' || stage === 'node-steward';
  });

  readonly currentStage = computed(() => this.identityService.identity().agencyStage);

  readonly currentStageOrder = computed(() => AGENCY_STAGES[this.currentStage()].order);

  readonly nextStage = computed(() => getNextStage(this.currentStage()));

  readonly nextStageInfo = computed(() => {
    const next = this.nextStage();
    return next ? AGENCY_STAGES[next] : null;
  });

  readonly stageList = Object.values(AGENCY_STAGES);

  readonly agencyLabel = computed(() => {
    const stage = this.identityService.identity().agencyStage;
    const labels: Record<string, string> = {
      visitor: 'Visitor',
      hosted: 'Hosted',
      'app-steward': 'App Steward',
      'node-steward': 'Node Steward',
    };
    return labels[stage] ?? stage;
  });

  readonly nonCurrentAppDevices = computed(() =>
    this.service.appStewardDevices().filter(d => !d.isCurrentDevice)
  );

  // Points state
  readonly pointsBalance = signal<{
    total_points: number;
    total_earned: number;
    points_by_trigger_json: string;
  } | null>(null);

  readonly weeklyPoints = signal(0);

  readonly triggerEntries = computed(() => {
    const balance = this.pointsBalance();
    if (!balance) return [];
    const byTrigger = this.pointsService.getPointsByTriggerSync();
    return Object.entries(byTrigger)
      .filter(([, pts]) => pts > 0)
      .map(([trigger, pts]) => ({
        trigger: trigger as LamadPointTrigger,
        label: getTriggerLabel(trigger as LamadPointTrigger),
        points: pts,
      }))
      .sort((a, b) => b.points - a.points);
  });

  // Mastery state
  readonly masteryStats = signal<{
    totalRecords: number;
    masteredCount: number;
    freshnessPercent: number;
  } | null>(null);

  ngOnInit(): void {
    void this.service.loadDevices();
    this.loadPointsData();
    this.loadMasteryData();
  }

  refresh(): void {
    this.ngOnInit();
  }

  selectTab(tab: 'activity' | 'devices'): void {
    this.activeTab.set(tab);
  }

  formatPointsDisplay(points: number): string {
    return formatPoints(points);
  }

  // Device display helpers
  getStatusDisplay(status: DeviceStatus) {
    return getDeviceStatusDisplay(status);
  }

  getCategoryDisplay(device: StewardedDevice) {
    return getDeviceCategoryDisplay(device.category);
  }

  getPlatformDisplay(device: StewardedDevice) {
    return getDevicePlatformDisplay(device.platform ?? 'unknown');
  }

  getNodeIcon(device: StewardedDevice): string {
    return device.nodeType ? getNodeTypeDisplay(device.nodeType).icon : 'dns';
  }

  getNodeLabel(device: StewardedDevice): string {
    return device.nodeType ? getNodeTypeDisplay(device.nodeType).label : 'Node';
  }

  storagePercent(device: StewardedDevice): number {
    const r = device.resources;
    if (!r || r.storageTotalGB === 0) return 0;
    return Math.round((r.storageUsedGB / r.storageTotalGB) * 100);
  }

  // Private loading methods
  private loadPointsData(): void {
    this.pointsService.getBalance$().subscribe(balance => {
      this.pointsBalance.set(balance);
      if (balance) {
        this.weeklyPoints.set(this.pointsService.getRecentPointsEarned(7));
      }
    });

    this.pointsService.loadHistory().subscribe();
  }

  private loadMasteryData(): void {
    const humanId = this.identityService.identity().humanId;
    if (!humanId) return;

    this.masteryService.getMasteryForHuman(humanId).subscribe(records => {
      const masteredCount = records.filter(r => r.masteryLevel === 'mastered').length;
      const freshCount = records.filter(r => !r.needsRefresh).length;
      const freshnessPercent =
        records.length > 0 ? Math.round((freshCount / records.length) * 100) : 0;

      this.masteryStats.set({
        totalRecords: records.length,
        masteredCount,
        freshnessPercent,
      });
    });
  }
}
