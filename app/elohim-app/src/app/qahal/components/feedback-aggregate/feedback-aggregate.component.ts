import { CommonModule } from '@angular/common';
import { Component, OnInit, inject, input, signal } from '@angular/core';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';
import type { SignalAggregateView } from '@elohim/storage-client/generated';

/**
 * FeedbackAggregateComponent — Visualizes aggregated governance signals.
 *
 * Displays total signals, by-type breakdown, by-value distribution bars,
 * and consensus strength meter. Pure CSS — no charting library.
 */
@Component({
  selector: 'app-feedback-aggregate',
  standalone: true,
  imports: [CommonModule],
  template: `
    @if (aggregate()) {
      <div class="feedback-aggregate">
        <div class="aggregate-summary">
          {{ toNumber(aggregate()!.totalSignals) }} signals from
          {{ toNumber(aggregate()!.uniqueParticipants) }} participants
        </div>

        @if (typeBreakdown().length > 0) {
          <div class="type-breakdown">
            {{ typeBreakdownText() }}
          </div>
        }

        @if (valueDistribution().length > 0) {
          <div class="value-distribution">
            @for (entry of valueDistribution(); track entry.label) {
              <div class="value-row">
                <span class="value-label">{{ entry.label }}</span>
                <div class="value-bar-track">
                  <div
                    class="value-bar-fill"
                    [style.width.%]="entry.percentage"
                  ></div>
                </div>
                <span class="value-count">
                  {{ entry.count }} ({{ entry.percentage | number: '1.0-0' }}%)
                </span>
              </div>
            }
          </div>
        }

        <div class="consensus-section">
          <div class="consensus-label">{{ consensusLabel() }}</div>
          <div class="consensus-meter-track">
            <div
              class="consensus-meter-fill"
              [style.width.%]="aggregate()!.consensusStrength * 100"
            ></div>
          </div>
        </div>
      </div>
    }
  `,
  styles: [
    `
      .feedback-aggregate {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        font-size: 0.875rem;
      }

      .aggregate-summary {
        font-weight: 600;
      }

      .type-breakdown {
        color: var(--text-secondary, #666);
      }

      .value-distribution {
        display: flex;
        flex-direction: column;
        gap: 0.375rem;
      }

      .value-row {
        display: flex;
        align-items: center;
        gap: 0.5rem;
      }

      .value-label {
        min-width: 5rem;
        text-transform: capitalize;
      }

      .value-bar-track {
        flex: 1;
        height: 0.5rem;
        background: var(--surface-variant, #e0e0e0);
        border-radius: 0.25rem;
        overflow: hidden;
      }

      .value-bar-fill {
        height: 100%;
        background: var(--primary, #4a6fa5);
        border-radius: 0.25rem;
        transition: width 0.3s ease;
      }

      .value-count {
        min-width: 5rem;
        text-align: right;
        font-size: 0.75rem;
        color: var(--text-secondary, #666);
      }

      .consensus-section {
        display: flex;
        flex-direction: column;
        gap: 0.25rem;
      }

      .consensus-label {
        font-size: 0.75rem;
        font-weight: 500;
        color: var(--text-secondary, #666);
      }

      .consensus-meter-track {
        height: 0.375rem;
        background: var(--surface-variant, #e0e0e0);
        border-radius: 0.1875rem;
        overflow: hidden;
      }

      .consensus-meter-fill {
        height: 100%;
        background: linear-gradient(90deg, #a8d5a2, #4caf50);
        border-radius: 0.1875rem;
        transition: width 0.3s ease;
      }
    `,
  ],
})
export class FeedbackAggregateComponent implements OnInit {
  entityType = input.required<string>();
  entityId = input.required<string>();

  aggregate = signal<SignalAggregateView | null>(null);

  private readonly governanceApi = inject(GovernanceApiService);

  async ngOnInit(): Promise<void> {
    const result = await this.governanceApi.getSignalAggregate(
      this.entityType(),
      this.entityId()
    );
    this.aggregate.set(result);
  }

  toNumber(value: bigint): number {
    return Number(value);
  }

  typeBreakdown(): { label: string; count: number }[] {
    const agg = this.aggregate();
    if (!agg) return [];
    return Object.entries(agg.byType)
      .filter(([, v]) => v !== undefined)
      .map(([label, count]) => ({ label, count: Number(count!) }));
  }

  typeBreakdownText(): string {
    return this.typeBreakdown()
      .map((e) => `${e.count} ${e.label}`)
      .join(' \u00b7 ');
  }

  valueDistribution(): { label: string; count: number; percentage: number }[] {
    const agg = this.aggregate();
    if (!agg) return [];
    const total = Number(agg.totalSignals);
    if (total === 0) return [];
    return Object.entries(agg.byValue)
      .filter(([, v]) => v !== undefined)
      .map(([label, count]) => {
        const n = Number(count!);
        return { label, count: n, percentage: (n / total) * 100 };
      });
  }

  consensusLabel(): string {
    const agg = this.aggregate();
    if (!agg) return '';
    const strength = agg.consensusStrength;
    if (strength < 0.3) return 'Diverse views';
    if (strength <= 0.7) return 'Mixed opinions';
    return 'Strong consensus';
  }
}
