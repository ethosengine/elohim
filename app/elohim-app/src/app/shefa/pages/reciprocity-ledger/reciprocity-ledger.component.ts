import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  OnDestroy,
  OnInit,
  inject,
  signal,
} from '@angular/core';

import { CommitmentBarComponent } from '../../../elohim/components/commitment-bar/commitment-bar.component';
import { ReciprocityService } from '../../services/reciprocity.service';

@Component({
  selector: 'shefa-reciprocity-ledger',
  standalone: true,
  imports: [CommonModule, CommitmentBarComponent],
  template: `
    <section class="reciprocity" data-testid="reciprocity-page">
      <header><h2>Reciprocity</h2></header>

      @if (recip.reciprocity(); as v) {
        <h3>Inflow (others committed to host my content)</h3>
        @if (v.inflow.length === 0) {
          <p class="empty" data-testid="reciprocity-inflow-empty">No inflow commitments yet.</p>
        }
        <ul class="rows">
          @for (row of v.inflow; track row.counterpartyHouseholdId) {
            <li data-testid="reciprocity-inflow-row">
              <strong>{{ row.displayName ?? row.counterpartyHouseholdId }}</strong> —
              {{ formatBytes(row.committedBytes) }} committed,
              {{ formatBytes(row.deliveredBytes) }} delivered
              <elohim-commitment-bar
                [committedBytes]="row.committedBytes"
                [deliveredBytes]="row.deliveredBytes"
              ></elohim-commitment-bar>
            </li>
          }
        </ul>

        <h3>Outflow (I committed to host others' content)</h3>
        @if (v.outflow.length === 0) {
          <p class="empty" data-testid="reciprocity-outflow-empty">No outflow commitments yet.</p>
        }
        <ul class="rows">
          @for (row of v.outflow; track row.counterpartyHouseholdId) {
            <li data-testid="reciprocity-outflow-row">
              <strong>{{ row.displayName ?? row.counterpartyHouseholdId }}</strong> —
              {{ formatBytes(row.committedBytes) }} committed,
              {{ formatBytes(row.deliveredBytes) }} hosting
              <elohim-commitment-bar
                [committedBytes]="row.committedBytes"
                [deliveredBytes]="row.deliveredBytes"
              ></elohim-commitment-bar>
            </li>
          }
        </ul>

        <dl class="totals">
          <dt>Net</dt>
          <dd data-testid="reciprocity-net">
            {{ formatBytes(v.netHostedBytes) }}
            {{
              v.netHostedBytes >= 0 ? 'hosted on my behalf' : 'I host more than I am hosted'
            }}
          </dd>
          <dt>Capacity</dt>
          <dd data-testid="reciprocity-capacity">{{ formatBytes(v.capacityAvailableBytes) }} free</dd>
        </dl>
      }
    </section>
  `,
  styles: [
    `
      .reciprocity {
        display: flex;
        flex-direction: column;
        gap: 0.75rem;
        padding: 1rem;
      }
      ul.rows {
        list-style: none;
        padding: 0;
        margin: 0;
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }
      .empty {
        color: var(--text-secondary, #777);
        font-size: 0.85rem;
      }
      dl.totals {
        display: grid;
        grid-template-columns: max-content 1fr;
        gap: 0.25rem 1rem;
      }
      dt {
        font-weight: 600;
      }
    `,
  ],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ReciprocityLedgerComponent implements OnInit, OnDestroy {
  protected readonly recip = inject(ReciprocityService);
  private stopPolling?: () => void;

  ngOnInit(): void {
    void this.recip.getMyReciprocity();
  }

  ngOnDestroy(): void {
    this.stopPolling?.();
  }

  startPolling(intervalMs = 30_000): void {
    this.stopPolling?.();
    this.stopPolling = this.recip.startPolling(intervalMs);
  }

  formatBytes(n: number): string {
    const abs = Math.abs(n);
    if (abs === 0) return '0 B';
    const gb = abs / 1e9;
    if (gb >= 1) return `${(n / 1e9).toFixed(1)} GB`;
    const mb = abs / 1e6;
    if (mb >= 1) return `${(n / 1e6).toFixed(1)} MB`;
    return `${(n / 1e3).toFixed(1)} KB`;
  }
}
