/**
 * Network Health Tab Component
 *
 * Renders P2P mesh posture metrics pulled from GET /api/v1/network/posture,
 * plus a household-grouped peer breakdown built from:
 *   - GET /api/v1/collectives (to enumerate households)
 *   - GET /api/v1/households/{id}/devices  (per-household peer vitals)
 *   - GET /api/v1/commitments?state=active (REA commitment counts per household)
 *
 * The posture summary cards are retained at top; the new grouped list lives
 * below. Gracefully degrades to an "unavailable" state when endpoints are not
 * yet deployed.
 */

import { CommonModule } from '@angular/common';
import { HttpClient } from '@angular/common/http';
import { Component, OnInit, OnDestroy, inject, signal } from '@angular/core';

import { catchError, map, switchMap, takeUntil } from 'rxjs/operators';

import { combineLatest, forkJoin, of, Subject } from 'rxjs';

import { CollectiveService } from '@app/qahal/services/collective.service';
import { HouseholdDevicesService } from '@app/shefa/services/household-devices.service';

import {
  NetworkPostureService,
  type NetworkPostureView,
} from '../../services/network-posture.service';

import type { HouseholdDevicesView } from '@app/generated/household-devices-view';
import type { CollectiveView } from '@elohim/storage-client/generated';

// ---------------------------------------------------------------------------
// View models
// ---------------------------------------------------------------------------

export interface HouseholdPeerRow {
  peerId: string;
  archetype?: string;
  lifecycleState: string;
}

export interface HouseholdGroup {
  householdId: string;
  label: string;
  peerCount: number;
  activeCommitments: number;
  peers: HouseholdPeerRow[];
}

/** REA commitment used only for counting active commitments per household. */
interface ReaCommitmentSummary {
  provider: string;
  state: string;
}

interface CommitmentsListResponse {
  items: ReaCommitmentSummary[];
  total: number;
}

// ---------------------------------------------------------------------------
// Component
// ---------------------------------------------------------------------------

@Component({
  selector: 'app-network-health-tab',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './network-health-tab.component.html',
  styleUrl: './network-health-tab.component.scss',
})
export class NetworkHealthTabComponent implements OnInit, OnDestroy {
  private readonly networkPosture = inject(NetworkPostureService);
  private readonly householdDevices = inject(HouseholdDevicesService);
  private readonly collectiveService = inject(CollectiveService);
  private readonly http = inject(HttpClient);
  private readonly destroy$ = new Subject<void>();

  readonly posture = signal<NetworkPostureView | null>(null);
  readonly loading = signal(true);

  /** Household-grouped peer breakdown. Null while loading; empty array when no peers. */
  readonly householdGroups = signal<HouseholdGroup[] | null>(null);
  readonly groupsLoading = signal(true);

  ngOnInit(): void {
    // Existing posture summary stream
    this.networkPosture
      .get()
      .pipe(takeUntil(this.destroy$))
      .subscribe(data => {
        this.posture.set(data);
        this.loading.set(false);
      });

    // Household-grouped breakdown
    this.loadHouseholdGroups();
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  // -------------------------------------------------------------------------
  // Household grouping reducer
  // -------------------------------------------------------------------------

  private loadHouseholdGroups(): void {
    // Step 1: list all collectives and filter to households
    const households$ = this.collectiveService.listCollectives({ activeOnly: true }).pipe(
      map((collectives: CollectiveView[]) =>
        collectives.filter(c => {
          const isHouseholdById = c.id.startsWith('household-');
          const isHouseholdByLayer = c.governanceLayer === 'household';
          return isHouseholdById ? true : isHouseholdByLayer;
        })
      ),
      catchError(() => of([] as CollectiveView[]))
    );

    // Step 2: fetch devices-per-household + active commitments in parallel
    combineLatest([households$, this.fetchActiveCommitments()])
      .pipe(
        switchMap(([households, commitments]) => {
          if (households.length === 0) {
            return of([] as HouseholdGroup[]);
          }

          // Build commitment-count map keyed by provider agent pubkey
          // (commitments.provider is the agent who made the commitment)
          const commitmentsByProvider = new Map<string, number>();
          for (const c of commitments) {
            if (c.state === 'active') {
              commitmentsByProvider.set(
                c.provider,
                (commitmentsByProvider.get(c.provider) ?? 0) + 1
              );
            }
          }

          // Fetch devices for each household in parallel
          const deviceCalls = households.map(h =>
            this.householdDevices
              .list(h.id)
              .pipe(
                catchError(() => of({ householdId: h.id, devices: [] } as HouseholdDevicesView))
              )
          );

          return forkJoin(deviceCalls).pipe(
            map((householdViews: HouseholdDevicesView[]) =>
              householdViews.map((hv): HouseholdGroup => {
                const peers: HouseholdPeerRow[] = hv.devices
                  .filter(d => d.peer !== null)
                  .map(d => ({
                    peerId: d.peer!.peerId,
                    archetype: d.peer!.archetypeClass ?? undefined,
                    lifecycleState: d.peer!.status,
                  }));

                // Sum active commitments where the provider is a peer in this household
                const peerIds = new Set(peers.map(p => p.peerId));
                let activeCommitments = 0;
                for (const [provider, count] of commitmentsByProvider) {
                  if (peerIds.has(provider)) {
                    activeCommitments += count;
                  }
                }

                return {
                  householdId: hv.householdId,
                  label: hv.householdId,
                  peerCount: peers.length,
                  activeCommitments,
                  peers,
                };
              })
            )
          );
        }),
        catchError(() => of([] as HouseholdGroup[])),
        takeUntil(this.destroy$)
      )
      .subscribe(groups => {
        this.householdGroups.set(groups);
        this.groupsLoading.set(false);
      });
  }

  private fetchActiveCommitments() {
    return this.http.get<CommitmentsListResponse>('/api/v1/commitments?state=active').pipe(
      map(res => res.items ?? []),
      catchError(() => of([] as ReaCommitmentSummary[]))
    );
  }

  // -------------------------------------------------------------------------
  // Template helpers
  // -------------------------------------------------------------------------

  get householdCount(): number {
    return this.householdGroups()?.length ?? 0;
  }

  /** Format storagePressure (0..1) as a percentage string */
  formatPressure(pressure: number): string {
    return `${Math.round(pressure * 100)}%`;
  }

  /** CSS class for storage pressure bar fill — matches dashboard health palette */
  pressureClass(pressure: number): string {
    if (pressure < 0.6) return 'pressure-ok';
    if (pressure < 0.85) return 'pressure-warning';
    return 'pressure-critical';
  }
}
