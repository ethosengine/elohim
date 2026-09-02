import { Component, EventEmitter, Input, Output, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ActivatedRoute, convertToParamMap, provideRouter } from '@angular/router';

import { BehaviorSubject, Subject, of, throwError } from 'rxjs';
import { beforeEach, describe, expect, it, vi } from 'vitest';

import { GovernanceApiService } from '@elohim/service';
import { DistributionService, ResilienceService } from '@elohim/service/public-api';

import { EprHomeComponent } from './epr-home.component';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';
import { AuthService } from '../../../imagodei/services/auth.service';
import { SeoService } from '../../../services/seo.service';
import { AffinityTrackingService } from '../../services/affinity-tracking.service';
import { EprResolverService } from '../../services/epr-resolver.service';
import { SessionNavStackService } from '../../services/session-nav-stack.service';
import { StorageApiService } from '../../services/storage-api.service';
import { StorageClientService } from '../../services/storage-client.service';

import {
  anchorWords,
  focalShape,
  heldChip,
  holdingWords,
  reachSubtitle,
  shortAnchor,
  toAtom,
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
      metadata: { author: 'Nicky Case', license: 'CC0 Public Domain', relatedNodeIds: ['a', 'b'] },
    });
    expect(atom.shape).toBe('immersive');
    expect(atom.author).toBe('Nicky Case');
    expect(atom.relatedIds).toEqual(['a', 'b']);
    expect(reachSubtitle(atom.reach)).toBe('anyone can reach this');
    expect(shortAnchor(atom.dhtAnchorHash!)).toBe('uhCkk_D-fLh9…KujiEm8f');
  });

  it('renders the felt status as one verdict in household words', () => {
    const words = holdingWords({
      contentId: 'x',
      feltStatus: {
        headline: 'Held by only 1 household — invite another to help hold these',
        reassurance: 'needs-help',
        heldBy: [{ id: 'household-dowell', kind: 'household', label: 'Dowell Household', intraHubPeers: 2 }],
        floor: { tier: 'standard', tierDeclared: false, wantsHouseholds: 3, hasHouseholds: 1 },
        suggestedAction: 'Invite a household to help hold these',
      },
    } as never);
    expect(words.has).toBe(1);
    expect(words.warm).toBe(true);
    expect(words.households).toEqual(['Dowell Household · 2 peers']);
    expect(heldChip(words)).toBe('Held by 1 of 3 households');
    expect(heldChip(holdingWords(null))).toBe('Not yet held by any household');
  });
});

@Component({ selector: 'app-epr-focal', standalone: true, template: '<div class="focal-stub"></div>' })
class EprFocalStub {
  @Input() slug = '';
  @Output() nodeLoaded = new EventEmitter<unknown>();
  @Output() notFound = new EventEmitter<string>();
  @Output() failed = new EventEmitter<string>();
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
  createdAt: '2026-05-27 20:46:37',
  updatedAt: '2026-08-05 18:40:53',
  metadata: { author: 'Nicky Case', license: 'CC0 Public Domain', estimatedTime: '30 minutes' },
};

function q(fixture: ComponentFixture<EprHomeComponent>, id: string): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${id}"]`);
}

describe('EprHomeComponent', () => {
  let fixture: ComponentFixture<EprHomeComponent>;
  let storage: { getContent: ReturnType<typeof vi.fn> };
  let navStack: { previous: ReturnType<typeof vi.fn>; record: ReturnType<typeof vi.fn> };
  let auth: { isAuthenticated: ReturnType<typeof signal<boolean>> };
  let affinity: {
    getAffinity: ReturnType<typeof vi.fn>;
    setAffinity: ReturnType<typeof vi.fn>;
    trackView: ReturnType<typeof vi.fn>;
    affinity$: ReturnType<typeof of>;
  };
  let seo: { updateForContent: ReturnType<typeof vi.fn>; setTitle: ReturnType<typeof vi.fn> };

  async function mount(resourceId: string): Promise<void> {
    await TestBed.configureTestingModule({
      imports: [EprHomeComponent],
      providers: [
        provideRouter([]),
        { provide: StorageClientService, useValue: storage },
        {
          provide: ActivatedRoute,
          useValue: { paramMap: of(convertToParamMap({ resourceId })) },
        },
        { provide: ResilienceService, useValue: { getSnapshot: vi.fn().mockReturnValue(of(null)) } },
        {
          provide: DistributionService,
          useValue: { getDetails: vi.fn().mockResolvedValue({ summary: { replicaCount: 5 } }) },
        },
        {
          provide: StorageApiService,
          useValue: { getStewardshipAllocations: vi.fn().mockReturnValue(of([])) },
        },
        {
          provide: EprResolverService,
          useValue: { resolveEprHead: vi.fn().mockReturnValue(of(null)) },
        },
        {
          provide: GovernanceApiService,
          useValue: { getChallengesForEntity: vi.fn().mockResolvedValue([]) },
        },
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
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  beforeEach(() => {
    storage = { getContent: vi.fn().mockReturnValue(of(rawSimulation)) };
    navStack = { previous: vi.fn().mockReturnValue(null), record: vi.fn() };
    auth = { isAuthenticated: signal(false) };
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

  it('hands the slug to the focal slot in the immersive shape', async () => {
    await mount('evolution-of-trust');
    const focal = q(fixture, 'epr-home-focal');
    expect(focal?.classList.contains('epr-home__focal--immersive')).toBe(true);
    expect(focal?.querySelector('.focal-stub')).not.toBeNull();
  });

  it('uses the reading shape for markdown', async () => {
    storage.getContent.mockReturnValue(of({ ...rawSimulation, contentFormat: 'markdown' }));
    await mount('succession');
    expect(q(fixture, 'epr-home-focal')?.classList.contains('epr-home__focal--reading')).toBe(
      true
    );
  });

  it('renders the out-of-reach gate for a null atom, with no chrome', async () => {
    storage.getContent.mockReturnValue(of(null));
    await mount('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain("We can't reach this one from here");
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-your-mark')).toBeNull();
    expect(q(fixture, 'epr-home-focal')).toBeNull();
  });

  it('renders the error state when the load fails', async () => {
    storage.getContent.mockReturnValue(throwError(() => new Error('boom')));
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-error')).not.toBeNull();
  });

  it('carries the universal address line', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-address')?.textContent).toContain('/epr/evolution-of-trust');
  });

  it('shows the held-by chip from the snapshot and renders all four legs', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-chip-held')?.textContent).toContain(
      'Not yet held by any household'
    );
    for (const leg of ['holds', 'lives', 'governed', 'from']) {
      expect(q(fixture, `epr-home-leg-${leg}`)).not.toBeNull();
    }
  });

  it('resets to loading between two atoms on the same component instance', async () => {
    const paramMap$ = new BehaviorSubject(convertToParamMap({ resourceId: 'evolution-of-trust' }));
    const second$ = new Subject<unknown>();
    storage.getContent = vi
      .fn()
      .mockReturnValueOnce(of(rawSimulation))
      .mockReturnValueOnce(second$);

    await TestBed.configureTestingModule({
      imports: [EprHomeComponent],
      providers: [
        provideRouter([]),
        { provide: StorageClientService, useValue: storage },
        { provide: ActivatedRoute, useValue: { paramMap: paramMap$ } },
        { provide: ResilienceService, useValue: { getSnapshot: vi.fn().mockReturnValue(of(null)) } },
        {
          provide: DistributionService,
          useValue: { getDetails: vi.fn().mockResolvedValue({ summary: { replicaCount: 5 } }) },
        },
        {
          provide: StorageApiService,
          useValue: { getStewardshipAllocations: vi.fn().mockReturnValue(of([])) },
        },
        {
          provide: EprResolverService,
          useValue: { resolveEprHead: vi.fn().mockReturnValue(of(null)) },
        },
        {
          provide: GovernanceApiService,
          useValue: { getChallengesForEntity: vi.fn().mockResolvedValue([]) },
        },
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
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();

    expect(q(fixture, 'epr-home-title')?.textContent).toContain('The Evolution of Trust');

    paramMap$.next(convertToParamMap({ resourceId: 'succession' }));
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-loading')).not.toBeNull();

    second$.next({ ...rawSimulation, id: 'succession', title: 'Succession' });
    second$.complete();
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-title')?.textContent).toContain('Succession');
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
    storage.getContent.mockReturnValue(of(null));
    navStack.previous.mockReturnValue({
      url: '/epr/evolution-of-trust',
      cid: '',
      label: 'The Evolution of Trust | Elohim Protocol',
      ts: 1,
    });
    await mount('concept-bidirectional-trust');
    expect(q(fixture, 'epr-home-gate')?.textContent).toContain('The Evolution of Trust');
    expect(q(fixture, 'epr-home-gate-back')?.getAttribute('href')).toBe('/epr/evolution-of-trust');
  });

  it('shows Your mark only when signed in, as one row without a percentage badge', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-your-mark')).toBeNull();
    auth.isAuthenticated.set(true);
    fixture.detectChanges();
    expect(q(fixture, 'epr-home-your-mark')?.textContent).toContain('Practicing · 20%');
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
  });

  it('renders the focal stub and no claimed card for an unclaimed contentType', async () => {
    await mount('evolution-of-trust');
    expect(q(fixture, 'epr-home-focal-claimed')).toBeNull();
    expect(fixture.nativeElement.querySelector('.focal-stub')).not.toBeNull();
  });
});
