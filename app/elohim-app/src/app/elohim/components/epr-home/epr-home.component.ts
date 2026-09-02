import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, effect, inject } from '@angular/core';
import { toObservable, toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { catchError, distinctUntilChanged, from, map, of, startWith, switchMap } from 'rxjs';

import { AuthService } from '@app/imagodei/services/auth.service';

import { GovernanceApiService, eprToUniversalHref } from '@elohim/service';
import { DistributionService, ResilienceService } from '@elohim/service/public-api';

import { SeoService } from '../../../services/seo.service';
import { EprRelationship } from '../../models/epr-head.model';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { EprNavService } from '../../services/epr-nav.service';
import { EprResolverService } from '../../services/epr-resolver.service';
import { SessionNavStackService } from '../../services/session-nav-stack.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageClientService } from '../../services/storage-client.service';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';

import { openInBundle } from './bundle-lens';
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
  private readonly navStack = inject(SessionNavStackService);
  private readonly eprNav = inject(EprNavService);
  private readonly auth = inject(AuthService);
  private readonly affinityService = inject(AffinityTrackingService);
  private readonly seoService = inject(SeoService);

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

  readonly stewards = toSignal(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.storageApi.getStewardshipAllocations({ contentId: id, activeOnly: true }).pipe(
              map(rows =>
                rows.map(
                  (r): StewardRow => ({
                    stewardPresenceId: r.stewardPresenceId,
                    contributionType: r.contributionType,
                    effectiveFrom: r.effectiveFrom,
                  })
                )
              ),
              catchError(() => of<StewardRow[]>([]))
            )
          : of<StewardRow[]>([])
      )
    ),
    { initialValue: [] as StewardRow[] }
  );

  readonly relationships = toSignal(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.eprResolver.resolveEprHead(id).pipe(
              map((head): EprRelationship[] => head?.relationships ?? []),
              catchError(() => of<EprRelationship[]>([]))
            )
          : of<EprRelationship[]>([])
      )
    ),
    { initialValue: [] as EprRelationship[] }
  );

  readonly challenges = toSignal(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? from(this.governance.getChallengesForEntity('content', id)).pipe(
              catchError(() => of<ChallengeView[]>([]))
            )
          : of<ChallengeView[]>([])
      )
    ),
    { initialValue: [] as ChallengeView[] }
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

  /** The cross-bundle "Open in <Bundle>" lens — minted from the generated all-bundle claims. */
  readonly lens = computed(() => {
    const a = this.atom();
    return a ? openInBundle(a.contentType, a.id) : null;
  });

  /** Where you actually came from — the previous stop on the session nav stack, or nothing. */
  readonly arrival = computed(() => {
    const prev = this.navStack.previous();
    if (!prev) return null;
    const label = (prev.label ?? prev.url).replace(/ \| Elohim Protocol$/, '').trim();
    return { href: prev.url, label: label || prev.url };
  });

  readonly signedIn = this.auth.isAuthenticated;

  private readonly affinityTick = toSignal(this.affinityService.affinity$, { initialValue: null });
  readonly affinity = computed(() => {
    this.affinityTick();
    const a = this.atom();
    return a ? this.affinityService.getAffinity(a.id) : 0;
  });
  readonly affinityPercent = computed(() => Math.round(this.affinity() * 100));
  readonly affinityWord = computed(() => {
    const v = this.affinity();
    if (v === 0) return 'Unseen';
    if (v > 0.9) return 'Got it';
    return 'Practicing';
  });

  constructor() {
    effect(() => {
      const a = this.atom();
      if (a) this.affinityService.trackView(a.id);
    });

    // The tab title and the session nav stack are both keyed off the load
    // state: a loaded atom names itself (SEO + this doorway's own history —
    // §12.2, so the arrival chip / gate referrer downstream have a prior
    // stop to name), the gate names itself "Out of reach" and records
    // nothing (an unreachable id isn't a place to come back to).
    effect(() => {
      const s = this.state();
      if (s.status === 'loaded') {
        this.seoService.updateForContent({
          id: s.atom.id,
          title: s.atom.title,
          summary: s.atom.description,
          contentType: s.atom.contentType,
          createdAt: s.atom.createdAt,
          updatedAt: s.atom.updatedAt,
        });
        this.navStack.record({
          url: this.address(),
          cid: s.atom.dhtAnchorHash ?? '',
          label: s.atom.title,
        });
      } else if (s.status === 'not-found') {
        this.seoService.setTitle('Out of reach');
      }
    });
  }

  onArrival(event: Event, href: string): void {
    event.preventDefault();
    this.eprNav.navigate(href);
  }

  markPracticing(): void {
    const a = this.atom();
    if (a) this.affinityService.setAffinity(a.id, 0.5);
  }

  markGotIt(): void {
    const a = this.atom();
    if (a) this.affinityService.setAffinity(a.id, 1);
  }
}
