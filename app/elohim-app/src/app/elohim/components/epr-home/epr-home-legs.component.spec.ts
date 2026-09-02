import { Component, Input } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { beforeEach, describe, expect, it } from 'vitest';

import { EprHomeLegsComponent } from './epr-home-legs.component';
import type { EprRelationship } from '../../models/epr-head.model';
import { EprRelationshipsPanelComponent } from '../epr-relationships-panel/epr-relationships-panel.component';
import { toAtom } from './epr-home.model';

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
  metadata: { sourceUrl: 'https://github.com/ncase/trust', license: 'CC0 Public Domain', relatedNodeIds: ['concept-bidirectional-trust'] },
});

const snapshot = {
  contentId: 'evolution-of-trust',
  feltStatus: {
    headline: 'Held by only 1 household — invite another to help hold these',
    reassurance: 'needs-help',
    heldBy: [{ id: 'household-dowell', kind: 'household', label: 'Dowell Household', intraHubPeers: 2 }],
    floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 1 },
    suggestedAction: 'Invite a household to help hold these',
  },
};

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

  it('Who holds it: the felt headline, the floor, the households, the action', () => {
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
  });

  it('Who holds it: collapses to one line when nothing is known', () => {
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-leg-holds')?.textContent).toContain(
      "We can't confirm this is backed up anywhere yet."
    );
  });

  it('Where this lives: names related ids and tags unreachable ones', () => {
    fixture.detectChanges();
    const leg = q(fixture, 'epr-home-leg-lives')!;
    expect(leg.textContent).toContain('Bidirectional trust');
    expect(leg.querySelector('a[href="/epr/concept-bidirectional-trust"]')).not.toBeNull();
  });

  it('Where this lives: keeps the relationships panel wrapper test id', () => {
    fixture.componentRef.setInput('relationships', [{ type: 'REFERENCES', target: 'governance-epic' }]);
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
      { stewardPresenceId: 'matthew-dowell', contributionType: 'original_creator', effectiveFrom: '2026-06-04 02:19:10' },
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
});
