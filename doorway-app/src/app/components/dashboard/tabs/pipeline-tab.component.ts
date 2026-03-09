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
          <button (click)="loadPipeline()" data-testid="pipeline-retry">Retry</button>
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
  styleUrl: './pipeline-tab.component.css',
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
