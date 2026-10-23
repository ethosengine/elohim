import { HttpClient } from '@angular/common/http';
import { Component, EventEmitter, Input, Output, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import {
  ActivatedRoute,
  ParamMap,
  Router,
  convertToParamMap,
  provideRouter,
} from '@angular/router';

import { BehaviorSubject, Observable, Subject, of, throwError } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { ObservationEmitterService } from '@elohim/rea-runtime';
import { GovernanceApiService } from '@elohim/service';
import { DistributionService, ResilienceService } from '@elohim/service/public-api';

import { EprHomeComponent } from './epr-home.component';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';
import { AuthService } from '../../../imagodei/services/auth.service';
import { SeoService } from '../../../services/seo.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { AttestationApiService } from '../../services/attestation-api.service';
import { EprResolverService } from '../../services/epr-resolver.service';
import { ResilienceDetailService } from '../../services/resilience-detail.service';
import { SessionNavStackService } from '../../services/session-nav-stack.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageClientService } from '../../services/storage-client.service';

import {
  CHROME_SECTION_IDS,
  DELIVERY_VALUE_MAX,
  anchorWords,
  attentionLine,
  completionWords,
  daysRemainingWords,
  appPermissions,
  deliveryValue,
  discussionTitle,
  focalShape,
  healthWords,
  heldChip,
  holdingWords,
  hopWords,
  labelWords,
  legFromFragment,
  parseDeliveryPath,
  reachSubtitle,
  requestFromFragment,
  shortAnchor,
  toAtom,
  verifyLine,
} from './epr-home.model';

describe('epr-home.model', () => {
  it('shapes the focal slot by contentFormat', () => {
    expect(focalShape('html5-app')).toBe('immersive');
    expect(focalShape('markdown')).toBe('reading');
    expect(focalShape('')).toBe('reading');
  });

  it('says the anchor state in words', () => {
    expect(anchorWords('notarized', 'unverified')).toBe('anchor not yet verified here');
    expect(anchorWords('notarized', 'verified')).toBe('anchor verified here');
    expect(anchorWords(null, null)).toBe('Not yet notarized');
  });

  it('projects the raw wire shape without reshaping identity', () => {
    const atom = toAtom({
      id: 'evolution-of-trust',
      title: 'The Evolution of Trust',
      contentType: 'collective',
      contentFormat: 'html5-app',
      reach: 'commons',
      trust: 'notarized',
      dhtAnchorHash: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
      dhtAnchorState: 'unverified',
      tags: ['game-theory', 'trust', ''],
      metadata: {
        author: 'Nicky Case',
        license: 'CC0 Public Domain',
        relatedNodeIds: ['a', 'b'],
        version: 3,
      },
    });
    expect(atom.shape).toBe('immersive');
    expect(atom.author).toBe('Nicky Case');
    expect(atom.relatedIds).toEqual(['a', 'b']);
    expect(atom.tags).toEqual(['game-theory', 'trust']);
    expect(atom.version).toBe('3');
    expect(reachSubtitle(atom.reach)).toBe('anyone can reach this');
    expect(shortAnchor(atom.dhtAnchorHash!)).toBe('uhCkk_D-fLh9…KujiEm8f');
  });

  it('renders the felt status as one verdict in household words', () => {
    const words = holdingWords({
      contentId: 'x',
      feltStatus: {
        headline: 'Held by only 1 household — invite another to help hold these',
        reassurance: 'needs-help',
        heldBy: [
          {
            id: 'household-dowell',
            kind: 'household',
            label: 'Dowell Household',
            intraHubPeers: 2,
          },
        ],
        floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 1 },
        suggestedAction: 'Invite a household to help hold these',
      },
    } as never);
    expect(words.has).toBe(1);
    expect(words.warm).toBe(true);
    expect(words.state).toBe('known');
    expect(words.households).toEqual(['Dowell Household · 2 peers']);
    expect(heldChip(words)).toBe('Held by 1 of 3 households');
    expect(heldChip(holdingWords(null))).toBe('Not yet held by any household');
  });

  it('distinguishes "not yet asked" (undefined) from "asked, nothing found" (null)', () => {
    const notYetAsked = holdingWords(undefined);
    expect(notYetAsked.state).toBe('unknown');
    expect(notYetAsked.headline).toBe('Checking who holds this…');
    expect(notYetAsked.has).toBe(0);
    expect(notYetAsked.action).toBe('');
    expect(heldChip(notYetAsked)).toBe('Checking who holds this…');

    const askedNothingFound = holdingWords(null);
    expect(askedNothingFound.state).toBe('known');
    expect(askedNothingFound.headline).toBe("We can't confirm this is backed up anywhere yet.");
    expect(heldChip(askedNothingFound)).toBe('Not yet held by any household');
  });

  it("opens the leg a fragment names, mapping the retired viewer's tabs", () => {
    expect(legFromFragment('network')).toBe('holds');
    expect(legFromFragment('governance')).toBe('governed');
    expect(legFromFragment('trust')).toBe('from');
    expect(legFromFragment('content')).toBeNull();
    for (const leg of ['holds', 'lives', 'governed', 'from', 'here']) {
      expect(legFromFragment(leg)).toBe(leg);
    }
    expect(legFromFragment('step/2')).toBeNull();
    expect(legFromFragment(null)).toBeNull();
    expect(legFromFragment('toString')).toBeNull();
  });

  it("maps the content chrome's card sections onto this page, and only those it carries", () => {
    expect(requestFromFragment('source')).toEqual({ leg: null, anchor: 'source' });
    expect(requestFromFragment('delivery')).toEqual({ leg: null, anchor: 'delivery' });
    expect(requestFromFragment('holders')).toEqual({ leg: 'holds', anchor: 'holds' });
    expect(requestFromFragment('steward')).toEqual({ leg: 'governed', anchor: 'governed' });
    expect(requestFromFragment('claims')).toEqual({ leg: 'from', anchor: 'claims' });
    expect(requestFromFragment('yours')).toEqual({ leg: null, anchor: 'yours' });
    expect(requestFromFragment('permissions')).toEqual({ leg: null, anchor: 'permissions' });
    expect(requestFromFragment('network')).toEqual({ leg: 'holds' });
    expect(requestFromFragment('holds')).toEqual({ leg: 'holds' });
    expect(requestFromFragment(null)).toBeNull();
    expect(requestFromFragment('toString')).toBeNull();
    expect(CHROME_SECTION_IDS).toContain('permissions');
  });

  it("keeps the link's delivery claim honest: absent parts stay absent, values are capped", () => {
    const path = (q: Record<string, string>) => parseDeliveryPath(convertToParamMap(q));
    expect(path({})).toEqual([]);
    expect(path({ via: '', peer: '   ' })).toEqual([]);
    expect(path({ via: 'doorway-alpha', peer: 'peer-jessica' })).toEqual([
      { role: 'doorway', id: 'doorway-alpha' },
      { role: 'holder', peer: 'peer-jessica' },
    ]);
    expect(path({ via: 'doorway-alpha' })).toEqual([{ role: 'doorway', id: 'doorway-alpha' }]);
    expect(path({ peer: 'peer-jessica' })).toEqual([{ role: 'holder', peer: 'peer-jessica' }]);
    const long = 'x'.repeat(DELIVERY_VALUE_MAX + 50);
    expect(deliveryValue(long)).toBe(`${'x'.repeat(DELIVERY_VALUE_MAX)}…`);
    expect(deliveryValue('x'.repeat(DELIVERY_VALUE_MAX))).toBe('x'.repeat(DELIVERY_VALUE_MAX));
    // Bidi overrides and controls cannot reorder or hide the surrounding text.
    expect(deliveryValue('alpha\u202Eahpla\u0000')).toBe('alphaahpla');
    expect(deliveryValue('\u202E')).toBeNull();
  });

  it("shows the chrome facing's app lens as vocabulary rows", () => {
    const facing = (app?: Record<string, unknown>) =>
      ({ ...chromeFacing(), app }) as unknown as Parameters<typeof appPermissions>[0];
    // Not read yet: no section. Read failed: claim nothing for an html5-app only.
    expect(appPermissions(undefined, 'html5-app')).toBeNull();
    expect(appPermissions(null, 'html5-app')).toBe('unreadable');
    expect(appPermissions(null, 'markdown')).toBeNull();
    // No app lens: not an app bundle.
    expect(appPermissions(facing(), 'markdown')).toBeNull();
    const perm = appPermissions(
      facing({
        entry: 'index.html',
        capabilities: ['forms', 'camera'],
        presents: false,
        sources: ['https://fonts.example'],
      }),
      'html5-app'
    ) as { capabilities: { value: string; label: string; device: boolean }[]; sources: string[] };
    expect(perm.capabilities.map(r => [r.value, r.label, r.device])).toEqual([
      ['forms', 'Submit forms', false],
      ['camera', 'Camera', true],
    ]);
    expect(perm.sources).toEqual(['https://fonts.example']);
    expect(
      appPermissions(
        facing({ entry: 'index.html', capabilities: [], presents: true }),
        'spa-bundle'
      )
    ).toEqual({ capabilities: [], sources: [] });
  });

  it('says each delivery hop in words, the value kept apart as the link wrote it', () => {
    expect(hopWords({ role: 'self' })).toEqual({ lead: 'from your own peer', value: null });
    expect(hopWords({ role: 'doorway', id: 'doorway-alpha' })).toEqual({
      lead: 'through doorway',
      value: 'doorway-alpha',
    });
    expect(hopWords({ role: 'holder', peer: 'peer-jessica' })).toEqual({
      lead: 'from peer',
      value: 'peer-jessica',
    });
    expect(hopWords({ role: 'holder', peer: 'p', transport: 'iroh' }).lead).toBe(
      'from peer (over iroh)'
    );
  });

  it("says the storage peer's health in words first", () => {
    const view = (status: string, canSurviveFailures: number) =>
      ({ health: { status, canSurviveFailures, score: 1 } }) as never;
    expect(healthWords(view('healthy', 2))).toEqual({
      line: 'Healthy · can lose 2 peers and still be whole',
      warm: false,
    });
    expect(healthWords(view('at_risk', 0))).toEqual({
      line: 'At risk · losing one more peer could leave it incomplete',
      warm: true,
    });
    expect(healthWords(view('degraded', 1)).line).toBe(
      'Thinning · can lose 1 peer and still be whole'
    );
  });

  it('says the whole-check result in one line, carrying the error when there is one', () => {
    const base = {
      contentId: 'x',
      encoding: 'rs',
      shardsAvailable: 1,
      originalHash: '',
      reconstructedHash: '',
      hashMatch: true,
    };
    expect(
      verifyLine({
        ...base,
        verified: true,
        shardsNeeded: 1,
        shardsLocated: 1,
        shardsMissing: 0,
        reconstructionTimeMs: 12,
        error: null,
      } as never)
    ).toBe('Whole · 1 of 1 piece found · 12 ms');
    expect(
      verifyLine({
        ...base,
        verified: false,
        shardsNeeded: 4,
        shardsLocated: 2,
        shardsMissing: 2,
        reconstructionTimeMs: 0,
        error: 'peer unreachable',
      } as never)
    ).toBe('Not whole · 2 pieces missing · peer unreachable');
  });

  it('says who has been counted here in words, never a percentage', () => {
    expect(attentionLine(null)).toBe('No one has been counted here yet.');
    expect(attentionLine({ views: 0 } as never)).toBe('No one has been counted here yet.');
    const line = attentionLine({
      contentId: 'x',
      views: 9,
      uniqueViewers: 6,
      completions: 2,
      completionRate: 0.34,
      computedAt: '',
    } as never);
    expect(line).toBe(
      'Viewed 9 times by 6 people · 2 completions · about 1 in 3 who viewed it finished'
    );
    expect(line).not.toMatch(/%/);
    expect(completionWords(0)).toBeNull();
    expect(completionWords(0.97)).toBe('nearly everyone who viewed it finished');
  });

  it('reads governance labels and challenge deadlines in words', () => {
    expect(labelWords([{ labelType: 'needs_review', severity: 'low' }, 'disputed-source'])).toEqual(
      ['needs review', 'disputed source']
    );
    expect(labelWords(null)).toEqual([]);
    const now = new Date('2026-08-05T00:00:00').getTime();
    expect(
      daysRemainingWords(
        { responseDeadline: '2026-08-08 00:00:00', slaDeadline: null } as never,
        now
      )
    ).toBe('3 days left to respond');
    expect(daysRemainingWords({ responseDeadline: '2026-08-01 00:00:00' } as never, now)).toBe(
      'past its response time'
    );
    expect(
      daysRemainingWords({ responseDeadline: '', slaDeadline: null } as never, now)
    ).toBeNull();
  });

  it('titles a conversation by its title, else its first line', () => {
    expect(discussionTitle({ title: 'On trust', body: 'x' })).toBe('On trust');
    expect(discussionTitle({ title: null, body: '\nIs the payoff right?\nmore' })).toBe(
      'Is the payoff right?'
    );
  });
});

@Component({
  selector: 'app-epr-focal',
  standalone: true,
  template: '<div class="focal-stub"></div>',
})
class EprFocalStub {
  @Input() anonymousPublicRead = false;
  @Input() slug = '';
  @Output() nodeLoaded = new EventEmitter<unknown>();
  @Output() notFound = new EventEmitter<string>();
  @Output() failed = new EventEmitter<string>();
}

/** Storage's content-chrome facing (GET /api/v1/epr/{id}/chrome), as the wire carries it. */
function chromeFacing(app: Record<string, unknown> | null = null): Record<string, unknown> {
  return {
    recordId: 'evolution-of-trust',
    title: 'The Evolution of Trust',
    format: 'html5-app',
    mode: 'trustful',
    reach: 'commons',
    witness: 'amber',
    computedAt: '2026-10-15T00:00:00Z',
    claims: { attestations: [] },
    steward: { stewards: [] },
    ...(app ? { app } : {}),
  };
}

const rawSimulation = {
  id: 'evolution-of-trust',
  title: 'The Evolution of Trust',
  description: 'An interactive guide to the game theory of trust.',
  contentType: 'collective',
  contentFormat: 'html5-app',
  reach: 'commons',
  trust: 'notarized',
  dhtAnchorHash: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
  dhtAnchorState: 'unverified',
  blobHash: 'sha256-abc',
  createdAt: '2026-05-27 20:46:37',
  updatedAt: '2026-08-05 18:40:53',
  tags: ['game-theory', 'trust'],
  metadata: {
    author: 'Nicky Case',
    license: 'CC0 Public Domain',
    estimatedTime: '30 minutes',
    version: '2',
  },
};

const resilienceView = {
  contentId: 'evolution-of-trust',
  encoding: { strategy: 'reed-solomon', dataShards: 2, parityShards: 1 },
  distribution: {
    totalShards: 3,
    shardsWithLocations: 2,
    distinctPeers: 3,
    shards: [
      { hash: 's1', shardType: 'data', peerIds: ['p1', 'p2'], status: 'ok' },
      { hash: 's2', shardType: 'data', peerIds: ['p3'], status: 'ok' },
      { hash: 's3', shardType: 'parity', peerIds: [], status: 'missing' },
    ],
  },
  stewardship: { stewardCount: 1, allocations: [] },
  commitments: { activePeers: 1, totalCommittedBytes: 0, totalUsedBytes: 0 },
  health: { score: 0.67, canSurviveFailures: 1, status: 'degraded' },
};

function q(fixture: ComponentFixture<EprHomeComponent>, id: string): HTMLElement | null {
  return fixture.nativeElement.querySelector(`[data-testid="${id}"]`);
}

describe('EprHomeComponent', () => {
  let fixture: ComponentFixture<EprHomeComponent>;
  let storage: {
    getContent: ReturnType<typeof vi.fn>;
    getContentEngagement: ReturnType<typeof vi.fn>;
    getStorageBaseUrl: ReturnType<typeof vi.fn>;
    getBlobUrl: ReturnType<typeof vi.fn>;
  };
  let storageApi: {
    getContentStewardship: ReturnType<typeof vi.fn>;
    getPresenceForContent: ReturnType<typeof vi.fn>;
    getRelatedContent: ReturnType<typeof vi.fn>;
  };
  let governance: Record<string, ReturnType<typeof vi.fn>>;
  let resilienceDetail: {
    getResilience: ReturnType<typeof vi.fn>;
    verifyResilience: ReturnType<typeof vi.fn>;
  };
  let observations: {
    begin: ReturnType<typeof vi.fn>;
    end: ReturnType<typeof vi.fn>;
    noteScroll: ReturnType<typeof vi.fn>;
  };
  let navStack: {
    previous: ReturnType<typeof vi.fn>;
    record: ReturnType<typeof vi.fn>;
    entries: ReturnType<typeof vi.fn>;
  };
  let auth: {
    isAuthenticated: ReturnType<typeof signal<boolean>>;
    humanId: ReturnType<typeof signal<string | null>>;
  };
  let affinity: {
    getAffinity: ReturnType<typeof vi.fn>;
    setAffinity: ReturnType<typeof vi.fn>;
    trackView: ReturnType<typeof vi.fn>;
    affinity$: ReturnType<typeof of>;
  };
  let seo: { updateForContent: ReturnType<typeof vi.fn>; setTitle: ReturnType<typeof vi.fn> };

  async function configure(
    paramMap: Observable<ParamMap>,
    opts: {
      fragment?: Observable<string | null>;
      snapshot?: Observable<unknown>;
      query?: Record<string, string>;
      chrome?: Observable<unknown>;
    } = {}
  ): Promise<void> {
    await TestBed.configureTestingModule({
      imports: [EprHomeComponent],
      providers: [
        provideRouter([]),
        {
          provide: HttpClient,
          useValue: {
            get: vi.fn((url: string) =>
              url.endsWith('/chrome') ? (opts.chrome ?? chromeResponse) : of({ related: [] })
            ),
          },
        },
        { provide: StorageClientService, useValue: storage },
        {
          provide: ActivatedRoute,
          useValue: {
            paramMap,
            fragment: opts.fragment ?? of(null),
            queryParamMap: of(convertToParamMap(opts.query ?? {})),
          },
        },
        {
          provide: ResilienceService,
          useValue: { getSnapshot: vi.fn().mockReturnValue(opts.snapshot ?? of(null)) },
        },
        {
          provide: DistributionService,
          useValue: {
            getDetails: vi.fn().mockResolvedValue({
              summary: { replicaCount: 5, replicaTarget: 7, replicaHealth: 'at_risk' },
            }),
          },
        },
        { provide: StorageApiService, useValue: storageApi },
        {
          provide: EprResolverService,
          useValue: { resolveEprHead: vi.fn().mockReturnValue(of(null)) },
        },
        { provide: GovernanceApiService, useValue: governance },
        {
          provide: AttestationApiService,
          useValue: { listBySubject: vi.fn().mockResolvedValue([]) },
        },
        { provide: ResilienceDetailService, useValue: resilienceDetail },
        { provide: ObservationEmitterService, useValue: observations },
        { provide: SessionNavStackService, useValue: navStack },
        { provide: AuthService, useValue: auth },
        { provide: AffinityTrackingService, useValue: affinity },
        { provide: SeoService, useValue: seo },
      ],
    })
      .overrideComponent(EprHomeComponent, {
        remove: { imports: [EprFocalComponent] },
        add: { imports: [EprFocalStub] },
      })
      .compileComponents();
    fixture = TestBed.createComponent(EprHomeComponent);
    await settle();
  }

  async function settle(): Promise<void> {
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  async function mount(
    resourceId: string,
    opts: {
      fragment?: Observable<string | null>;
      snapshot?: Observable<unknown>;
      query?: Record<string, string>;
      chrome?: Observable<unknown>;
    } = {}
  ): Promise<void> {
    await configure(of(convertToParamMap({ resourceId })), opts);
  }

  /** The facing every mount reads unless a test names its own: an app declaring nothing extra. */
  let chromeResponse: Observable<unknown>;

  beforeEach(() => {
    chromeResponse = of(chromeFacing({ entry: 'index.html', capabilities: [], presents: false }));
    storage = {
      getStorageBaseUrl: vi.fn().mockReturnValue('https://doorway.test'),
      getContent: vi.fn().mockReturnValue(of(rawSimulation)),
      getContentEngagement: vi
        .fn()
        .mockReturnValue(
          of({ contentId: 'x', views: 9, uniqueViewers: 6, completions: 2, completionRate: 0.34 })
        ),
      getBlobUrl: vi.fn((hash: string) => `https://doorway.test/blob/${hash}`),
    };
    storageApi = {
      getContentStewardship: vi.fn().mockReturnValue(of({ allocations: [] })),
      getPresenceForContent: vi.fn().mockReturnValue(of([])),
      getRelatedContent: vi.fn().mockReturnValue(of([])),
    };
    governance = {
      getChallengesForEntity: vi.fn().mockResolvedValue([]),
      getGovernanceState: vi.fn().mockResolvedValue(null),
      getSignalAggregate: vi.fn().mockResolvedValue({
        entityType: 'content',
        entityId: 'evolution-of-trust',
        totalSignals: 3,
        byType: { reaction: 3 },
        byValue: { moved: 2, hopeful: 1 },
        uniqueParticipants: 2,
        consensusStrength: 0.8,
      }),
      queryDiscussions: vi.fn().mockResolvedValue([]),
      recordSignal: vi.fn().mockResolvedValue({}),
      fileChallenge: vi.fn().mockResolvedValue({}),
    };
    resilienceDetail = {
      getResilience: vi.fn().mockReturnValue(of(resilienceView)),
      verifyResilience: vi.fn().mockReturnValue(
        of({
          contentId: 'evolution-of-trust',
          verified: true,
          encoding: 'reed-solomon',
          shardsAvailable: 2,
          shardsNeeded: 2,
          shardsLocated: 2,
          shardsMissing: 0,
          reconstructionTimeMs: 12,
          originalHash: '',
          reconstructedHash: '',
          hashMatch: true,
          error: null,
        })
      ),
    };
    observations = { begin: vi.fn(), end: vi.fn(), noteScroll: vi.fn() };
    navStack = {
      previous: vi.fn().mockReturnValue(null),
      record: vi.fn(),
      entries: vi.fn().mockReturnValue([]),
    };
    auth = { isAuthenticated: signal(false), humanId: signal<string | null>(null) };
    affinity = {
      getAffinity: vi.fn().mockReturnValue(0.2),
      setAffinity: vi.fn(),
      trackView: vi.fn(),
      affinity$: of({}),
    };
    seo = { updateForContent: vi.fn(), setTitle: vi.fn() };
  });

  it('renders the frame with identity and chips for a reachable atom', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home')).not.toBeNull();
    expect(q(fixture, 'epr-home-title')?.textContent).toContain('The Evolution of Trust');
    expect(q(fixture, 'epr-home-chip-reach')?.textContent).toContain('Commons');
    expect(q(fixture, 'epr-home-chip-notarized')?.textContent).toContain(
      'anchor not yet verified here'
    );
    expect(fixture.nativeElement.textContent).not.toContain('Back to Lamad');
  });

  it('builds the download link even when resolving the blob URL writes a signal (NG0600)', async () => {
    // The connection strategy logs while resolving the base URL, and the logger
    // keeps recent lines in a signal: that write must not land inside a computed.
    const recentLogs = signal<string[]>([]);
    storage.getBlobUrl.mockImplementation((hash: string) => {
      recentLogs.update(lines => [...lines, `resolve ${hash}`]);
      return `https://doorway.test/blob/${hash}`;
    });
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-download')?.getAttribute('href')).toBe(
      'https://doorway.test/blob/sha256-abc'
    );
    expect(recentLogs()).toEqual(['resolve sha256-abc']);
  });

  it('shows the tags under the lede and the version in the eyebrow', async () => {
    await mount('evolution-of-trust');
    const tags = fixture.nativeElement.querySelectorAll('[data-testid="epr-home-tag"]');
    expect(Array.from(tags as NodeListOf<Element>).map(t => t.textContent?.trim())).toEqual([
      'game-theory',
      'trust',
    ]);
    expect(fixture.nativeElement.querySelector('.epr-home__eyebrow')?.textContent).toContain('v2');
  });

  it('hands the slug to the focal slot in the immersive shape', async () => {
    await mount('evolution-of-trust');
    const focal = q(fixture, 'epr-home-focal');
    expect(focal?.classList.contains('epr-home__focal--immersive')).toBe(true);
    expect(focal?.querySelector('.focal-stub')).not.toBeNull();
  });

  it('uses the reading shape for markdown', async () => {
    storage.getContent.mockReturnValue(of({ ...rawSimulation, contentFormat: 'markdown' }));
    await mount('succession');
    expect(q(fixture, 'epr-home-focal')?.classList.contains('epr-home__focal--reading')).toBe(true);
  });

  it('renders the out-of-reach gate for a null atom, with no chrome', async () => {
    storage.getContent.mockReturnValue(of(null));
    await mount('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain("We can't reach this one from here");
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-your-mark')).toBeNull();
    expect(q(fixture, 'epr-home-focal')).toBeNull();
    expect(q(fixture, 'epr-home-edit')).toBeNull();
    expect(observations.begin).not.toHaveBeenCalled();
  });

  it('renders the error state when the load fails', async () => {
    storage.getContent.mockReturnValue(throwError(() => new Error('boom')));
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-error')).not.toBeNull();
  });

  it('carries the universal address line, and Network detail is an in-page #holds anchor', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-address')?.textContent).toContain('/epr/evolution-of-trust');
    const detail = q(fixture, 'epr-home-network-detail')!;
    expect(detail.getAttribute('href')).toBe('/epr/evolution-of-trust#holds');
    expect(fixture.nativeElement.innerHTML).not.toContain('/resource/');
  });

  it('opens the holds fold from the footer and only then reads the network detail', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-fold-holds')).toBeNull();
    expect(resilienceDetail.getResilience).not.toHaveBeenCalled();

    q(fixture, 'epr-home-network-detail')!.click();
    await settle();

    expect(q(fixture, 'epr-home-fold-holds')).not.toBeNull();
    expect(resilienceDetail.getResilience).toHaveBeenCalledWith('evolution-of-trust');
    expect(q(fixture, 'epr-home-health')?.textContent).toContain(
      'Thinning · can lose 1 peer and still be whole'
    );
    expect(q(fixture, 'epr-home-fold-holds')?.textContent).toContain('5 of 7 wanted · at risk');
  });

  it('arriving with the legacy #network fragment opens the holds fold', async () => {
    await mount('evolution-of-trust', { fragment: of('network') });
    expect(q(fixture, 'epr-home-fold-holds')).not.toBeNull();
    expect(q(fixture, 'epr-home-fold-toggle-holds')?.getAttribute('aria-expanded')).toBe('true');
    expect(resilienceDetail.getResilience).toHaveBeenCalledTimes(1);
    expect(q(fixture, 'epr-home-pieces')?.querySelectorAll('li').length).toBe(3);
  });

  it('arriving with #governance opens the governed fold', async () => {
    await mount('evolution-of-trust', { fragment: of('governance') });
    expect(q(fixture, 'epr-home-fold-governed')).not.toBeNull();
    expect(q(fixture, 'epr-home-fold-holds')).toBeNull();
  });

  it("checks it's still whole and says the result in one line", async () => {
    await mount('evolution-of-trust', { fragment: of('holds') });
    q(fixture, 'epr-home-verify')!.click();
    await settle();
    expect(resilienceDetail.verifyResilience).toHaveBeenCalledWith('evolution-of-trust');
    expect(q(fixture, 'epr-home-verify-result')?.textContent).toContain(
      'Whole · 2 of 2 pieces found · 12 ms'
    );
  });

  it("says the check couldn't run when the peer refuses it", async () => {
    resilienceDetail.verifyResilience.mockReturnValue(throwError(() => new Error('503')));
    await mount('evolution-of-trust', { fragment: of('holds') });
    q(fixture, 'epr-home-verify')!.click();
    await settle();
    expect(q(fixture, 'epr-home-verify-result')?.textContent).toContain(
      "Couldn't run the check from here."
    );
  });

  it("names who has been counted here in the Who's here leg", async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-leg-here')).not.toBeNull();
    expect(q(fixture, 'epr-home-attention')?.textContent).toContain(
      'Viewed 9 times by 6 people · 2 completions'
    );
  });

  it('shows the held-by chip from the snapshot and renders every leg', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-chip-held')?.textContent).toContain(
      'Not yet held by any household'
    );
    for (const leg of ['holds', 'here', 'lives', 'governed', 'from']) {
      expect(q(fixture, `epr-home-leg-${leg}`)).not.toBeNull();
    }
  });

  it('shows the checking line before the resilience snapshot resolves, then the held chip after', async () => {
    const snapshot$ = new Subject<unknown>();
    await mount('evolution-of-trust', { snapshot: snapshot$ });

    expect(q(fixture, 'epr-home-chip-held')?.textContent).toContain('Checking who holds this…');

    snapshot$.next({
      contentId: 'evolution-of-trust',
      feltStatus: {
        headline: 'Held by only 1 household — invite another to help hold these',
        reassurance: 'needs-help',
        heldBy: [{ id: 'household-dowell', kind: 'household', label: 'Dowell Household' }],
        floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 1 },
        suggestedAction: 'Invite a household to help hold these',
      },
    });
    await settle();

    expect(q(fixture, 'epr-home-chip-held')?.textContent).toContain('Held by 1 of 3 households');
  });

  it('resets to loading between two atoms on the same component instance', async () => {
    const paramMap$ = new BehaviorSubject(convertToParamMap({ resourceId: 'evolution-of-trust' }));
    const second$ = new Subject<unknown>();
    storage.getContent = vi
      .fn()
      .mockReturnValueOnce(of(rawSimulation))
      .mockReturnValueOnce(second$);

    await configure(paramMap$);

    expect(q(fixture, 'epr-home-title')?.textContent).toContain('The Evolution of Trust');

    paramMap$.next(convertToParamMap({ resourceId: 'succession' }));
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-loading')).not.toBeNull();

    second$.next({ ...rawSimulation, id: 'succession', title: 'Succession' });
    second$.complete();
    await settle();
    expect(q(fixture, 'epr-home-title')?.textContent).toContain('Succession');
  });

  it('opens one witnessed view per atom and closes the previous one on leaving it', async () => {
    const paramMap$ = new BehaviorSubject(convertToParamMap({ resourceId: 'evolution-of-trust' }));
    storage.getContent = vi
      .fn()
      .mockReturnValueOnce(of(rawSimulation))
      .mockReturnValueOnce(of({ ...rawSimulation, id: 'succession', title: 'Succession' }));
    await configure(paramMap$);
    expect(observations.begin).toHaveBeenCalledWith('evolution-of-trust');

    paramMap$.next(convertToParamMap({ resourceId: 'succession' }));
    await settle();
    expect(observations.end).toHaveBeenCalledWith('evolution-of-trust');
    expect(observations.begin).toHaveBeenCalledWith('succession');

    fixture.destroy();
    expect(observations.end).toHaveBeenCalledWith('succession');
  });

  it.each(['challenge', 'feedback'] as const)(
    'ignores a late %s after navigation and clears focus and notes',
    async type => {
      auth.isAuthenticated.set(true);
      auth.humanId.set('human-jessica');
      let resolve!: (value: unknown) => void;
      const pending = new Promise(done => {
        resolve = done;
      });
      governance[type === 'challenge' ? 'fileChallenge' : 'recordSignal'].mockReturnValue(pending);
      const params = new BehaviorSubject(convertToParamMap({ resourceId: 'evolution-of-trust' }));
      storage.getContent.mockImplementation((id: string) => of({ ...rawSimulation, id }));
      await configure(params);
      fixture.componentInstance.onConcern({ type, text: 'Please check' });
      fixture.componentInstance.concernNote.set('Earlier note');
      fixture.componentInstance.toggleFocus();
      await settle();
      const exit = q(fixture, 'epr-home-focus-exit')!;
      expect(exit.textContent).toContain('Leave focus');
      expect(fixture.nativeElement.querySelector('button')).toBe(exit);
      params.next(convertToParamMap({ resourceId: 'next-atom' }));
      await settle();
      expect(fixture.componentInstance.concernNote()).toBeNull();
      expect(fixture.componentInstance.focused()).toBe(false);
      expect(document.body.classList.contains('focused-view-mode')).toBe(false);
      const refresh =
        governance[type === 'challenge' ? 'getChallengesForEntity' : 'getSignalAggregate'];
      const calls = refresh.mock.calls.length;
      resolve({});
      await settle();
      expect(fixture.componentInstance.concernNote()).toBeNull();
      expect(refresh).toHaveBeenCalledTimes(calls);
    }
  );

  it('leaves focus through the button outside the focal frame', async () => {
    await mount('evolution-of-trust');
    fixture.componentInstance.toggleFocus();
    await settle();
    q(fixture, 'epr-home-focus-exit')!.click();
    await settle();
    expect(fixture.componentInstance.focused()).toBe(false);
  });

  it('reports scroll depth from the window while the atom is open', async () => {
    await mount('evolution-of-trust');
    observations.noteScroll.mockClear();
    window.dispatchEvent(new Event('scroll'));
    expect(observations.noteScroll).toHaveBeenCalled();
  });

  it('Focus shows the focal alone and Escape leaves it', async () => {
    await mount('evolution-of-trust');
    const host = fixture.nativeElement as HTMLElement;
    q(fixture, 'epr-home-focus')!.click();
    await settle();
    expect(host.classList.contains('epr-home--focused')).toBe(true);
    expect(q(fixture, 'epr-home-focus')?.getAttribute('aria-pressed')).toBe('true');
    expect(document.body.classList.contains('focused-view-mode')).toBe(true);

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await settle();
    expect(host.classList.contains('epr-home--focused')).toBe(false);
    expect(document.body.classList.contains('focused-view-mode')).toBe(false);
  });

  it('names the previous stop in the arrival chip when the nav stack has one', async () => {
    navStack.previous.mockReturnValue({
      url: '/epr/succession',
      cid: '',
      label: 'Succession Without Conquest | Elohim Protocol',
      ts: 1,
    });
    await mount('evolution-of-trust');
    const chip = q(fixture, 'epr-home-arrival')!;
    expect(chip.textContent).toContain('Succession Without Conquest');
    expect(chip.textContent).not.toContain('| Elohim Protocol');
    expect(chip.getAttribute('href')).toBe('/epr/succession');
  });

  it('renders no arrival chip on a cold link', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-arrival')).toBeNull();
  });

  it('the gate names the referring resource when there is one', async () => {
    // The gate never records itself (spec §12.2 — unreachable isn't a place
    // to come back to), so it reads the top of the stack directly, not
    // previous() — the entry the atom you came FROM recorded for itself.
    storage.getContent.mockReturnValue(of(null));
    navStack.entries.mockReturnValue([
      {
        url: '/epr/evolution-of-trust',
        cid: '',
        label: 'The Evolution of Trust | Elohim Protocol',
        ts: 1,
      },
    ]);
    await mount('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain('The Evolution of Trust');
    expect(q(fixture, 'epr-home-gate-back')?.getAttribute('href')).toBe('/epr/evolution-of-trust');
  });

  it('the gate never names itself when the top of the stack is its own address', async () => {
    // An atom viewed, recorded, then found unreachable on a later load: the
    // top of the nav stack is now the gate's own address. The gate must not
    // read that back as "what we do know" / "Back to <itself>".
    storage.getContent.mockReturnValue(of(null));
    navStack.entries.mockReturnValue([
      {
        url: '/epr/concept-bidirectional-trust',
        cid: '',
        label: 'Bidirectional trust | Elohim Protocol',
        ts: 1,
      },
    ]);
    await mount('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-arrival')).toBeNull();
    expect(q(fixture, 'epr-home-gate-back')).toBeNull();
  });

  it('shows Your mark only when signed in, as one row without a percentage badge', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-your-mark')).toBeNull();
    auth.isAuthenticated.set(true);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-your-mark')?.textContent).toContain('Practicing · 20%');
  });

  it('says how it landed in words and records a reaction as the signed-in human', async () => {
    auth.isAuthenticated.set(true);
    auth.humanId.set('human-jessica');
    await mount('evolution-of-trust');
    const landed = q(fixture, 'epr-home-reactions')!;
    expect(landed.textContent).toContain('3 reactions from 2 people · consensus strong');
    expect(landed.textContent).not.toMatch(/\d+%/);
    const bar = landed.querySelector('elohim-reaction-bar') as HTMLElement & { counts: unknown };
    expect(bar.counts).toEqual([
      { type: 'moved', count: 2 },
      { type: 'hopeful', count: 1 },
    ]);
    bar.dispatchEvent(
      new CustomEvent('reaction-submit', {
        detail: { type: 'moved', entityType: 'content', entityId: 'evolution-of-trust' },
      })
    );
    expect(governance['recordSignal']).toHaveBeenCalledWith(
      expect.objectContaining({
        humanId: 'human-jessica',
        signalType: 'reaction',
        signalValue: 'moved',
      })
    );
  });

  it('offers no reaction bar when the session carries no human id', async () => {
    auth.isAuthenticated.set(true);
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-reactions')?.querySelector('elohim-reaction-bar')).toBeNull();
    expect(q(fixture, 'epr-home-raise-concern')).toBeNull();
    expect(q(fixture, 'epr-home-raise-concern-signed-out')?.textContent).toContain(
      'Sign in to raise a concern.'
    );
  });

  it('files a challenge raised through the gate, then reads the challenges again', async () => {
    auth.isAuthenticated.set(true);
    auth.humanId.set('human-jessica');
    await mount('evolution-of-trust');
    const trigger = q(fixture, 'epr-home-raise-concern')!;
    expect(trigger.tagName.toLowerCase()).toBe('elohim-gate-feedback-trigger');
    expect(governance['getChallengesForEntity']).toHaveBeenCalledTimes(1);

    trigger.dispatchEvent(
      new CustomEvent('feedback-submit', {
        detail: { type: 'challenge', contentId: 'evolution-of-trust', text: 'The date is wrong' },
        bubbles: true,
      })
    );
    await settle();

    expect(governance['fileChallenge']).toHaveBeenCalledWith({
      entityType: 'content',
      entityId: 'evolution-of-trust',
      challengerId: 'human-jessica',
      standingBasis: 'commons-member',
      groundsPrimary: 'other',
      groundsSecondary: null,
      evidence: 'The date is wrong',
      requestedOutcome: null,
    });
    expect(governance['getChallengesForEntity']).toHaveBeenCalledTimes(2);
    expect(q(fixture, 'epr-home-leg-governed')?.textContent).toContain('Your challenge was filed.');
  });

  it('records gate feedback as a context-menu signal', async () => {
    auth.isAuthenticated.set(true);
    auth.humanId.set('human-jessica');
    await mount('evolution-of-trust');
    q(fixture, 'epr-home-raise-concern')!.dispatchEvent(
      new CustomEvent('feedback-submit', {
        detail: { type: 'feedback', contentId: 'evolution-of-trust', text: 'Lovely.' },
        bubbles: true,
      })
    );
    await settle();
    expect(governance['recordSignal']).toHaveBeenCalledWith(
      expect.objectContaining({ signalType: 'feedback', signalValue: 'Lovely.', mechanismLevel: 0 })
    );
  });

  it('offers "Edit in Lamad" only when signed in, minted, never for a generated step id', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-edit')).toBeNull();
    auth.isAuthenticated.set(true);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-edit')?.getAttribute('href')).toBe(
      '/lamad/resource/evolution-of-trust/edit'
    );
  });

  it('never offers Edit for a generated path-step id', async () => {
    auth.isAuthenticated.set(true);
    storage.getContent.mockReturnValue(of({ ...rawSimulation, id: 'path-foundations-step-3' }));
    await mount('path-foundations-step-3');
    expect(q(fixture, 'epr-home-edit')).toBeNull();
  });

  it('lists conversations newest first, and one line when there are none', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-conversation')?.textContent).toContain(
      'No one has spoken about this here yet.'
    );
    expect(q(fixture, 'epr-home-start-conversation')).toBeNull();
  });

  it('renders conversation rows from discussions', async () => {
    governance['queryDiscussions'].mockResolvedValue([
      {
        id: 'd1',
        contentId: 'evolution-of-trust',
        authorPresenceId: 'p',
        body: 'Older thread',
        parentId: null,
        createdAt: '2026-08-01 00:00:00',
        updatedAt: '2026-08-01 00:00:00',
        title: null,
        category: 'question',
        status: 'open',
        messageCount: 2,
        lastActivityAt: null,
      },
      {
        id: 'd2',
        contentId: 'evolution-of-trust',
        authorPresenceId: 'p',
        body: 'b',
        parentId: null,
        createdAt: '2026-08-03 00:00:00',
        updatedAt: '2026-08-03 00:00:00',
        title: 'On the payoff matrix',
        category: null,
        status: null,
        messageCount: 1,
        lastActivityAt: null,
      },
      { id: 'r1', parentId: 'd1', body: 'a reply', createdAt: '2026-08-04 00:00:00', title: null },
    ]);
    await mount('evolution-of-trust');
    const rows = Array.from(
      fixture.nativeElement.querySelectorAll(
        '[data-testid="epr-home-discussion"]'
      ) as NodeListOf<Element>
    ).map(r => r.textContent?.replace(/\s+/g, ' ').trim());
    expect(rows).toEqual([
      'On the payoff matrix 1 message',
      'Older thread 2 messages · open · question',
    ]);
  });

  it('reads stewardship from the content-stewardship view, with share, recognition and dispute', async () => {
    storageApi.getContentStewardship.mockReturnValue(
      of({
        contentId: 'evolution-of-trust',
        allocations: [
          {
            steward: { displayName: 'Matthew Dowell' },
            stewardPresenceId: 'presence-matthew',
            allocationRatio: 0.6,
            contributionType: 'original_creator',
            effectiveFrom: '2026-06-04 02:19:10',
            recognitionAccumulated: 12,
            disputeId: 'dispute-1',
            disputedAt: '2026-08-01 00:00:00',
            disputeReason: 'Co-authored',
            governanceState: 'disputed',
          },
        ],
        totalAllocation: 0.6,
        hasDisputes: true,
        primarySteward: null,
      })
    );
    await mount('evolution-of-trust');
    const steward = q(fixture, 'epr-home-steward')!;
    expect(steward.textContent).toContain('Matthew Dowell');
    expect(steward.textContent).toContain('60% share');
    expect(steward.textContent).toContain('12 recognition');
    expect(steward.textContent).toContain('Disputed: Co-authored');
  });

  it('links the download to the blob', async () => {
    await mount('evolution-of-trust');
    const dl = q(fixture, 'epr-home-download')!;
    expect(dl.getAttribute('href')).toBe('https://doorway.test/blob/sha256-abc');
    expect(dl.hasAttribute('download')).toBe(true);
    expect(dl.textContent).toContain('Download (html5-app)');
  });

  it('offers "Open in Lamad" for a claimed contentType', async () => {
    storage.getContent.mockReturnValue(
      of({ ...rawSimulation, id: 'foundations-christian-technology', contentType: 'path' })
    );
    await mount('foundations-christian-technology');
    const openInBundle = q(fixture, 'epr-home-open-in-bundle');
    expect(openInBundle?.textContent).toContain('Open in Lamad');
    expect(openInBundle?.getAttribute('href')).toBe('/lamad/path/foundations-christian-technology');
  });

  it('offers no "Open in <bundle>" lens for an unclaimed contentType', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-open-in-bundle')).toBeNull();
  });

  it('records the loaded atom on the session nav stack so a walk inside the shell has a prior stop', async () => {
    await mount('evolution-of-trust');
    expect(navStack.record).toHaveBeenCalledWith({
      url: '/epr/evolution-of-trust',
      cid: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
      label: 'The Evolution of Trust',
    });
  });

  it('does not record the gate on the nav stack — an unreachable id is not a place to come back to', async () => {
    storage.getContent.mockReturnValue(of(null));
    await mount('concept-bidirectional-trust');
    expect(navStack.record).not.toHaveBeenCalled();
  });

  it('sets the tab title from the loaded atom', async () => {
    await mount('evolution-of-trust');
    expect(seo.updateForContent).toHaveBeenCalledWith(
      expect.objectContaining({ id: 'evolution-of-trust', title: 'The Evolution of Trust' })
    );
  });

  it('sets the tab title to "Out of reach" on the gate', async () => {
    storage.getContent.mockReturnValue(of(null));
    await mount('concept-bidirectional-trust');
    expect(seo.setTitle).toHaveBeenCalledWith('Out of reach');
  });

  it('renders a claimed card instead of the focal renderer for a claimed contentType', async () => {
    storage.getContent.mockReturnValue(
      of({ ...rawSimulation, id: 'foundations-christian-technology', contentType: 'path' })
    );
    await mount('foundations-christian-technology');
    expect(q(fixture, 'epr-home-focal-claimed')?.textContent).toContain(
      'A path the Lamad app teaches'
    );
    expect(q(fixture, 'epr-home-focal-claimed')?.querySelector('.focal-stub')).toBeNull();
    expect(fixture.nativeElement.querySelector('.focal-stub')).toBeNull();
    expect(q(fixture, 'epr-home-focus')).toBeNull();
  });

  it('renders the focal stub and no claimed card for an unclaimed contentType', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-focal-claimed')).toBeNull();
    expect(fixture.nativeElement.querySelector('.focal-stub')).not.toBeNull();
  });

  describe('how this reached you (?via=&peer= from the content chrome)', () => {
    const hops = () =>
      Array.from(
        fixture.nativeElement.querySelectorAll(
          '[data-testid="epr-home-delivery-hop"]'
        ) as NodeListOf<HTMLElement>
      );

    it('names both the doorway and the peer, as the link describes them', async () => {
      await mount('evolution-of-trust', { query: { via: 'doorway-alpha', peer: 'peer-jessica' } });
      const block = q(fixture, 'epr-home-delivery')!;
      expect(block).not.toBeNull();
      expect(block.id).toBe('delivery');
      expect(block.textContent).toContain('as your link describes it');
      expect(hops().map(h => h.textContent?.trim().replace(/\s+/g, ' '))).toEqual([
        'through doorway doorway-alpha',
        'from peer peer-jessica',
      ]);
      expect(block.textContent).toMatch(
        /through doorway\s*doorway-alpha\s*·\s*from peer\s*peer-jessica/
      );
    });

    it('shows no block when the link names neither', async () => {
      await mount('evolution-of-trust');
      expect(q(fixture, 'epr-home-delivery')).toBeNull();
      expect(fixture.nativeElement.querySelector('#delivery')).toBeNull();
    });

    it('never guesses a missing part', async () => {
      await mount('evolution-of-trust', { query: { peer: 'peer-jessica' } });
      expect(hops().length).toBe(1);
      expect(hops()[0].textContent).toContain('from peer');
      expect(hops()[0].textContent).toContain('peer-jessica');
      expect(q(fixture, 'epr-home-delivery')?.textContent).not.toContain('doorway');
      expect(q(fixture, 'epr-home-delivery')?.textContent).not.toContain('·');
    });

    it('renders crafted values as escaped, capped text', async () => {
      const html = '<img src=x onerror="alert(1)"><b>bold</b>';
      const long = 'p'.repeat(400);
      await mount('evolution-of-trust', { query: { via: html, peer: long } });
      const [via, peer] = hops();
      expect(via.querySelector('img')).toBeNull();
      expect(via.querySelector('b')).toBeNull();
      expect(via.querySelector('code')?.textContent).toBe(html);
      expect(peer.querySelector('code')?.textContent).toBe(`${'p'.repeat(DELIVERY_VALUE_MAX)}…`);
      expect(q(fixture, 'epr-home-delivery')!.querySelector('img')).toBeNull();
    });

    it('is not shown on the out-of-reach gate (nothing was delivered)', async () => {
      storage.getContent.mockReturnValue(of(null));
      await mount('concept-bidirectional-trust', { query: { via: 'doorway-alpha' } });
      expect(q(fixture, 'epr-home-delivery')).toBeNull();
    });
  });

  describe("the content chrome's card sections", () => {
    let scrolled: string[];
    let navigate: ReturnType<typeof vi.spyOn>;
    const original = HTMLElement.prototype.scrollIntoView;

    beforeEach(() => {
      scrolled = [];
      HTMLElement.prototype.scrollIntoView = function (this: HTMLElement) {
        scrolled.push(this.id);
      };
      navigate = vi.spyOn(Router.prototype, 'navigate').mockResolvedValue(true);
      auth.isAuthenticated.set(true);
      auth.humanId.set('human-matthew');
    });

    afterEach(() => {
      HTMLElement.prototype.scrollIntoView = original;
      navigate.mockRestore();
    });

    for (const section of [
      'source',
      'delivery',
      'holders',
      'steward',
      'claims',
      'yours',
      'permissions',
    ]) {
      it(`#${section} lands on a section this page carries, and keeps the link as written`, async () => {
        await mount('evolution-of-trust', {
          fragment: of(section),
          query: { via: 'doorway-alpha', peer: 'peer-jessica' },
        });
        await settle();
        const anchor = requestFromFragment(section)!.anchor!;
        expect(fixture.nativeElement.querySelector(`#${anchor}`)).not.toBeNull();
        expect(scrolled).toContain(anchor);
        expect(navigate).not.toHaveBeenCalled();
      });
    }

    it('#claims opens the attestations fold', async () => {
      await mount('evolution-of-trust', { fragment: of('claims') });
      expect(q(fixture, 'epr-home-fold-from')).not.toBeNull();
    });

    it('#holders opens the network detail', async () => {
      await mount('evolution-of-trust', { fragment: of('holders') });
      expect(q(fixture, 'epr-home-fold-holds')).not.toBeNull();
      expect(resilienceDetail.getResilience).toHaveBeenCalledWith('evolution-of-trust');
    });

    it('#permissions scrolls nowhere on a record that is not an html5-app', async () => {
      storage.getContent.mockReturnValue(of({ ...rawSimulation, contentFormat: 'markdown' }));
      await mount('evolution-of-trust', {
        fragment: of('permissions'),
        chrome: of(chromeFacing()),
      });
      expect(q(fixture, 'epr-home-leg-permissions')).toBeNull();
      await settle();
      expect(scrolled).toEqual([]);
    });
  });

  describe("what it may do (the content-chrome facing's app lens)", () => {
    const caps = () =>
      Array.from(
        fixture.nativeElement.querySelectorAll(
          '[data-testid="epr-home-capability"]'
        ) as NodeListOf<HTMLElement>
      );
    const app = (extra: Record<string, unknown>) =>
      of(chromeFacing({ entry: 'index.html', capabilities: [], presents: false, ...extra }));

    it("reads the record's app declarations from storage's chrome facing", async () => {
      await mount('evolution-of-trust');
      const http = TestBed.inject(HttpClient) as unknown as { get: ReturnType<typeof vi.fn> };
      expect(http.get).toHaveBeenCalledWith(
        'https://doorway.test/api/v1/epr/evolution-of-trust/chrome'
      );
    });

    it('labels each declared capability and marks device ones as your own choice', async () => {
      await mount('evolution-of-trust', { chrome: app({ capabilities: ['forms', 'microphone'] }) });
      const section = q(fixture, 'epr-home-leg-permissions')!;
      expect(section.id).toBe('permissions');
      expect(section.textContent).toContain('What it may do');
      expect(caps().map(c => c.dataset['capability'])).toEqual(['forms', 'microphone']);
      expect(caps()[0].textContent).toContain('Submit forms');
      expect(caps()[0].textContent).not.toContain('asks you first');
      expect(caps()[1].textContent).toContain('Microphone');
      expect(caps()[1].textContent).toContain('asks you first');
      expect(q(fixture, 'epr-home-permissions-none')).toBeNull();
    });

    it('names the origins it says it reaches, as declarations', async () => {
      await mount('evolution-of-trust', {
        chrome: app({ sources: ['https://fonts.example', 'https://cdn.example'] }),
      });
      const sources = Array.from(
        fixture.nativeElement.querySelectorAll(
          '[data-testid="epr-home-permission-source"]'
        ) as NodeListOf<HTMLElement>
      ).map(el => el.textContent?.trim());
      expect(sources).toEqual(['https://fonts.example', 'https://cdn.example']);
      expect(q(fixture, 'epr-home-permissions-sources')?.textContent).toContain('not a grant');
      expect(q(fixture, 'epr-home-permissions-none')).toBeNull();
    });

    it('says "Declares nothing extra" when the lens declares nothing', async () => {
      await mount('evolution-of-trust');
      expect(q(fixture, 'epr-home-permissions-none')?.textContent).toContain(
        'Declares nothing extra'
      );
      expect(caps()).toEqual([]);
    });

    it('claims nothing when the facing cannot be read', async () => {
      await mount('evolution-of-trust', { chrome: throwError(() => new Error('503')) });
      expect(q(fixture, 'epr-home-permissions-unreadable')).not.toBeNull();
      expect(q(fixture, 'epr-home-permissions-none')).toBeNull();
    });

    it('is absent when the facing carries no app lens', async () => {
      storage.getContent.mockReturnValue(
        of({ ...rawSimulation, contentFormat: 'markdown', contentBody: '# hi' })
      );
      await mount('evolution-of-trust', { chrome: of(chromeFacing()) });
      expect(q(fixture, 'epr-home-leg-permissions')).toBeNull();
    });
  });
});
