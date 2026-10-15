import { CommonModule } from '@angular/common';
import {
  CUSTOM_ELEMENTS_SCHEMA,
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  input,
  linkedSignal,
  output,
} from '@angular/core';
import { RouterModule } from '@angular/router';

import 'elohim-core/register';
import 'elohim-imagodei/register';

import { eprToUniversalHref } from '@elohim/service';

import { EprRelationship } from '../../models/epr-head.model';
import { EprRelationshipsPanelComponent } from '../epr-relationships-panel/epr-relationships-panel.component';

import {
  EprHomeAtom,
  FoldLeg,
  LegAnchor,
  StewardRow,
  attentionLine,
  daysRemainingWords,
  dayWords,
  healthWords,
  holdingWords,
  labelWords,
  shortAnchor,
  sizeWords,
  verifyLine,
} from './epr-home.model';

import type { AttestationView } from '@app/generated/attestation-view';
import type { ResilienceSnapshotView } from '@app/generated/resilience-snapshot-view';
import type {
  ChallengeView,
  ContentEngagementStatsView,
  ContributorPresenceView,
  GovernanceStateView,
  RelationshipWithContentView,
  VerificationResultView,
} from '@elohim/storage-client/generated';
import type { ResilienceView } from '@elohim/storage-client/generated/ResilienceView';
import type { GateFeedbackType } from 'elohim-core';

/** "concept-bidirectional-trust" → "Bidirectional trust" (label absent → humanized slug). */
export function humanizeSlug(slug: string): string {
  const words = slug
    .replace(/^(concept|fct-module-\d+)-/, '')
    .split('-')
    .filter(Boolean);
  return words.map((w, i) => (i === 0 ? w.charAt(0).toUpperCase() + w.slice(1) : w)).join(' ');
}

/** A request to open one leg's fold (a new object each time, so a repeat re-opens). */
export interface FoldRequest {
  leg: LegAnchor;
}

/** The "is it still whole" check, as the frame tracks it. */
export type VerifyState =
  | { state: 'idle' }
  | { state: 'checking' }
  | { state: 'done'; result: VerificationResultView }
  | { state: 'failed' };

/** Copies of the bytes, as the distribution summary states them. */
export interface CopiesSummary {
  has: number;
  wants: number;
  health: string;
}

/** A concern raised through the gate trigger. */
export interface ConcernSubmit {
  type: GateFeedbackType;
  text: string;
}

/**
 * The legs of the atom, in household words (spec §2.2), each with its fold
 * (§11.1) — closed by default, opened by its toggle or by `openFold` naming
 * the leg. Presentational: everything arrives as inputs; the frame owns
 * loading and every write.
 *
 * `undefined` on a loadable input = still checking; `null` = could not read it.
 */
@Component({
  selector: 'app-epr-home-legs',
  standalone: true,
  imports: [CommonModule, RouterModule, EprRelationshipsPanelComponent],
  schemas: [CUSTOM_ELEMENTS_SCHEMA],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './epr-home-legs.component.html',
  styleUrl: './epr-home-legs.component.css',
})
export class EprHomeLegsComponent {
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly injector = inject(Injector);

  readonly atom = input.required<EprHomeAtom>();
  /** `undefined` = not yet asked; `null` = asked, nothing found (spec F5). */
  readonly snapshot = input<ResilienceSnapshotView | null | undefined>(undefined);
  readonly stewards = input<StewardRow[]>([]);
  readonly relationships = input<EprRelationship[]>([]);
  readonly challenges = input<ChallengeView[]>([]);
  readonly peersHolding = input<number | null>(null);
  readonly copies = input<CopiesSummary | null>(null);
  readonly resilience = input<ResilienceView | null | undefined>(undefined);
  readonly verification = input<VerifyState>({ state: 'idle' });
  readonly attestations = input<AttestationView[] | null | undefined>(undefined);
  readonly governanceState = input<GovernanceStateView | null>(null);
  readonly seeAlso = input<RelationshipWithContentView[] | null | undefined>(undefined);
  readonly contributors = input<ContributorPresenceView[]>([]);
  readonly engagement = input<ContentEngagementStatsView | null | undefined>(undefined);
  /** True when a write can be made as the signed-in person (a session with a human id). */
  readonly canWrite = input(false);
  readonly concernNote = input<string | null>(null);
  readonly downloadHref = input<string | null>(null);
  readonly openFold = input<FoldRequest | null>(null);

  readonly foldOpened = output<FoldLeg>();
  readonly verify = output<void>();
  readonly concern = output<ConcernSubmit>();

  /** Which folds are open. A new request adds its leg; toggles flip one leg. */
  readonly open = linkedSignal<FoldRequest | null, ReadonlySet<string>>({
    source: this.openFold,
    computation: (req, previous) => {
      const next = new Set(previous?.value ?? []);
      if (req && req.leg !== 'here') next.add(req.leg);
      return next;
    },
  });

  readonly holding = computed(() => holdingWords(this.snapshot()));
  readonly pips = computed(() =>
    Array.from({ length: this.holding().wants }, (_, i) => i < this.holding().has)
  );
  readonly related = computed(() =>
    this.atom().relatedIds.map(id => ({
      id,
      label: humanizeSlug(id),
      href: eprToUniversalHref({ id, tier: 'head' }),
    }))
  );
  readonly openChallenges = computed(() =>
    this.challenges().filter(c => c.state !== 'responded' && !c.respondedAt && !c.resolvedAt)
  );
  readonly labels = computed(() => labelWords(this.governanceState()?.labels));
  readonly labelsReviewed = computed(() => {
    const at = this.governanceState()?.lastUpdated ?? this.governanceState()?.updatedAt;
    return at ? dayWords(at) : null;
  });
  readonly anchorShort = computed(() => {
    const h = this.atom().dhtAnchorHash;
    return h ? shortAnchor(h) : null;
  });
  readonly anchorVerified = computed(() => this.atom().dhtAnchorState === 'verified');
  readonly addedOn = computed(() => dayWords(this.atom().createdAt));
  readonly updatedOn = computed(() => dayWords(this.atom().updatedAt));
  readonly rawHref = computed(() =>
    eprToUniversalHref({ id: this.atom().id, tier: 'head', subview: 'raw' })
  );
  readonly sourceLabel = computed(() => {
    const s = this.atom().sourceUrl ?? this.atom().canonicalUrl;
    return s ? s.replace(/^https?:\/\//, '').replace(/\/$/, '') : null;
  });

  readonly health = computed(() => {
    const r = this.resilience();
    return r ? healthWords(r) : null;
  });
  readonly storageCommitted = computed(() => {
    const r = this.resilience();
    return r && Number(r.commitments.totalCommittedBytes) > 0
      ? sizeWords(r.commitments.totalCommittedBytes)
      : null;
  });
  readonly verifyWords = computed(() => {
    const v = this.verification();
    if (v.state === 'done') return verifyLine(v.result);
    if (v.state === 'failed') return "Couldn't run the check from here.";
    return null;
  });
  readonly attention = computed(() => attentionLine(this.engagement()));
  readonly seeAlsoRows = computed(() =>
    (this.seeAlso() ?? []).map(r => ({
      id: r.id,
      targetId: r.targetId,
      title: r.target?.title ?? humanizeSlug(r.targetId),
      kind: r.relationshipType.replace(/_/g, ' ').toLowerCase(),
      inference: r.inferenceSource === 'explicit' ? 'explicit' : 'discovered',
      href: eprToUniversalHref({ id: r.targetId, tier: 'head' }),
    }))
  );

  constructor() {
    // Arriving with a leg's fragment (or the footer's Network detail) brings
    // that leg into view; the router does no anchor scrolling here.
    effect(() => {
      const req = this.openFold();
      if (!req) return;
      afterNextRender(
        () => {
          const el = this.host.nativeElement.querySelector<HTMLElement>(`#${req.leg}`);
          if (el && typeof el.scrollIntoView === 'function') {
            el.scrollIntoView({ block: 'start' });
          }
        },
        { injector: this.injector }
      );
    });
  }

  isOpen(leg: FoldLeg): boolean {
    return this.open().has(leg);
  }

  toggle(leg: FoldLeg): void {
    const next = new Set(this.open());
    if (next.has(leg)) {
      next.delete(leg);
    } else {
      next.add(leg);
      this.foldOpened.emit(leg);
    }
    this.open.set(next);
  }

  onConcern(event: Event): void {
    const detail = (event as CustomEvent<{ type: GateFeedbackType; text: string }>).detail;
    if (detail?.text) this.concern.emit({ type: detail.type, text: detail.text });
  }

  contribution(row: StewardRow): string {
    return row.contributionType.replace(/_/g, ' ');
  }

  since(row: StewardRow): string {
    return dayWords(row.effectiveFrom);
  }

  /** "60% share" — a share of stewardship, stated by the allocation. */
  share(row: StewardRow): string | null {
    return row.allocationRatio === undefined
      ? null
      : `${Math.round(row.allocationRatio * 100)}% share`;
  }

  recognition(row: StewardRow): string | null {
    return row.recognitionAccumulated === undefined
      ? null
      : `${row.recognitionAccumulated} recognition`;
  }

  challengeDays(c: ChallengeView): string | null {
    return daysRemainingWords(c);
  }

  filedOn(c: ChallengeView): string {
    return dayWords(c.filedAt);
  }

  grounds(c: ChallengeView): string {
    return c.groundsPrimary.replace(/_/g, ' ');
  }

  attestedOn(a: AttestationView): string {
    return dayWords(a.createdAt);
  }

  issuerShort(a: AttestationView): string {
    return shortAnchor(a.issuerCid);
  }

  proofWords(a: AttestationView): string {
    return a.proofClass.replace(/-/g, ' ');
  }

  replicaHealthWords(health: string): string {
    return health.replace(/_/g, ' ');
  }
}
