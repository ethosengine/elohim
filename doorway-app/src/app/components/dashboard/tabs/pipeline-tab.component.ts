/**
 * Pipeline Tab Component
 *
 * Horizontal funnel showing agency pipeline stage counts:
 * Registered -> Hosted -> Graduating -> Steward
 */

import { Component, OnInit, inject, signal, computed } from '@angular/core';
import { CommonModule } from '@angular/common';
import { DoorwayAdminService } from '../../../services/doorway-admin.service';
import {
  PipelineResponse,
  PipelineStage,
  pipelineStageColor,
  pipelineStageName,
} from '../../../models/doorway.model';

interface FunnelStage {
  key: PipelineStage;
  label: string;
  count: number;
  color: string;
  widthPercent: number;
}

@Component({
  selector: 'app-pipeline-tab',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div class="pipeline-tab">
      @if (loading()) {
        <div class="loading-state">
          <div class="spinner"></div>
          <p>Loading pipeline...</p>
        </div>
      } @else if (error()) {
        <div class="error-state">
          <p>{{ error() }}</p>
          <button (click)="loadPipeline()">Retry</button>
        </div>
      } @else {
        <!-- Total -->
        <div class="pipeline-total">
          <span class="total-value">{{ total() }}</span>
          <span class="total-label">Total Users in Pipeline</span>
        </div>

        <!-- Funnel -->
        <div class="funnel">
          @for (stage of stages(); track stage.key) {
            <div class="funnel-stage">
              <div class="funnel-bar-wrapper">
                <div
                  class="funnel-bar"
                  [style.width.%]="stage.widthPercent"
                  [style.background]="stage.color">
                  <span class="funnel-count">{{ stage.count }}</span>
                </div>
              </div>
              <span class="funnel-label">{{ stage.label }}</span>
            </div>
          }
        </div>

        <!-- Conversion rates -->
        @if (total() > 0) {
          <div class="conversions">
            <h3>Conversion Rates</h3>
            <div class="conversion-grid">
              @for (conv of conversions(); track conv.label) {
                <div class="conversion-item">
                  <span class="conv-value">{{ conv.rate | number:'1.1-1' }}%</span>
                  <span class="conv-label">{{ conv.label }}</span>
                </div>
              }
            </div>
          </div>
        }
      }
    </div>
  `,
  styles: [`
    .pipeline-tab {
      padding: 1rem 0;
    }

    .loading-state, .error-state {
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

    .error-state button {
      margin-top: 1rem;
      padding: 0.5rem 1rem;
      background: #ef4444;
      color: white;
      border: none;
      border-radius: 0.375rem;
      cursor: pointer;
    }

    .pipeline-total {
      text-align: center;
      margin-bottom: 2rem;

      .total-value {
        display: block;
        font-size: 2.5rem;
        font-weight: 700;
        color: var(--text-primary, #111827);
      }

      .total-label {
        font-size: 0.875rem;
        color: var(--text-secondary, #6b7280);
      }
    }

    .funnel {
      display: flex;
      flex-direction: column;
      gap: 1rem;
      max-width: 600px;
      margin: 0 auto 2rem;
    }

    .funnel-stage {
      display: flex;
      align-items: center;
      gap: 1rem;
    }

    .funnel-bar-wrapper {
      flex: 1;
      display: flex;
      justify-content: center;
    }

    .funnel-bar {
      height: 40px;
      border-radius: 6px;
      display: flex;
      align-items: center;
      justify-content: center;
      min-width: 60px;
      transition: width 0.5s ease;
    }

    .funnel-count {
      font-size: 1rem;
      font-weight: 600;
      color: white;
      text-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
    }

    .funnel-label {
      width: 100px;
      font-size: 0.875rem;
      font-weight: 500;
      color: var(--text-secondary, #6b7280);
    }

    .conversions {
      max-width: 600px;
      margin: 0 auto;

      h3 {
        font-size: 0.875rem;
        font-weight: 600;
        color: var(--text-secondary, #6b7280);
        margin: 0 0 1rem;
        text-transform: uppercase;
        letter-spacing: 0.05em;
      }
    }

    .conversion-grid {
      display: grid;
      grid-template-columns: repeat(3, 1fr);
      gap: 1rem;
    }

    .conversion-item {
      display: flex;
      flex-direction: column;
      align-items: center;
      padding: 1rem;
      background: var(--bg-secondary, #f9fafb);
      border-radius: 0.5rem;

      .conv-value {
        font-size: 1.25rem;
        font-weight: 600;
        color: var(--text-primary, #111827);
      }

      .conv-label {
        font-size: 0.75rem;
        color: var(--text-secondary, #6b7280);
        text-align: center;
        margin-top: 0.25rem;
      }
    }

    @media (prefers-color-scheme: dark) {
      .conversion-item {
        background: #374151;
      }
    }
  `],
})
export class PipelineTabComponent implements OnInit {
  private readonly adminService = inject(DoorwayAdminService);

  readonly loading = signal(true);
  readonly error = signal<string | null>(null);
  readonly pipeline = signal<PipelineResponse | null>(null);

  readonly total = computed(() => {
    const p = this.pipeline();
    if (!p) return 0;
    return p.registered + p.hosted + p.graduating + p.steward;
  });

  readonly stages = computed((): FunnelStage[] => {
    const p = this.pipeline();
    if (!p) return [];
    const max = Math.max(p.registered, p.hosted, p.graduating, p.steward, 1);
    const stageKeys: PipelineStage[] = ['registered', 'hosted', 'graduating', 'steward'];
    return stageKeys.map(key => ({
      key,
      label: pipelineStageName(key),
      count: p[key],
      color: pipelineStageColor(key),
      widthPercent: Math.max((p[key] / max) * 100, 10),
    }));
  });

  readonly conversions = computed(() => {
    const p = this.pipeline();
    if (!p) return [];
    const safe = (a: number, b: number) => b > 0 ? (a / b) * 100 : 0;
    return [
      { label: 'Registered to Hosted', rate: safe(p.hosted, p.registered) },
      { label: 'Hosted to Graduating', rate: safe(p.graduating, p.hosted) },
      { label: 'Graduating to Steward', rate: safe(p.steward, p.graduating) },
    ];
  });

  ngOnInit(): void {
    this.loadPipeline();
  }

  async loadPipeline(): Promise<void> {
    this.loading.set(true);
    this.error.set(null);

    try {
      const result = await this.adminService.getPipeline().toPromise();
      if (result) {
        this.pipeline.set(result);
      }
    } catch {
      this.error.set('Failed to load pipeline data');
    } finally {
      this.loading.set(false);
    }
  }
}
