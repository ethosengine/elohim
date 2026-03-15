import { CommonModule } from '@angular/common';
import { HttpClient } from '@angular/common/http';
import { Component, OnInit, inject, signal } from '@angular/core';
import { ActivatedRoute, Router } from '@angular/router';

import { forkJoin } from 'rxjs';

import type {
  CollectiveView,
  CollectiveParticipationView,
  HumanView,
  ProposalView,
  DiscussionView,
} from '@elohim/storage-client/generated';

import { GovernanceApiService } from '@app/elohim/services/governance-api.service';

import { FaceCardComponent } from '../face-card/face-card.component';

interface ListResponse<T> {
  items: T[];
  count: number;
}

export type DetailTab = 'members' | 'proposals' | 'discussions';

/**
 * CollectiveDetailComponent - View a single collective with members, proposals, and discussions.
 *
 * Fetches collective metadata, participants (resolved to humans), proposals, and discussions.
 * Renders in a tabbed layout with FaceCards for members.
 */
@Component({
  selector: 'app-collective-detail',
  standalone: true,
  imports: [CommonModule, FaceCardComponent],
  template: `
    <div class="detail">
      <div class="header">
        <button class="back-btn" type="button" data-testid="back-to-directory" aria-label="Back to directory" (click)="navigateBack()">
          &larr; Directory
        </button>
        @if (collective()) {
          <h1>{{ collective()!.name }}</h1>
          @if (collective()!.description) {
            <p class="description">{{ collective()!.description }}</p>
          }
          <div class="meta">
            <span class="badge">{{ collective()!.governanceLayer }}</span>
            <span class="badge">{{ collective()!.reach }}</span>
          </div>
        }
      </div>

      <div class="tabs" role="tablist">
        @for (tab of tabOptions; track tab.value) {
          <button
            role="tab"
            type="button"
            class="tab"
            [class.active]="activeTab() === tab.value"
            [attr.aria-selected]="activeTab() === tab.value"
            (click)="setTab(tab.value)"
            [attr.data-testid]="'tab-' + tab.value"
          >
            {{ tab.label }}
          </button>
        }
      </div>

      @if (loading()) {
        <div class="loading">Loading...</div>
      } @else {
        @if (activeTab() === 'members') {
          <div class="card-grid" data-testid="members-view">
            @for (member of members(); track member.id) {
              <app-face-card
                [humanId]="member.id"
                [displayName]="member.displayName"
                [profilePhotoUrl]="member.profilePhotoUrl"
                [roleTag]="memberRoles().get(member.id) ?? null"
              />
            } @empty {
              <p class="empty">No members found.</p>
            }
          </div>
        }

        @if (activeTab() === 'proposals') {
          <div class="list" data-testid="proposals-view">
            @for (proposal of proposals(); track proposal.id) {
              <div class="list-item">
                <div class="list-item-header">
                  <span class="list-item-title">{{ proposal.title }}</span>
                  <span class="badge" [class.voting]="proposal.status === 'voting'">
                    {{ proposal.status }}
                  </span>
                </div>
                <span class="list-item-meta">
                  {{ proposal.proposalType }} &middot; {{ proposal.createdAt | date: 'mediumDate' }}
                </span>
              </div>
            } @empty {
              <p class="empty">No proposals yet.</p>
            }
          </div>
        }

        @if (activeTab() === 'discussions') {
          <div class="list" data-testid="discussions-view">
            @for (discussion of discussions(); track discussion.id) {
              <div class="list-item">
                <div class="list-item-header">
                  <span class="list-item-title">
                    {{ discussion.body.length > 80 ? discussion.body.slice(0, 80) + '...' : discussion.body }}
                  </span>
                </div>
                <span class="list-item-meta">
                  {{ discussion.authorPresenceId }} &middot; {{ discussion.createdAt | date: 'mediumDate' }}
                </span>
              </div>
            } @empty {
              <p class="empty">No discussions yet.</p>
            }
          </div>
        }
      }
    </div>
  `,
  styles: [
    `
      .detail {
        max-width: 960px;
        margin: 0 auto;
        padding: 1.5rem;
      }

      .header {
        margin-bottom: 1.5rem;
      }

      .back-btn {
        background: none;
        border: none;
        cursor: pointer;
        font-size: 0.875rem;
        color: var(--primary, #6366f1);
        padding: 0;
        margin-bottom: 0.75rem;
        font-family: inherit;
      }

      .back-btn:hover {
        text-decoration: underline;
      }

      h1 {
        font-size: 1.75rem;
        margin: 0 0 0.5rem 0;
        color: var(--text-primary, #1a1a1a);
      }

      .description {
        font-size: 0.875rem;
        color: var(--text-secondary, #666);
        margin: 0 0 0.75rem 0;
        line-height: 1.5;
      }

      .meta {
        display: flex;
        gap: 0.5rem;
      }

      .badge {
        font-size: 0.6875rem;
        padding: 0.125rem 0.5rem;
        border-radius: 9999px;
        background: var(--surface-elevated, #f5f5f5);
        color: var(--text-secondary, #666);
        text-transform: capitalize;
      }

      .badge.voting {
        background: var(--primary, #6366f1);
        color: #fff;
      }

      .tabs {
        display: flex;
        gap: 0.25rem;
        margin-bottom: 1.5rem;
        border-bottom: 2px solid var(--border, #e5e5e5);
      }

      .tab {
        padding: 0.625rem 1.25rem;
        border: none;
        background: none;
        cursor: pointer;
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-secondary, #666);
        border-bottom: 2px solid transparent;
        margin-bottom: -2px;
        transition:
          color 0.15s,
          border-color 0.15s;
        font-family: inherit;
      }

      .tab:hover {
        color: var(--text-primary, #1a1a1a);
      }

      .tab.active {
        color: var(--primary, #6366f1);
        border-bottom-color: var(--primary, #6366f1);
      }

      .card-grid {
        display: grid;
        grid-template-columns: repeat(auto-fill, minmax(120px, 1fr));
        gap: 0.75rem;
      }

      .list {
        display: flex;
        flex-direction: column;
        gap: 0.5rem;
      }

      .list-item {
        padding: 0.75rem 1rem;
        border: 1px solid var(--border, #e5e5e5);
        border-radius: 8px;
        background: var(--surface, #fff);
      }

      .list-item-header {
        display: flex;
        justify-content: space-between;
        align-items: center;
        gap: 0.5rem;
      }

      .list-item-title {
        font-size: 0.875rem;
        font-weight: 500;
        color: var(--text-primary, #1a1a1a);
      }

      .list-item-meta {
        font-size: 0.75rem;
        color: var(--text-secondary, #666);
        margin-top: 0.25rem;
        display: block;
      }

      .loading {
        text-align: center;
        padding: 3rem;
        color: var(--text-secondary, #666);
      }

      .empty {
        text-align: center;
        color: var(--text-secondary, #666);
        grid-column: 1 / -1;
        padding: 2rem;
      }

      @media (prefers-color-scheme: dark) {
        h1 {
          color: var(--text-primary, #f5f5f5);
        }

        .list-item {
          background: var(--surface, #1a1a1a);
          border-color: var(--border, #333);
        }

        .list-item-title {
          color: var(--text-primary, #f5f5f5);
        }

        .tab:hover {
          color: var(--text-primary, #f5f5f5);
        }
      }
    `,
  ],
})
export class CollectiveDetailComponent implements OnInit {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);
  private readonly governanceApi = inject(GovernanceApiService);

  readonly tabOptions: { label: string; value: DetailTab }[] = [
    { label: 'Members', value: 'members' },
    { label: 'Proposals', value: 'proposals' },
    { label: 'Discussions', value: 'discussions' },
  ];

  readonly collective = signal<CollectiveView | null>(null);
  readonly members = signal<HumanView[]>([]);
  readonly memberRoles = signal<Map<string, string>>(new Map());
  readonly proposals = signal<ProposalView[]>([]);
  readonly discussions = signal<DiscussionView[]>([]);
  readonly activeTab = signal<DetailTab>('members');
  readonly loading = signal(true);

  ngOnInit(): void {
    this.route.paramMap.subscribe(params => {
      const id = params.get('id');
      if (id) {
        this.loadCollective(id);
      }
    });
  }

  setTab(tab: DetailTab): void {
    this.activeTab.set(tab);
  }

  navigateBack(): void {
    this.router.navigate(['/community', 'directory']);
  }

  private loadCollective(id: string): void {
    this.loading.set(true);

    // Fetch collective details and participants in parallel
    forkJoin({
      collective: this.http.get<CollectiveView>(`/api/v1/collectives/${encodeURIComponent(id)}`),
      participants: this.http.get<ListResponse<CollectiveParticipationView>>(
        `/api/v1/collectives/${encodeURIComponent(id)}/participants`
      ),
    }).subscribe({
      next: ({ collective, participants }) => {
        this.collective.set(collective);

        // Build role map from participations
        const roles = new Map<string, string>();
        for (const p of participants.items) {
          if (p.roleContext) {
            roles.set(p.humanId, p.roleContext);
          }
        }
        this.memberRoles.set(roles);

        // Resolve human IDs to full human records
        const humanIds = participants.items.map(p => p.humanId);
        if (humanIds.length > 0) {
          this.http.get<ListResponse<HumanView>>('/db/humans').subscribe({
            next: response => {
              const humanSet = new Set(humanIds);
              this.members.set(response.items.filter(h => humanSet.has(h.id)));
              this.loading.set(false);
            },
            error: () => {
              this.loading.set(false);
            },
          });
        } else {
          this.loading.set(false);
        }

        // Load proposals and discussions for this collective's content
        this.loadGovernanceData(id);
      },
      error: () => {
        this.loading.set(false);
      },
    });
  }

  private loadGovernanceData(collectiveId: string): void {
    this.governanceApi.queryProposals(collectiveId).then(
      proposals => this.proposals.set(proposals),
      () => this.proposals.set([])
    );

    this.governanceApi.queryDiscussions(collectiveId).then(
      discussions => this.discussions.set(discussions),
      () => this.discussions.set([])
    );
  }
}
