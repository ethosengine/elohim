/**
 * SensemakingPageComponent - Route wrapper for the sensemaking experience
 *
 * Reads entityType/entityId from query params and composes the three
 * sensemaking sub-components into a full-page experience:
 * 1. ContributeStatementComponent - Polis-style one-at-a-time voting
 * 2. OpinionClusterComponent - 2D opinion landscape visualization
 * 3. Bridging statements + bracket synthesis trigger
 *
 * Navigated to from the FeedbackMechanismGateway when accumulation
 * signals indicate the entity is ready for sensemaking.
 */

import { Component, computed, inject, OnInit, signal } from '@angular/core';
import { ActivatedRoute, RouterLink } from '@angular/router';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

import { BracketSynthesisService } from '../../services/bracket-synthesis.service';
import { ContributeStatementComponent } from '../contribute-statement/contribute-statement.component';
import { OpinionClusterComponent } from '../opinion-cluster/opinion-cluster.component';

import type { SensemakingResultView, StatementView } from '@elohim/storage-client/generated';

@Component({
  selector: 'qahal-sensemaking-page',
  standalone: true,
  imports: [ContributeStatementComponent, OpinionClusterComponent, RouterLink],
  template: `
    <div class="sensemaking-page">
      <h2>Community Sensemaking</h2>
      <p class="subtitle">Contribute your perspective and see where opinions cluster</p>

      @if (entityType() && entityId()) {
        <!-- Polis-style statement voting -->
        <section class="contribute-section">
          <h3>Share & Vote</h3>
          <app-contribute-statement [entityType]="entityType()!" [entityId]="entityId()!" />
        </section>

        <!-- Opinion cluster visualization -->
        <section class="clusters-section">
          <h3>Opinion Landscape</h3>
          <app-opinion-cluster [entityType]="entityType()!" [entityId]="entityId()!" />
        </section>

        <!-- Bridging statements + synthesis -->
        <section class="bridging-section">
          <h3>Common Ground</h3>
          @for (statement of bridgingStatements(); track statement.id) {
            <div class="bridging-card">
              <span class="badge">Common Ground</span>
              <p>{{ statement.text }}</p>
            </div>
          }
          @if (bridgingStatements().length > 0) {
            <button
              class="synthesize-btn"
              data-testid="synthesize-bracket"
              (click)="synthesizeBracket()"
              [disabled]="synthesizing()"
            >
              {{ synthesizing() ? 'Synthesizing...' : 'Synthesize Ranked-Choice Bracket' }}
            </button>
          }
        </section>

        <!-- Divisive statements (need more dialogue) -->
        @if (divisiveStatements().length > 0) {
          <section class="divisive-section">
            <h3>Needs More Dialogue</h3>
            @for (statement of divisiveStatements(); track statement.id) {
              <div class="divisive-card">
                <span class="badge badge-divisive">Divisive</span>
                <p>{{ statement.text }}</p>
              </div>
            }
          </section>
        }
      } @else {
        <div class="missing-context">
          <p>Missing entity context. Please navigate here from a content item.</p>
          <a routerLink="/community">Back to Community</a>
        </div>
      }
    </div>
  `,
  styles: `
    .sensemaking-page {
      max-width: 800px;
      margin: 0 auto;
      padding: 1.5rem 1rem;
    }

    h2 {
      margin: 0 0 0.25rem;
      font-size: 1.5rem;
      font-weight: 600;
    }

    .subtitle {
      margin: 0 0 2rem;
      color: var(--text-secondary, #666);
      font-size: 0.9375rem;
    }

    h3 {
      margin: 0 0 0.75rem;
      font-size: 1.125rem;
      font-weight: 600;
    }

    section {
      margin-bottom: 2rem;
    }

    .bridging-card {
      display: flex;
      align-items: flex-start;
      gap: 0.75rem;
      padding: 0.75rem 1rem;
      margin-bottom: 0.5rem;
      border: 1px solid var(--border-color, #e0e0e0);
      border-radius: 0.5rem;
      background: var(--surface-color, #fff);
    }

    .bridging-card p {
      margin: 0;
      line-height: 1.5;
      font-size: 0.9375rem;
    }

    .badge {
      flex-shrink: 0;
      display: inline-block;
      padding: 0.125rem 0.5rem;
      font-size: 0.6875rem;
      font-weight: 600;
      text-transform: uppercase;
      letter-spacing: 0.04em;
      border-radius: 999px;
      background: var(--settled-bg, #d1fae5);
      color: var(--settled-fg, #065f46);
      white-space: nowrap;
    }

    .synthesize-btn {
      display: block;
      margin-top: 1rem;
      padding: 0.625rem 1.25rem;
      border: none;
      border-radius: 0.375rem;
      background: var(--primary-color, #2196f3);
      color: #fff;
      font-size: 0.875rem;
      font-weight: 500;
      cursor: pointer;
      transition: opacity 0.15s ease;
    }

    .synthesize-btn:hover:not(:disabled) {
      opacity: 0.9;
    }

    .synthesize-btn:disabled {
      opacity: 0.5;
      cursor: not-allowed;
    }

    .missing-context {
      text-align: center;
      padding: 3rem 1rem;
      color: var(--text-secondary, #666);
    }

    .missing-context a {
      display: inline-block;
      margin-top: 1rem;
      color: var(--primary-color, #2196f3);
      text-decoration: none;
    }

    .missing-context a:hover {
      text-decoration: underline;
    }

    .divisive-card {
      display: flex;
      align-items: flex-start;
      gap: 0.75rem;
      padding: 0.75rem 1rem;
      margin-bottom: 0.5rem;
      border: 1px solid var(--border-color, #e0e0e0);
      border-radius: 0.5rem;
      background: var(--surface-color, #fff);
    }

    .divisive-card p {
      margin: 0;
      line-height: 1.5;
      font-size: 0.9375rem;
    }

    .badge-divisive {
      background: var(--warning-bg, #fef3c7);
      color: var(--warning-fg, #92400e);
    }

    @media (prefers-color-scheme: dark) {
      .bridging-card,
      .divisive-card {
        background: var(--surface-color, #1a1a1a);
        border-color: var(--border-color, #333);
      }

      .badge {
        background: var(--settled-bg, #14432a);
        color: var(--settled-fg, #6ee7b7);
      }

      .badge-divisive {
        background: var(--warning-bg, #451a03);
        color: var(--warning-fg, #fcd34d);
      }
    }
  `,
})
export class SensemakingPageComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly governanceApi = inject(GovernanceApiService);
  private readonly bracketSynthesis = inject(BracketSynthesisService);

  /** Entity type from query params. */
  readonly entityType = signal<string | null>(null);

  /** Entity ID from query params. */
  readonly entityId = signal<string | null>(null);

  /** Full sensemaking result (clusters + bridging statements). */
  private readonly sensemakingResult = signal<SensemakingResultView | null>(null);

  /** Bridging statements extracted from the sensemaking result. */
  readonly bridgingStatements = computed<StatementView[]>(
    () => this.sensemakingResult()?.bridgingStatements ?? []
  );

  /** Divisive statements that split clusters. */
  readonly divisiveStatements = computed<StatementView[]>(
    () => this.sensemakingResult()?.divisiveStatements ?? []
  );

  /** Whether bracket synthesis is in progress. */
  readonly synthesizing = signal(false);

  ngOnInit(): void {
    this.route.queryParamMap.subscribe(params => {
      const type = params.get('entityType');
      const id = params.get('entityId');
      this.entityType.set(type);
      this.entityId.set(id);

      if (type && id) {
        this.loadClusters(type, id);
      }
    });
  }

  /** Trigger bracket synthesis from bridging statements. */
  async synthesizeBracket(): Promise<void> {
    const result = this.sensemakingResult();
    const type = this.entityType();
    const id = this.entityId();

    if (!result || !type || !id) return;

    this.synthesizing.set(true);
    try {
      await this.bracketSynthesis.synthesizeBracket(type, id, result);
    } finally {
      this.synthesizing.set(false);
    }
  }

  private async loadClusters(entityType: string, entityId: string): Promise<void> {
    try {
      const result = await this.governanceApi.getClusters(entityType, entityId);
      this.sensemakingResult.set(result);
    } catch {
      // Clusters may not be available yet — leave empty
      this.sensemakingResult.set(null);
    }
  }
}
