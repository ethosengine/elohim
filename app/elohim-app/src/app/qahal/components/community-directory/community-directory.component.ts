import { CommonModule } from '@angular/common';
import { HttpClient } from '@angular/common/http';
import { Component, OnInit, computed, inject, signal } from '@angular/core';

import { forkJoin } from 'rxjs';

import type { CollectiveView, CollectiveParticipationView, HumanView } from '@elohim/storage-client/generated';

import { FaceCardComponent } from '../face-card/face-card.component';

/**
 * Type aliases for clarity within directory context.
 */
export type HumanEntry = HumanView;
export type CollectiveEntry = CollectiveView;
export type ParticipantEntry = CollectiveParticipationView;

export type DirectoryView = 'all' | 'households' | 'groups';

interface ListResponse<T> {
  items: T[];
  count: number;
}

/**
 * CommunityDirectoryComponent - Browse community members by view mode
 *
 * Three views toggled by tabs:
 * - All: flat grid of face cards with household subtitle
 * - Households: grouped by household, with "Individuals" overflow
 * - Groups: grouped by collective with description and role tags
 */
@Component({
  selector: 'app-community-directory',
  standalone: true,
  imports: [CommonModule, FaceCardComponent],
  template: `
    <div class="directory">
      <h1>Community Directory</h1>

      <div class="tabs" role="tablist">
        @for (tab of tabs; track tab.value) {
          <button
            role="tab"
            type="button"
            class="tab"
            [class.active]="activeView() === tab.value"
            [attr.aria-selected]="activeView() === tab.value"
            (click)="activeView.set(tab.value)"
            [attr.data-testid]="'tab-' + tab.value"
          >
            {{ tab.label }}
          </button>
        }
      </div>

      @if (loading()) {
        <div class="loading" data-testid="loading-indicator">Loading directory...</div>
      } @else {
        <!-- All view -->
        @if (activeView() === 'all') {
          <div class="card-grid" data-testid="all-view">
            @for (human of humans(); track human.id) {
              <app-face-card
                [humanId]="human.id"
                [displayName]="human.displayName"
                [profilePhotoUrl]="human.profilePhotoUrl"
                [subtitle]="humanHousehold().get(human.id) ?? null"
                (selected)="onHumanSelected($event)"
              />
            } @empty {
              <p class="empty">No community members found.</p>
            }
          </div>
        }

        <!-- Households view -->
        @if (activeView() === 'households') {
          <div data-testid="households-view">
            @for (household of households(); track household.id) {
              <section class="group-section">
                <h2 class="group-header">{{ household.name }}</h2>
                <div class="card-grid">
                  @for (
                    participant of participantsByCollective().get(household.id) ?? [];
                    track participant.id
                  ) {
                    <app-face-card
                      [humanId]="participant.humanId"
                      [displayName]="humanNameMap().get(participant.humanId) ?? participant.humanId"
                      [profilePhotoUrl]="humanPhotoMap().get(participant.humanId) ?? null"
                      [roleTag]="participant.roleContext"
                      (selected)="onHumanSelected($event)"
                    />
                  }
                </div>
              </section>
            }

            @if (individualsNotInHousehold().length > 0) {
              <section class="group-section">
                <h2 class="group-header">Individuals</h2>
                <div class="card-grid">
                  @for (human of individualsNotInHousehold(); track human.id) {
                    <app-face-card
                      [humanId]="human.id"
                      [displayName]="human.displayName"
                      [profilePhotoUrl]="human.profilePhotoUrl"
                      (selected)="onHumanSelected($event)"
                    />
                  }
                </div>
              </section>
            }
          </div>
        }

        <!-- Groups view -->
        @if (activeView() === 'groups') {
          <div data-testid="groups-view">
            @for (group of groups(); track group.id) {
              <section class="group-section">
                <h2 class="group-header">{{ group.name }}</h2>
                @if (group.description) {
                  <p class="group-description">{{ group.description }}</p>
                }
                <div class="card-grid">
                  @for (
                    participant of participantsByCollective().get(group.id) ?? [];
                    track participant.id
                  ) {
                    <app-face-card
                      [humanId]="participant.humanId"
                      [displayName]="humanNameMap().get(participant.humanId) ?? participant.humanId"
                      [profilePhotoUrl]="humanPhotoMap().get(participant.humanId) ?? null"
                      [roleTag]="participant.roleContext"
                      (selected)="onHumanSelected($event)"
                    />
                  }
                </div>
              </section>
            } @empty {
              <p class="empty">No groups found.</p>
            }
          </div>
        }
      }
    </div>
  `,
  styles: [
    `
      .directory {
        max-width: 960px;
        margin: 0 auto;
        padding: 1.5rem;
      }

      h1 {
        font-size: 1.75rem;
        margin: 0 0 1.5rem 0;
        color: var(--text-primary, #1a1a1a);
      }

      .tabs {
        display: flex;
        gap: 0.25rem;
        margin-bottom: 1.5rem;
        border-bottom: 2px solid var(--border, #e5e5e5);
        padding-bottom: 0;
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

      .group-section {
        margin-bottom: 2rem;
      }

      .group-header {
        font-size: 1.125rem;
        margin: 0 0 0.75rem 0;
        color: var(--text-primary, #1a1a1a);
        padding-bottom: 0.375rem;
        border-bottom: 1px solid var(--border, #e5e5e5);
      }

      .group-description {
        font-size: 0.8125rem;
        color: var(--text-secondary, #666);
        margin: -0.375rem 0 0.75rem 0;
        line-height: 1.5;
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
        h1,
        .group-header {
          color: var(--text-primary, #f5f5f5);
        }

        .tab:hover {
          color: var(--text-primary, #f5f5f5);
        }
      }
    `,
  ],
})
export class CommunityDirectoryComponent implements OnInit {
  private readonly http = inject(HttpClient);

  readonly tabs: { label: string; value: DirectoryView }[] = [
    { label: 'All', value: 'all' },
    { label: 'Households', value: 'households' },
    { label: 'Groups', value: 'groups' },
  ];

  readonly humans = signal<HumanEntry[]>([]);
  readonly households = signal<CollectiveEntry[]>([]);
  readonly groups = signal<CollectiveEntry[]>([]);
  readonly participantsByCollective = signal<Map<string, ParticipantEntry[]>>(new Map());
  readonly activeView = signal<DirectoryView>('all');
  readonly loading = signal(true);

  /** Map humanId -> display name for participant lookups */
  readonly humanNameMap = computed(() => {
    const m = new Map<string, string>();
    for (const h of this.humans()) {
      m.set(h.id, h.displayName);
    }
    return m;
  });

  /** Map humanId -> profilePhotoUrl for participant lookups */
  readonly humanPhotoMap = computed(() => {
    const m = new Map<string, string | null>();
    for (const h of this.humans()) {
      m.set(h.id, h.profilePhotoUrl);
    }
    return m;
  });

  /** Map humanId -> household name for subtitle in "all" view */
  readonly humanHousehold = computed(() => {
    const m = new Map<string, string>();
    const householdList = this.households();
    const participantsMap = this.participantsByCollective();

    for (const household of householdList) {
      const participants = participantsMap.get(household.id) ?? [];
      for (const p of participants) {
        m.set(p.humanId, household.name);
      }
    }
    return m;
  });

  /** Humans not in any household */
  readonly individualsNotInHousehold = computed(() => {
    const householdMembers = this.humanHousehold();
    return this.humans().filter(h => !householdMembers.has(h.id));
  });

  ngOnInit(): void {
    this.loadDirectory();
  }

  onHumanSelected(humanId: string): void {
    // Future: navigate to human profile
    console.log('Selected human:', humanId);
  }

  private loadDirectory(): void {
    this.loading.set(true);

    forkJoin({
      humans: this.http.get<ListResponse<HumanEntry>>('/db/humans'),
      collectives: this.http.get<ListResponse<CollectiveEntry>>('/api/v1/collectives'),
    }).subscribe({
      next: ({ humans, collectives }) => {
        this.humans.set(humans.items);

        const allCollectives = collectives.items;
        this.households.set(allCollectives.filter(c => c.governanceLayer === 'family'));
        this.groups.set(allCollectives.filter(c => c.governanceLayer !== 'family'));

        // Load participants for each collective
        if (allCollectives.length > 0) {
          const participantRequests: Record<string, ReturnType<typeof this.http.get<ListResponse<ParticipantEntry>>>> = {};
          for (const c of allCollectives) {
            participantRequests[c.id] = this.http.get<ListResponse<ParticipantEntry>>(
              `/api/v1/collectives/${c.id}/participants`
            );
          }

          forkJoin(participantRequests).subscribe({
            next: responses => {
              const map = new Map<string, ParticipantEntry[]>();
              for (const [id, response] of Object.entries(responses)) {
                map.set(id, (response as ListResponse<ParticipantEntry>).items);
              }
              this.participantsByCollective.set(map);
              this.loading.set(false);
            },
            error: () => {
              this.loading.set(false);
            },
          });
        } else {
          this.loading.set(false);
        }
      },
      error: () => {
        this.loading.set(false);
      },
    });
  }
}
