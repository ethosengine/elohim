import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toObservable, toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { catchError, distinctUntilChanged, from, map, of, startWith, switchMap } from 'rxjs';

import { GovernanceApiService, eprToUniversalHref } from '@elohim/service';
import { DistributionService, ResilienceService } from '@elohim/service/public-api';

import { EprRelationship } from '../../models/epr-head.model';
import { EprResolverService } from '../../services/epr-resolver.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageClientService } from '../../services/storage-client.service';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';

import { EprHomeLegsComponent, humanizeSlug } from './epr-home-legs.component';
import {
  EprHomeAtom,
  StewardRow,
  anchorWords,
  heldChip,
  holdingWords,
  reachSubtitle,
  toAtom,
} from './epr-home.model';

import type { ResilienceSnapshotView } from '@app/generated/resilience-snapshot-view';
import type { ChallengeView } from '@elohim/storage-client/generated';

type LoadState =
  | { status: 'loading'; id: string }
  | { status: 'loaded'; id: string; atom: EprHomeAtom }
  | { status: 'not-found'; id: string }
  | { status: 'error'; id: string };

/**
 * EprHomeComponent — the shell-owned universal address (/epr/{id}).
 *
 * One frame for every atom: arrival · identity · focal render shaped by format
 * · the four legs · the address line. Reads the RAW ContentView for the frame
 * and hands only the slug to <app-epr-focal>; imports nothing from the lamad
 * bundle. An unreachable atom renders the designed gate (spec §2.5) — never a
 * wall, never the controls of a thing that cannot be seen.
 */
@Component({
  selector: 'app-epr-home',
  standalone: true,
  imports: [CommonModule, RouterModule, EprFocalComponent, EprHomeLegsComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './epr-home.component.html',
  styleUrl: './epr-home.component.css',
})
export class EprHomeComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly storage = inject(StorageClientService);
  private readonly resilience = inject(ResilienceService);
  private readonly distribution = inject(DistributionService);
  private readonly storageApi = inject(StorageApiService);
  private readonly eprResolver = inject(EprResolverService);
  private readonly governance = inject(GovernanceApiService);

  private readonly state = toSignal(
    this.route.paramMap.pipe(
      map(p => p.get('resourceId') ?? ''),
      distinctUntilChanged(),
      switchMap(id =>
        this.storage.getContent(id).pipe(
          map(
            (raw): LoadState =>
              raw === null
                ? { status: 'not-found', id }
                : { status: 'loaded', id, atom: toAtom(raw as unknown as Record<string, unknown>) }
          ),
          catchError(() => of<LoadState>({ status: 'error', id })),
          startWith<LoadState>({ status: 'loading', id })
        )
      )
    ),
    { initialValue: { status: 'loading', id: '' } as LoadState }
  );

  readonly resourceId = computed(() => this.state().id);
  readonly status = computed(() => this.state().status);
  readonly atom = computed<EprHomeAtom | null>(() => {
    const s = this.state();
    return s.status === 'loaded' ? s.atom : null;
  });

  /** The loaded atom's id as a stream — every leg loader keys off it. */
  private readonly atomId$ = toObservable(this.atom).pipe(
    map(a => a?.id ?? null),
    distinctUntilChanged()
  );

  readonly snapshot = toSignal<ResilienceSnapshotView | null>(
    this.atomId$.pipe(
      switchMap(id =>
        id ? this.resilience.getSnapshot(id).pipe(catchError(() => of(null))) : of(null)
      )
    ),
    { initialValue: null }
  );

  readonly peersHolding = toSignal<number | null>(
    toObservable(this.atom).pipe(
      map(a => a?.blobHash ?? null),
      distinctUntilChanged(),
      switchMap(hash =>
        hash
          ? from(this.distribution.getDetails(hash)).pipe(
              map(d => d.summary.replicaCount),
              catchError(() => of(null))
            )
          : of(null)
      )
    ),
    { initialValue: null }
  );

  readonly stewards = toSignal<StewardRow[]>(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.storageApi.getStewardshipAllocations({ contentId: id, activeOnly: true }).pipe(
              map(rows =>
                rows.map(r => ({
                  stewardPresenceId: r.stewardPresenceId,
                  contributionType: r.contributionType,
                  effectiveFrom: r.effectiveFrom,
                }))
              ),
              catchError(() => of([]))
            )
          : of([])
      )
    ),
    { initialValue: [] }
  );

  readonly relationships = toSignal<EprRelationship[]>(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.eprResolver.resolveEprHead(id).pipe(
              map(head => head?.relationships ?? []),
              catchError(() => of([]))
            )
          : of([])
      )
    ),
    { initialValue: [] }
  );

  readonly challenges = toSignal<ChallengeView[]>(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? from(this.governance.getChallengesForEntity('content', id)).pipe(
              catchError(() => of([]))
            )
          : of([])
      )
    ),
    { initialValue: [] }
  );

  readonly heldChipLabel = computed(() => heldChip(holdingWords(this.snapshot())));
  readonly heldWarm = computed(() => holdingWords(this.snapshot()).warm);

  readonly reachLabel = computed(() => {
    const r = this.atom()?.reach ?? 'commons';
    return r.charAt(0).toUpperCase() + r.slice(1);
  });
  readonly reachSub = computed(() => reachSubtitle(this.atom()?.reach ?? 'commons'));
  readonly notarizedLabel = computed(() =>
    this.atom()?.trust === 'notarized' ? 'Notarized' : 'Not yet notarized'
  );
  readonly anchorSub = computed(() => {
    const a = this.atom();
    return a ? anchorWords(a.trust, a.dhtAnchorState) : '';
  });
  readonly eyebrow = computed<string[]>(() => {
    const a = this.atom();
    if (!a) return [];
    return [
      humanizeSlug(a.category ?? a.contentType),
      a.estimatedTime,
      a.author ? `by ${a.author}` : null,
      a.license,
    ].filter((x): x is string => !!x);
  });
  readonly address = computed(() => eprToUniversalHref({ id: this.resourceId(), tier: 'head' }));
  readonly rawHref = computed(() =>
    eprToUniversalHref({ id: this.resourceId(), tier: 'head', subview: 'raw' })
  );
}
