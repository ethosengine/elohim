import { CommonModule, DOCUMENT } from '@angular/common';
import { HttpClient } from '@angular/common/http';
import {
  CUSTOM_ELEMENTS_SCHEMA,
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  signal,
  untracked,
} from '@angular/core';
import { takeUntilDestroyed, toObservable, toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, Router, RouterModule } from '@angular/router';

import {
  Subject,
  catchError,
  combineLatest,
  distinctUntilChanged,
  forkJoin,
  from,
  map,
  merge,
  of,
  startWith,
  switchMap,
  take,
} from 'rxjs';

import 'elohim-core/register';

import { AuthService } from '@app/imagodei/services/auth.service';

import { ObservationEmitterService, scrollDepthPct } from '@elohim/rea-runtime';
import { GovernanceApiService, eprToUniversalHref } from '@elohim/service';
import { DistributionService, ResilienceService } from '@elohim/service/public-api';

import { SeoService } from '../../../services/seo.service';
import { EprRelationship } from '../../models/epr-head.model';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { AttestationApiService } from '../../services/attestation-api.service';
import { EprNavService } from '../../services/epr-nav.service';
import { EprResolverService } from '../../services/epr-resolver.service';
import { ResilienceDetailService } from '../../services/resilience-detail.service';
import { SessionNavStackService } from '../../services/session-nav-stack.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageClientService } from '../../services/storage-client.service';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';

import { editInBundle, openInBundle } from './bundle-lens';
import {
  ConcernSubmit,
  CopiesSummary,
  EprHomeLegsComponent,
  FoldRequest,
  VerifyState,
  humanizeSlug,
} from './epr-home-legs.component';
import { SeeAlsoRow, mergeSeeAlso } from './epr-home-see-also';
import {
  SignalConcernKind,
  challengeInput,
  concernSignal,
  landedSummary,
  reactionCounts,
  reactionSignal,
} from './epr-home-signals';
import {
  EprHomeAtom,
  FoldLeg,
  LegAnchor,
  StewardRow,
  anchorWords,
  discussionTitle,
  heldChip,
  holdingWords,
  legFromFragment,
  reachSubtitle,
  toAtom,
} from './epr-home.model';

import type { AttestationView } from '@app/generated/attestation-view';
import type { ResilienceSnapshotView } from '@app/generated/resilience-snapshot-view';
import type {
  ChallengeView,
  ContentGraphView,
  ContentEngagementStatsView,
  ContributorPresenceView,
  DiscussionView,
  GovernanceStateView,
  SignalAggregateView,
} from '@elohim/storage-client/generated';
import type { ResilienceView } from '@elohim/storage-client/generated/ResilienceView';
import type { ReactionSubmitEvent } from 'elohim-core';

type LoadState =
  | { status: 'loading'; id: string }
  | { status: 'loaded'; id: string; atom: EprHomeAtom }
  | { status: 'not-found'; id: string }
  | { status: 'error'; id: string };

/** The global focused-view effect (navigation hidden, scroll locked) the shell styles own. */
const FOCUSED_VIEW_BODY_CLASS = 'focused-view-mode';

/** What the person is told once a signal-kind concern has been recorded. */
const CONCERN_RECORDED: Readonly<Record<SignalConcernKind, string>> = {
  flag: 'Your flag was recorded.',
  feedback: 'Your feedback was recorded.',
  report: 'Your report was recorded.',
};

/** The frame's scroller when Focus is on (fixed, overflow:auto). */
const FOCUS_SCROLLER_CLASS = 'epr-home';

/**
 * EprHomeComponent — the shell-owned universal address (/epr/{id}).
 *
 * One frame for every atom: arrival · identity · focal render shaped by format
 * · the legs, each with its fold · the commons · the address line. Reads the
 * RAW ContentView for the frame and hands only the slug to <app-epr-focal>;
 * imports nothing from the lamad bundle. An unreachable atom renders the
 * designed gate (spec §2.5) — never a wall, never the controls of a thing that
 * cannot be seen. Since spec §11 the home also carries every concern the
 * legacy resource viewer did; `/resource/{id}` redirects here.
 *
 * Writes go through existing coordinator-backed routes only, and every write
 * field comes from the session or a declared vocabulary (epr-home-signals.ts).
 */
@Component({
  selector: 'app-epr-home',
  standalone: true,
  imports: [CommonModule, RouterModule, EprFocalComponent, EprHomeLegsComponent],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './epr-home.component.html',
  styleUrl: './epr-home.component.css',
  host: {
    '[class.epr-home--focused]': 'focused()',
    '(document:keydown.escape)': 'exitFocus()',
  },
})
export class EprHomeComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly router = inject(Router);
  private readonly http = inject(HttpClient);
  private readonly storage = inject(StorageClientService);
  private readonly resilience = inject(ResilienceService);
  private readonly resilienceDetail = inject(ResilienceDetailService);
  private readonly distribution = inject(DistributionService);
  private readonly storageApi = inject(StorageApiService);
  private readonly attestationApi = inject(AttestationApiService);
  private readonly eprResolver = inject(EprResolverService);
  private readonly governance = inject(GovernanceApiService);
  private readonly navStack = inject(SessionNavStackService);
  private readonly eprNav = inject(EprNavService);
  private readonly auth = inject(AuthService);
  private readonly affinityService = inject(AffinityTrackingService);
  private readonly seoService = inject(SeoService);
  /** Witnesses dwell + scroll depth as a `lamad:content-viewed` observation (R-A1/R-A3). */
  private readonly observationEmitter = inject(ObservationEmitterService);
  private readonly document = inject(DOCUMENT);
  private readonly elRef = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly injector = inject(Injector);
  private readonly destroyRef = inject(DestroyRef);

  private readonly state = toSignal(
    this.route.paramMap.pipe(
      map(p => p.get('resourceId') ?? ''),
      distinctUntilChanged(),
      switchMap(id =>
        this.storage.getContent(id, !this.auth.isAuthenticated()).pipe(
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

  // ── Folds: a route fragment or an in-page click opens a leg's fold ─────────

  private readonly legClicks = new Subject<LegAnchor>();
  private readonly fragment$ = this.route.fragment ?? of(null);

  /** A new object per request, so arriving at the same leg twice re-opens it. */
  readonly openFold = toSignal<FoldRequest | null>(
    merge(
      combineLatest([this.route.paramMap, this.fragment$]).pipe(map(([, f]) => legFromFragment(f))),
      this.legClicks
    ).pipe(map(leg => (leg ? { leg } : null))),
    { initialValue: null }
  );

  /** The atom whose network detail has been asked for (lazy: holds fold or #holds). */
  private readonly resilienceWantedFor = signal<string | null>(null);

  /**
   * `undefined` until the first snapshot request for the current atom
   * resolves (or errors) — distinguishes "not yet asked" from "asked,
   * nothing found" (spec F5). Leg loaders map a failure to `null`, which is
   * itself a resolved (known) answer.
   */
  readonly snapshot = toSignal<ResilienceSnapshotView | null | undefined>(
    this.atomId$.pipe(
      switchMap(id =>
        id ? this.resilience.getSnapshot(id).pipe(catchError(() => of(null))) : of(null)
      )
    ),
    { initialValue: undefined }
  );

  private readonly distributionSummary = toSignal<CopiesSummary | null>(
    toObservable(this.atom).pipe(
      map(a => a?.blobHash ?? null),
      distinctUntilChanged(),
      switchMap(hash =>
        hash
          ? from(this.distribution.getDetails(hash)).pipe(
              map(
                (d): CopiesSummary => ({
                  has: d.summary.replicaCount,
                  wants: d.summary.replicaTarget,
                  health: d.summary.replicaHealth,
                })
              ),
              catchError(() => of(null))
            )
          : of(null)
      )
    ),
    { initialValue: null }
  );

  readonly peersHolding = computed(() => this.distributionSummary()?.has ?? null);
  readonly copies = computed(() => {
    const c = this.distributionSummary();
    return c && c.wants > 0 ? c : null;
  });

  readonly resilienceDetailView = toSignal<ResilienceView | null | undefined>(
    combineLatest([this.atomId$, toObservable(this.resilienceWantedFor)]).pipe(
      map(([id, wanted]) => (id && id === wanted ? id : null)),
      distinctUntilChanged(),
      switchMap(id =>
        id
          ? this.resilienceDetail.getResilience(id).pipe(
              catchError(() => of(null)),
              startWith(undefined)
            )
          : of(undefined)
      )
    ),
    { initialValue: undefined }
  );

  private readonly verifyState = signal<{ id: string; state: VerifyState } | null>(null);
  readonly verification = computed<VerifyState>(() => {
    const v = this.verifyState();
    return v && v.id === this.atom()?.id ? v.state : { state: 'idle' };
  });

  readonly stewards = toSignal(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.storageApi.getContentStewardship(id).pipe(
              map(view =>
                view.allocations.map(
                  (r): StewardRow => ({
                    stewardPresenceId: r.stewardPresenceId,
                    displayName: r.steward?.displayName ?? null,
                    contributionType: r.contributionType,
                    effectiveFrom: r.effectiveFrom,
                    allocationRatio: r.allocationRatio,
                    recognitionAccumulated: r.recognitionAccumulated,
                    disputed: !!r.disputeId || !!r.disputedAt,
                    disputeReason: r.disputeReason,
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

  private readonly challengesRefresh = signal(0);
  readonly challenges = toSignal(
    combineLatest([this.atomId$, toObservable(this.challengesRefresh)]).pipe(
      switchMap(([id]) =>
        id
          ? from(this.governance.getChallengesForEntity('content', id)).pipe(
              catchError(() => of<ChallengeView[]>([]))
            )
          : of<ChallengeView[]>([])
      )
    ),
    { initialValue: [] as ChallengeView[] }
  );

  /** Null until the governance-state endpoint answers (404 → null, silent — spec §8). */
  readonly governanceState = toSignal<GovernanceStateView | null>(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? from(this.governance.getGovernanceState('content', id)).pipe(catchError(() => of(null)))
          : of(null)
      )
    ),
    { initialValue: null }
  );

  /**
   * Attestations name the content by its CID (subject_cid); the slug is asked
   * too because no content-quality minter has fixed the key yet (see lamad
   * data-loader "KEYING CAVEAT"). Rows are de-duplicated by attestation id.
   */
  readonly attestations = toSignal<AttestationView[] | null | undefined>(
    toObservable(this.atom).pipe(
      map(a => (a ? { id: a.id, cid: a.blobCid } : null)),
      distinctUntilChanged((x, y) => x?.id === y?.id && x?.cid === y?.cid),
      switchMap(keys => {
        if (!keys) return of(undefined);
        const subjects = [keys.id, ...(keys.cid && keys.cid !== keys.id ? [keys.cid] : [])];
        return forkJoin(subjects.map(s => from(this.attestationApi.listBySubject(s)))).pipe(
          map(lists => {
            const seen = new Set<string>();
            return lists.flat().filter(a => !seen.has(a.id) && !!seen.add(a.id));
          }),
          catchError(() => of(null)),
          startWith(undefined)
        );
      })
    ),
    { initialValue: undefined }
  );

  readonly seeAlso = toSignal<SeeAlsoRow[] | null | undefined>(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? // The computed graph carries both kinds: authored edges arrive marked
            // `explicit`, discovered ones by how they were found (`tag`, …). The
            // persisted /db/relationships list is not read here: on the live
            // doorway it ignores `sourceId` and answers a paged envelope.
            this.http
              .get<ContentGraphView>(
                `${this.storage.getStorageBaseUrl()}/db/relationships/graph/${encodeURIComponent(id)}?computed=true`
              )
              .pipe(
                map(graph => mergeSeeAlso([], graph)),
                catchError(() => of(null)),
                startWith(undefined)
              )
          : of(undefined)
      )
    ),
    { initialValue: undefined }
  );

  readonly contributors = toSignal(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.storageApi
              .getPresenceForContent(id)
              .pipe(catchError(() => of<ContributorPresenceView[]>([])))
          : of<ContributorPresenceView[]>([])
      )
    ),
    { initialValue: [] as ContributorPresenceView[] }
  );

  readonly engagement = toSignal<ContentEngagementStatsView | null | undefined>(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? this.storage.getContentEngagement(id).pipe(
              catchError(() => of(null)),
              startWith(undefined)
            )
          : of(undefined)
      )
    ),
    { initialValue: undefined }
  );

  private readonly signalsRefresh = signal(0);
  readonly signalAggregate = toSignal<SignalAggregateView | null>(
    combineLatest([this.atomId$, toObservable(this.signalsRefresh)]).pipe(
      switchMap(([id]) =>
        id
          ? from(this.governance.getSignalAggregate('content', id)).pipe(catchError(() => of(null)))
          : of(null)
      )
    ),
    { initialValue: null }
  );
  readonly reactionCounts = computed(() => reactionCounts(this.signalAggregate()));
  readonly landedLine = computed(
    () => landedSummary(this.signalAggregate()) ?? 'No one has reacted here yet.'
  );

  private readonly discussionsRaw = toSignal(
    this.atomId$.pipe(
      switchMap(id =>
        id
          ? from(this.governance.queryDiscussions(id)).pipe(
              catchError(() => of<DiscussionView[]>([]))
            )
          : of<DiscussionView[]>([])
      )
    ),
    { initialValue: [] as DiscussionView[] }
  );

  /** Top-level conversations, newest first, each with its row title in words. */
  readonly discussions = computed(() =>
    this.discussionsRaw()
      .filter(d => !d.parentId)
      .map(d => ({
        id: d.id,
        title: discussionTitle(d),
        category: d.category,
        status: d.status,
        messages: d.messageCount ?? 0,
        at: d.lastActivityAt ?? d.updatedAt ?? d.createdAt,
      }))
      .sort((a, b) => (a.at < b.at ? 1 : -1))
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
      a.version ? `v${a.version}` : null,
      a.estimatedTime,
      a.author ? `by ${a.author}` : null,
      a.license,
    ].filter((x): x is string => !!x);
  });
  readonly address = computed(() => eprToUniversalHref({ id: this.resourceId(), tier: 'head' }));
  readonly rawHref = computed(() =>
    eprToUniversalHref({ id: this.resourceId(), tier: 'head', subview: 'raw' })
  );
  /** The in-page address of "Who holds it" with its network detail open. */
  readonly holdsHref = computed(() => `${this.address()}#holds`);

  /** The cross-bundle "Open in <Bundle>" lens — minted from the generated all-bundle claims. */
  readonly lens = computed(() => {
    const a = this.atom();
    return a ? openInBundle(a.contentType, a.id) : null;
  });

  readonly signedIn = this.auth.isAuthenticated;
  /** Writes are made AS the signed-in human; no human id → no write affordance. */
  private readonly humanId = computed(() =>
    this.auth.isAuthenticated() ? (this.auth.humanId() ?? null) : null
  );
  readonly canWrite = computed(() => !!this.humanId());

  /** "Edit in <Bundle>" — signed in only, never for a generated path-step id. */
  readonly editLens = computed(() => {
    const a = this.atom();
    return a && this.signedIn() ? editInBundle(a.id) : null;
  });

  readonly downloadHref = computed(() => {
    const hash = this.atom()?.blobHash;
    return hash ? this.storage.getBlobUrl(hash) : null;
  });

  readonly concernNote = signal<string | null>(null);

  readonly focused = signal(false);

  /**
   * Where you actually came from — the previous stop on the session nav
   * stack, or nothing. A loaded atom records ITSELF on load, so its own
   * entry is now the top of the stack and `previous()` (second-to-last)
   * correctly names the prior stop. The gate never records itself (an
   * unreachable id isn't a place to come back to — spec §12.2), so nothing
   * pushed a "self" entry for it to skip past: the top of the stack IS the
   * prior stop, read directly.
   */
  readonly arrival = computed(() => {
    let prev =
      this.status() === 'not-found'
        ? (this.navStack.entries().at(-1) ?? null)
        : this.navStack.previous();
    // An atom that was viewed, recorded, then became unreachable (or a race
    // that records this very gate) would otherwise have the top-of-stack
    // entry point right back at the gate's own address. The gate must never
    // name itself as its own arrival.
    if (prev && this.status() === 'not-found' && prev.url === this.address()) {
      prev = null;
    }
    if (!prev) return null;
    const label = (prev.label ?? prev.url).replace(/ \| Elohim Protocol$/, '').trim();
    return { href: prev.url, label: label || prev.url };
  });

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

  /**
   * The reaction bar's `reaction-submit` event, recorded as the signed-in
   * human. (Angular refuses a binding to an `on*` property, so the element's
   * event is the seam rather than its `onReactionSubmit` callback.)
   */
  onReaction(event: Event): void {
    const detail = (event as CustomEvent<ReactionSubmitEvent>).detail;
    const humanId = this.humanId();
    if (!humanId || !detail?.type) return;
    this.governance
      .recordSignal(reactionSignal(detail, humanId))
      .then(() => this.signalsRefresh.update(n => n + 1))
      .catch(() => {
        // A reaction that did not land leaves the counts as they were.
      });
  }

  /** The atom whose witnessed view is open, so leaving it closes the right one. */
  private witnessed: string | null = null;

  constructor() {
    // A legacy viewer fragment (#network, #governance, #trust) can arrive here
    // without passing the client-side redirect: the server answers
    // /resource/{id} with a 302 to /epr/{id}, and the browser carries the old
    // fragment across it. Rename it in place to the leg it opens.
    this.fragment$.pipe(takeUntilDestroyed(this.destroyRef)).subscribe(fragment => {
      const leg = legFromFragment(fragment);
      if (fragment && leg && leg !== fragment) {
        void this.router.navigate([], {
          relativeTo: this.route,
          fragment: leg,
          queryParamsHandling: 'preserve',
          replaceUrl: true,
        });
      }
    });

    effect(() => {
      this.resourceId();
      untracked(() => {
        this.concernNote.set(null);
        this.verifyState.set(null);
        this.resilienceWantedFor.set(null);
        this.exitFocus();
      });
    });

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

    // Network detail is read only when someone asks for it: the holds fold
    // opened by its toggle, or an arrival on #holds.
    effect(() => {
      const req = this.openFold();
      const id = this.atom()?.id;
      if (req?.leg === 'holds' && id) this.resilienceWantedFor.set(id);
    });

    // The witnessed view (dwell + scroll depth, posted on leave): one open
    // view per loaded atom; moving to another atom or leaving closes it.
    effect(() => {
      const id = this.atom()?.id ?? null;
      if (id === this.witnessed) return;
      if (this.witnessed) this.observationEmitter.end(this.witnessed);
      this.witnessed = id;
      if (id) {
        this.observationEmitter.begin(id);
        // A claimed atom renders no focal renderer, so nothing announces the
        // content is in the DOM; measure once after this render instead.
        if (this.lens()) this.probeDepthAfterRender();
      }
    });

    afterNextRender(() => {
      // Window scroll is the reading depth; passive, it never blocks scrolling.
      const win = this.document.defaultView;
      win?.addEventListener('scroll', this.onWindowScroll, { passive: true });
      // With Focus on, the frame itself is the scroller (fixed, overflow:auto)
      // and a scroll event does not bubble, so listen in the capture phase.
      const host = this.elRef.nativeElement;
      host.addEventListener('scroll', this.onContainerScroll, { capture: true, passive: true });
      this.destroyRef.onDestroy(() => {
        win?.removeEventListener('scroll', this.onWindowScroll);
        host.removeEventListener('scroll', this.onContainerScroll, { capture: true });
      });
    });

    this.destroyRef.onDestroy(() => {
      if (this.witnessed) this.observationEmitter.end(this.witnessed);
      this.witnessed = null;
      this.document.body?.classList.remove(FOCUSED_VIEW_BODY_CLASS);
    });
  }

  onArrival(event: Event, href: string): void {
    event.preventDefault();
    this.eprNav.navigate(href);
  }

  /** Open a leg's fold in place (the href carries the same #leg for a new tab or a share). */
  openLeg(event: Event, leg: LegAnchor): void {
    event.preventDefault();
    this.legClicks.next(leg);
  }

  onFoldOpened(leg: FoldLeg): void {
    const id = this.atom()?.id;
    if (leg === 'holds' && id) this.resilienceWantedFor.set(id);
  }

  onVerify(): void {
    const id = this.atom()?.id;
    if (!id) return;
    this.verifyState.set({ id, state: { state: 'checking' } });
    this.resilienceDetail
      .verifyResilience(id)
      .pipe(take(1), takeUntilDestroyed(this.destroyRef))
      .subscribe({
        next: result => {
          if (this.resourceId() === id)
            this.verifyState.set({ id, state: { state: 'done', result } });
        },
        error: () => {
          if (this.resourceId() === id) this.verifyState.set({ id, state: { state: 'failed' } });
        },
      });
  }

  /** A concern raised through the gate: a challenge is filed; every other kind is a signal. */
  onConcern(concern: ConcernSubmit): void {
    const id = this.atom()?.id;
    const humanId = this.humanId();
    if (!id || !humanId) return;
    if (concern.type === 'challenge') {
      this.governance
        .fileChallenge(challengeInput(id, concern.text, humanId))
        .then(() => {
          if (this.resourceId() !== id) return;
          this.concernNote.set('Your challenge was filed. Its steward is asked to respond.');
          this.challengesRefresh.update(n => n + 1);
        })
        .catch(() => {
          if (this.resourceId() === id) this.concernNote.set("Couldn't file this from here.");
        });
    } else {
      const kind: SignalConcernKind = concern.type;
      this.governance
        .recordSignal(concernSignal(kind, id, concern.text, humanId))
        .then(() => {
          if (this.resourceId() !== id) return;
          this.concernNote.set(CONCERN_RECORDED[kind]);
          this.signalsRefresh.update(n => n + 1);
        })
        .catch(() => {
          if (this.resourceId() === id) this.concernNote.set("Couldn't send this from here.");
        });
    }
  }

  onFocalLoaded(): void {
    this.probeDepthAfterRender();
  }

  toggleFocus(): void {
    this.setFocus(!this.focused());
  }

  exitFocus(): void {
    if (this.focused()) this.setFocus(false);
  }

  markPracticing(): void {
    const a = this.atom();
    if (a) this.affinityService.setAffinity(a.id, 0.5);
  }

  markGotIt(): void {
    const a = this.atom();
    if (a) this.affinityService.setAffinity(a.id, 1);
  }

  private setFocus(on: boolean): void {
    this.focused.set(on);
    this.document.body?.classList.toggle(FOCUSED_VIEW_BODY_CLASS, on);
    this.probeDepthAfterRender();
  }

  private readonly onWindowScroll = (): void => {
    this.reportWindowDepth();
  };

  private readonly onContainerScroll = (event: Event): void => {
    const target = event.target;
    if (target instanceof HTMLElement && target.classList.contains(FOCUS_SCROLLER_CLASS)) {
      this.reportElementDepth(target);
    }
  };

  /**
   * Measure depth where the person is reading now — the frame when Focus makes
   * it the scroller, else the window — once the view has rendered, so a page
   * shorter than its viewport reports 100 without any scroll (R-A11).
   */
  private probeDepthAfterRender(): void {
    afterNextRender(
      () => {
        const scroller = this.focused()
          ? this.elRef.nativeElement.querySelector<HTMLElement>(`.${FOCUS_SCROLLER_CLASS}`)
          : null;
        if (scroller) {
          this.reportElementDepth(scroller);
        } else {
          this.reportWindowDepth();
        }
      },
      { injector: this.injector }
    );
  }

  private reportWindowDepth(): void {
    const win = this.document.defaultView;
    if (!win) return;
    const root = this.document.documentElement;
    this.observationEmitter.noteScroll(
      scrollDepthPct(win.scrollY || root.scrollTop, win.innerHeight, root.scrollHeight)
    );
  }

  private reportElementDepth(el: HTMLElement): void {
    this.observationEmitter.noteScroll(
      scrollDepthPct(el.scrollTop, el.clientHeight, el.scrollHeight)
    );
  }
}
