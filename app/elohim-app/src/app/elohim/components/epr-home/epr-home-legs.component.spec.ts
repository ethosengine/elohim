import { Component, Input } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { beforeEach, describe, expect, it } from 'vitest';

import { EprHomeLegsComponent } from './epr-home-legs.component';
import type { EprRelationship } from '../../models/epr-head.model';
import { EprRelationshipsPanelComponent } from '../epr-relationships-panel/epr-relationships-panel.component';
import { toAtom } from './epr-home.model';

import type { ChallengeView } from '@elohim/storage-client/generated';

/**
 * Stubs the relationships panel — its own DI tree (ambient EPR resolution +
 * lamad's ResilienceService) is that component's concern, not this frame's.
 * Keeps this spec from pulling a lamad-typed service into the shell's
 * cross-workspace import ledger for a wrapper this test doesn't inspect.
 */
@Component({ selector: 'app-epr-relationships-panel', standalone: true, template: '' })
class EprRelationshipsPanelStub {
  @Input() relationships: EprRelationship[] = [];
}

const atom = toAtom({
  id: 'evolution-of-trust',
  title: 'The Evolution of Trust',
  contentType: 'collective',
  contentFormat: 'html5-app',
  reach: 'commons',
  trust: 'notarized',
  dhtAnchorHash: 'uhCkk_D-fLh9hgcSAk4ZE6375dJuKrzf4Y9CDEOoX4e9fKujiEm8f',
  dhtAnchorState: 'unverified',
  createdAt: '2026-05-27 20:46:37',
  updatedAt: '2026-08-05 18:40:53',
  metadata: {
    sourceUrl: 'https://github.com/ncase/trust',
    license: 'CC0 Public Domain',
    relatedNodeIds: ['concept-bidirectional-trust'],
  },
});

const snapshot = {
  contentId: 'evolution-of-trust',
  feltStatus: {
    headline: 'Held by only 1 household — invite another to help hold these',
    reassurance: 'needs-help',
    heldBy: [
      { id: 'household-dowell', kind: 'household', label: 'Dowell Household', intraHubPeers: 2 },
    ],
    floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 1 },
    suggestedAction: 'Invite a household to help hold these',
  },
};

const protectedSnapshot = {
  contentId: 'evolution-of-trust',
  feltStatus: {
    headline: 'Held by 3 households — this is well protected',
    reassurance: 'protected',
    heldBy: [
      { id: 'household-a', kind: 'household', label: 'Household A' },
      { id: 'household-b', kind: 'household', label: 'Household B' },
      { id: 'household-c', kind: 'household', label: 'Household C' },
    ],
    floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 3 },
    // suggestedAction intentionally absent — nothing to invite anyone to do.
  },
};

/** Fills every ChallengeView field with an inert default so tests only spell out what they assert. */
function makeChallenge(overrides: Partial<ChallengeView>): ChallengeView {
  return {
    id: 'chal-1',
    entityType: 'content',
    entityId: 'evolution-of-trust',
    challengerId: 'presence-someone',
    standingBasis: 'steward',
    groundsPrimary: 'factual_error',
    groundsSecondary: null,
    evidence: null,
    requestedOutcome: null,
    state: 'pending',
    responseOutcome: null,
    responseReasoning: null,
    responseActions: null,
    responseBy: null,
    setsPrecedent: false,
    filedAt: '2026-08-01 00:00:00',
    acknowledgedAt: null,
    responseDeadline: '2026-08-08 00:00:00',
    respondedAt: null,
    resolvedAt: null,
    createdAt: '2026-08-01 00:00:00',
    slaStatus: 'on_track',
    dhtAnchorHash: null,
    challengerName: null,
    challengerStanding: null,
    grounds: null,
    description: null,
    status: null,
    priority: null,
    slaDeadline: null,
    assignedElohim: null,
    resolution: null,
    updatedAt: null,
    metadata: null,
    ...overrides,
  };
}

function q(fixture: ComponentFixture<EprHomeLegsComponent>, id: string): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${id}"]`);
}

describe('EprHomeLegsComponent', () => {
  let fixture: ComponentFixture<EprHomeLegsComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EprHomeLegsComponent],
      providers: [provideRouter([])],
    })
      .overrideComponent(EprHomeLegsComponent, {
        remove: { imports: [EprRelationshipsPanelComponent] },
        add: { imports: [EprRelationshipsPanelStub] },
      })
      .compileComponents();
    fixture = TestBed.createComponent(EprHomeLegsComponent);
    fixture.componentRef.setInput('atom', atom);
  });

  it('Who holds it: the felt headline, the floor, the households, the action as plain text (not a button)', () => {
    fixture.componentRef.setInput('snapshot', snapshot);
    fixture.componentRef.setInput('peersHolding', 5);
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-holds')!;
    expect(leg.textContent).toContain('Held by only 1 household');
    expect(leg.textContent).toContain('1 of 3 households this should live in');
    expect(leg.textContent).toContain('Dowell Household');
    expect(leg.textContent).toContain('5 peers keep a copy');
    expect(leg.textContent).toContain('Invite a household to help hold these');
    expect(leg.textContent).not.toMatch(/\d+%/);
    const invite = q(fixture, 'epr-home-invite-household')!;
    expect(invite.tagName).toBe('P');
  });

  it('Who holds it: shows the checking line and no pips before a snapshot has been asked for', () => {
    // snapshot input left at its default (undefined) — not yet asked.
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-holds')!;
    expect(leg.textContent).toContain('Checking who holds this…');
    expect(leg.querySelectorAll('.pip').length).toBe(0);
    expect(q(fixture, 'epr-home-invite-household')).toBeNull();
  });

  it('Who holds it: collapses to one line when asked and nothing is found', () => {
    fixture.componentRef.setInput('snapshot', null);
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-holds')!;
    expect(leg.textContent).toContain("We can't confirm this is backed up anywhere yet.");
    expect(leg.querySelectorAll('.pip').length).toBe(3);
  });

  it('Where this lives: names related ids and tags them as not checked from here', () => {
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-lives')!;
    expect(leg.textContent).toContain('Bidirectional trust');
    expect(leg.textContent).toContain('not checked from here');
    expect(leg.querySelector('a[href="/epr/concept-bidirectional-trust"]')).not.toBeNull();
  });

  it('Where this lives: keeps the relationships panel wrapper test id', () => {
    fixture.componentRef.setInput('relationships', [
      { type: 'REFERENCES', target: 'governance-epic' },
    ]);
    fixture.detectChanges();
    expect(q(fixture, 'viewer-relationships-panel')).not.toBeNull();
  });

  it("How it's governed: one line when nothing is in question", () => {
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-leg-governed')?.textContent).toContain(
      'No challenges, no labels. Nothing is in question.'
    );
  });

  it('Where it came from: steward, source, anchor in words, dates, raw link', () => {
    fixture.componentRef.setInput('stewards', [
      {
        stewardPresenceId: 'matthew-dowell',
        contributionType: 'original_creator',
        effectiveFrom: '2026-06-04 02:19:10',
        allocationRatio: 1,
        recognitionAccumulated: 4,
      },
    ]);
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-from')!;
    expect(leg.textContent).toContain('matthew-dowell');
    expect(leg.textContent).toContain('original creator');
    expect(leg.textContent).toContain('github.com/ncase/trust');
    expect(leg.textContent).toContain('uhCkk_D-fLh9…KujiEm8f');
    expect(leg.textContent).toContain('not yet verified on this doorway');
    expect(leg.textContent).toContain('May 27, 2026');
    expect(leg.querySelector('a[href="/epr/evolution-of-trust/raw"]')).not.toBeNull();
  });

  it("How it's governed: a responded challenge is not open — backend never writes state:'resolved'", () => {
    fixture.componentRef.setInput('challenges', [
      makeChallenge({
        id: 'chal-responded',
        state: 'responded',
        respondedAt: '2026-08-05 00:00:00',
      }),
    ]);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-leg-governed')?.textContent).toContain(
      'No challenges, no labels. Nothing is in question.'
    );
  });

  it("How it's governed: a pending challenge renders its grounds", () => {
    fixture.componentRef.setInput('challenges', [
      makeChallenge({
        id: 'chal-pending',
        state: 'pending',
        groundsPrimary: 'outdated_information',
      }),
    ]);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-leg-governed')?.textContent).toContain('outdated_information');
  });

  it("How it's governed: Raise a concern is the gate trigger on the page, all four kinds", () => {
    fixture.componentRef.setInput('canWrite', true);
    fixture.detectChanges();
    const raise = q(fixture, 'epr-home-raise-concern') as HTMLElement & {
      menuItems: { type: string }[];
    };
    expect(raise.tagName.toLowerCase()).toBe('elohim-gate-feedback-trigger');
    expect(raise.menuItems.map(m => m.type)).toEqual(['flag', 'challenge', 'feedback', 'report']);
    expect(fixture.nativeElement.innerHTML).not.toContain('/resource/');

    const raised: unknown[] = [];
    fixture.componentInstance.concern.subscribe(c => raised.push(c));
    raise.dispatchEvent(
      new CustomEvent('feedback-submit', {
        detail: { type: 'challenge', contentId: 'evolution-of-trust', text: 'Outdated' },
      })
    );
    expect(raised).toEqual([{ type: 'challenge', text: 'Outdated' }]);
  });

  it("How it's governed: signed out, one line instead of the trigger", () => {
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-raise-concern')).toBeNull();
    expect(q(fixture, 'epr-home-raise-concern-signed-out')?.textContent).toContain(
      'Sign in to raise a concern.'
    );
  });

  it("How it's governed: labels as warm pills, and the fold carries each challenge's detail", () => {
    fixture.componentRef.setInput('governanceState', {
      labels: [{ labelType: 'needs_review', severity: 'low' }],
      lastUpdated: '2026-08-02 00:00:00',
    });
    fixture.componentRef.setInput('challenges', [
      makeChallenge({ id: 'chal-1', evidence: 'The 2017 figures were revised.' }),
    ]);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-labels')?.textContent).toContain('needs review');
    expect(q(fixture, 'epr-home-challenge')).toBeNull();

    (q(fixture, 'epr-home-fold-toggle-governed') as HTMLElement).click();
    fixture.detectChanges();
    const row = q(fixture, 'epr-home-challenge')!;
    expect(row.textContent).toContain('factual error');
    expect(row.textContent).toContain('The 2017 figures were revised.');
    expect(row.textContent).toContain('filed Aug 1, 2026');
    expect(q(fixture, 'epr-home-fold-governed')?.textContent).toContain(
      'Labels last reviewed Aug 2, 2026'
    );
  });

  it('folds are closed by default and open by their toggle, with aria-expanded', () => {
    fixture.detectChanges();
    for (const leg of ['holds', 'lives', 'governed', 'from']) {
      const toggle = q(fixture, `epr-home-fold-toggle-${leg}`)!;
      expect(toggle.getAttribute('aria-expanded')).toBe('false');
      expect(q(fixture, `epr-home-fold-${leg}`)).toBeNull();
    }
    const opened: string[] = [];
    fixture.componentInstance.foldOpened.subscribe(l => opened.push(l));
    (q(fixture, 'epr-home-fold-toggle-lives') as HTMLElement).click();
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-toggle-lives')?.getAttribute('aria-expanded')).toBe('true');
    expect(q(fixture, 'epr-home-fold-lives')).not.toBeNull();
    expect(opened).toEqual(['lives']);

    (q(fixture, 'epr-home-fold-toggle-lives') as HTMLElement).click();
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-lives')).toBeNull();
  });

  it('a fold request opens the named leg, and a repeat re-opens it after closing', () => {
    fixture.componentRef.setInput('openFold', { leg: 'holds' });
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-holds')).not.toBeNull();

    (q(fixture, 'epr-home-fold-toggle-holds') as HTMLElement).click();
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-holds')).toBeNull();

    fixture.componentRef.setInput('openFold', { leg: 'holds' });
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-holds')).not.toBeNull();
  });

  it('Network detail: checking, then could-not-read, each one line', () => {
    fixture.componentRef.setInput('openFold', { leg: 'holds' });
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-holds')?.textContent).toContain('Checking…');
    fixture.componentRef.setInput('resilience', null);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-holds')?.textContent).toContain(
      "Couldn't read this from here."
    );
  });

  it('Network detail: health in words, facts, one cell per piece with its peer count', () => {
    fixture.componentRef.setInput('openFold', { leg: 'holds' });
    fixture.componentRef.setInput('resilience', {
      contentId: 'evolution-of-trust',
      encoding: { strategy: 'reed-solomon', dataShards: 2, parityShards: 1 },
      distribution: {
        totalShards: 3,
        shardsWithLocations: 2,
        distinctPeers: 3,
        shards: [
          { hash: 's1', shardType: 'data', peerIds: ['a', 'b'], status: 'ok' },
          { hash: 's2', shardType: 'parity', peerIds: ['c'], status: 'ok' },
          { hash: 's3', shardType: 'data', peerIds: [], status: 'missing' },
        ],
      },
      stewardship: { stewardCount: 1, allocations: [] },
      commitments: { activePeers: 1, totalCommittedBytes: 2147483648, totalUsedBytes: 0 },
      health: { score: 1, canSurviveFailures: 1, status: 'healthy' },
    });
    fixture.componentRef.setInput('verification', {
      state: 'done',
      result: {
        verified: false,
        shardsNeeded: 2,
        shardsLocated: 1,
        shardsMissing: 1,
        reconstructionTimeMs: 0,
        error: null,
      },
    });
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-health')?.textContent).toContain(
      'Healthy · can lose 1 peer and still be whole'
    );
    const fold = q(fixture, 'epr-home-fold-holds')!;
    expect(fold.textContent).toContain('2 of 3');
    expect(fold.textContent).toContain('2 GB');
    const cells = Array.from(q(fixture, 'epr-home-pieces')!.querySelectorAll('li'));
    expect(cells.map(c => c.textContent?.trim())).toEqual(['2', '1', '?']);
    expect(cells[1].classList.contains('piece--parity')).toBe(true);
    expect(cells[2].classList.contains('piece--missing')).toBe(true);
    expect(q(fixture, 'epr-home-verify-result')?.textContent).toContain(
      'Not whole · 1 piece missing'
    );
    expect(fold.textContent).not.toMatch(/\d+%/);
  });

  it("Who's here: the attention line, and one line when nobody is counted", () => {
    fixture.componentRef.setInput('engagement', {
      contentId: 'x',
      views: 0,
      uniqueViewers: 0,
      completions: 0,
      completionRate: 0,
      computedAt: '',
    });
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-attention')?.textContent).toContain(
      'No one has been counted here yet.'
    );
  });

  it('Where this lives: See also rows are minted links, discovered ones tagged', () => {
    fixture.componentRef.setInput('openFold', { leg: 'lives' });
    fixture.componentRef.setInput('seeAlso', [
      {
        id: 'rel-1',
        targetId: 'iterated-games',
        target: { title: 'Iterated games' },
        relationshipType: 'RELATES_TO',
        inferenceSource: 'explicit',
      },
      {
        id: 'rel-2',
        targetId: 'concept-reciprocity',
        target: null,
        relationshipType: 'SIMILAR_TO',
        inferenceSource: 'embedding',
      },
    ]);
    fixture.detectChanges();
    const cards = Array.from(
      fixture.nativeElement.querySelectorAll(
        '[data-testid="epr-home-see-also-card"]'
      ) as NodeListOf<HTMLElement>
    );
    expect(cards.map(c => c.getAttribute('data-inference'))).toEqual(['explicit', 'discovered']);
    expect(cards[0].getAttribute('href')).toBe('/epr/iterated-games');
    expect(cards[1].textContent).toContain('Reciprocity');
    expect(cards[1].textContent).toContain('discovered');
    expect(cards[0].textContent).not.toContain('discovered');
  });

  it('Where this lives: See also collapses to one line when empty', () => {
    fixture.componentRef.setInput('openFold', { leg: 'lives' });
    fixture.componentRef.setInput('seeAlso', []);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-lives')?.textContent).toContain(
      'Nothing else is named alongside this yet.'
    );
  });

  it('Where it came from: attestation rows, revoked ones struck with the reason', () => {
    fixture.componentRef.setInput('openFold', { leg: 'from' });
    fixture.componentRef.setInput('attestations', [
      {
        id: 'att-1',
        title: 'Reviewed for accuracy',
        issuerCid: 'uhCAkIssuerAgentKeyLongEnoughToShorten1234',
        proofClass: 'witness',
        createdAt: '2026-08-01 00:00:00',
        revokedAt: null,
      },
      {
        id: 'att-2',
        title: 'Age-appropriate',
        issuerCid: 'uhCAkOther',
        proofClass: 'self-attest',
        createdAt: '2026-08-02 00:00:00',
        revokedAt: '2026-08-03 00:00:00',
        revocationReason: 'issued in error',
      },
    ]);
    fixture.detectChanges();
    const rows = fixture.nativeElement.querySelectorAll('[data-testid="epr-home-attestation"]');
    expect(rows.length).toBe(2);
    expect(rows[0].textContent).toContain('Reviewed for accuracy');
    expect(rows[0].textContent).toContain('witness');
    expect(rows[1].classList.contains('leg__row--revoked')).toBe(true);
    expect(rows[1].textContent).toContain('Revoked: issued in error');
    expect(q(fixture, 'epr-home-attestations')?.textContent).not.toMatch(/\d+%/);
  });

  it('Where it came from: no attestations is one line', () => {
    fixture.componentRef.setInput('openFold', { leg: 'from' });
    fixture.componentRef.setInput('attestations', []);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-from')?.textContent).toContain(
      'No one has attested to this yet.'
    );
  });

  it('Where it came from: contributors stay visible in the leg body with the fold closed', () => {
    const nicky = { id: 'presence-nicky', displayName: 'Nicky Case', presenceState: 'unclaimed' };
    fixture.componentRef.setInput('contributors', [nicky]);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-fold-from')).toBeNull();
    expect(q(fixture, 'viewer-contributors-section')).not.toBeNull();
    expect(q(fixture, 'viewer-contributors-list')).not.toBeNull();
    const card = q(fixture, 'viewer-contributor-card-presence-nicky') as HTMLElement & {
      presence: unknown;
    };
    expect(card.tagName.toLowerCase()).toBe('elohim-contributor-card');
    expect(card.presence).toEqual(nicky);
  });

  it('Where it came from: the download link only when there is a blob', () => {
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-download')).toBeNull();
    fixture.componentRef.setInput('downloadHref', 'https://doorway.test/blob/sha256-abc');
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-download')?.getAttribute('href')).toBe(
      'https://doorway.test/blob/sha256-abc'
    );
  });

  it('Who holds it: the peers row is absent when nothing is measured (0 or null)', () => {
    fixture.componentRef.setInput('peersHolding', 0);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-leg-holds')?.textContent).not.toContain('peers keep a copy');

    fixture.componentRef.setInput('peersHolding', null);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-leg-holds')?.textContent).not.toContain('peers keep a copy');
  });

  it('Who holds it: no blank invite button when a protected atom has no suggested action', () => {
    fixture.componentRef.setInput('snapshot', protectedSnapshot);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-invite-household')).toBeNull();
  });
});
