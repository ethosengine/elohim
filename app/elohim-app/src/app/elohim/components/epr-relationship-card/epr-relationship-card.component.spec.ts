/**
 * EprRelationshipCardComponent Tests
 *
 * Covers: title resolution, type label, reach badge, resilience badge,
 * link routing, and fallback when resolution returns null.
 *
 * Resolution now flows through the ambient EprResolutionProvider (I2):
 * `resolveHead` is Promise-based, so the card resolves asynchronously — each
 * test flushes a macrotask (`setTimeout(0)`) after `ngOnChanges` before asserting.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';

import { of } from 'rxjs';

import { EprRelationshipCardComponent } from './epr-relationship-card.component';
import { EPR_RESOLUTION_PROVIDER } from '../../providers/epr-resolution.provider';
import { ResilienceService } from '@app/lamad/services/resilience.service';
import type { EprRelationship } from '../../models/epr-head.model';
import type { EprHeadResolution } from 'elohim-core';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const headResolution: EprHeadResolution = {
  state: 'resolved',
  head: {
    id: 'systems-thinking',
    title: 'Systems Thinking',
    description: 'An introduction.',
    reach: 'community',
    contentType: 'concept',
    contentFormat: 'markdown',
    tags: [],
    route: ['/epr', 'systems-thinking'],
    href: '/epr/systems-thinking',
  },
};

const resilienceView = {
  contentId: 'systems-thinking',
  encoding: {
    strategy: 'rs',
    dataShards: 4,
    parityShards: 2,
    totalSizeBytes: 0,
    shardSizeBytes: 0,
  },
  distribution: { totalShards: 6, shardsWithLocations: 6, distinctPeers: 3, shards: [] },
  stewardship: { stewardCount: 3, allocations: [] },
  commitments: { activePeers: 3, totalCommittedBytes: 0, totalUsedBytes: 0 },
  health: { score: 0.9, canSurviveFailures: 2, status: 'healthy' },
};

const prerequisiteRelationship: EprRelationship = {
  type: 'PREREQUISITE',
  target: 'systems-thinking',
};

// ── Helpers ───────────────────────────────────────────────────────────────────

function query(
  fixture: ComponentFixture<EprRelationshipCardComponent>,
  testId: string
): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${testId}"]`);
}

// ── Suite ─────────────────────────────────────────────────────────────────────

describe('EprRelationshipCardComponent', () => {
  let fixture: ComponentFixture<EprRelationshipCardComponent>;
  let component: EprRelationshipCardComponent;
  let mockResolution: {
    resolveHead: ReturnType<typeof vi.fn>;
    resolveRoute: ReturnType<typeof vi.fn>;
    resolveBody: ReturnType<typeof vi.fn>;
  };
  let mockResilience: { getContentResilience: ReturnType<typeof vi.fn> };

  /** Set the relationship, trigger change detection, and flush the async head. */
  async function apply(rel: EprRelationship): Promise<void> {
    component.relationship = rel;
    component.ngOnChanges({ relationship: {} as never });
    fixture.detectChanges();
    // resolveHead is Promise-based (from() emits on a microtask); flush a
    // macrotask so the resolved state is rendered before we assert.
    await new Promise(resolve => setTimeout(resolve, 0));
    fixture.detectChanges();
  }

  beforeEach(async () => {
    mockResolution = {
      resolveHead: vi.fn().mockResolvedValue(headResolution),
      resolveRoute: vi
        .fn()
        .mockReturnValue({ route: ['/epr', 'systems-thinking'], href: '/epr/systems-thinking' }),
      resolveBody: vi.fn().mockResolvedValue({ state: 'missing' }),
    };
    mockResilience = { getContentResilience: vi.fn().mockReturnValue(of(resilienceView)) };

    await TestBed.configureTestingModule({
      imports: [EprRelationshipCardComponent, RouterModule.forRoot([])],
      providers: [
        { provide: EPR_RESOLUTION_PROVIDER, useValue: mockResolution },
        { provide: ResilienceService, useValue: mockResilience },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(EprRelationshipCardComponent);
    component = fixture.componentInstance;

    await apply(prerequisiteRelationship);
  });

  // ── Title ──────────────────────────────────────────────────────────────────

  it('renders target title after resolution', () => {
    const el = query(fixture, 'epr-rel-card-title');
    expect(el).toBeTruthy();
    expect(el!.textContent).toContain('Systems Thinking');
  });

  // ── Type label ────────────────────────────────────────────────────────────

  it('renders relationship type label as "Prerequisite"', () => {
    const el = query(fixture, 'epr-rel-card-type');
    expect(el).toBeTruthy();
    expect(el!.textContent).toContain('Prerequisite');
  });

  // ── Reach badge ───────────────────────────────────────────────────────────

  it('renders reach badge with title attribute containing "community"', () => {
    const el = query(fixture, 'epr-rel-card-reach');
    expect(el).toBeTruthy();
    expect(el!.getAttribute('title')).toContain('community');
  });

  // ── Resilience badge ──────────────────────────────────────────────────────

  it('renders resilience badge with title attribute containing steward count', () => {
    const el = query(fixture, 'epr-rel-card-resilience');
    expect(el).toBeTruthy();
    expect(el!.getAttribute('title')).toContain('3');
  });

  // ── Peers count badge ─────────────────────────────────────────────────────

  it('renders the distinct-peers badge when distinctPeers > 0', () => {
    const el = query(fixture, 'epr-rel-card-peers');
    expect(el).toBeTruthy();
    // distinctPeers is 3 in the fixture
    expect(el!.textContent).toContain('3');
  });

  it('hides the distinct-peers badge when distinctPeers is 0', async () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({
        ...resilienceView,
        distribution: { ...resilienceView.distribution, distinctPeers: 0 },
      })
    );

    await apply({ type: 'PREREQUISITE', target: 'systems-thinking' });

    expect(query(fixture, 'epr-rel-card-peers')).toBeNull();
  });

  it('hides the distinct-peers badge when resilience is unavailable', async () => {
    mockResilience.getContentResilience.mockReturnValue(of(null));

    await apply({ type: 'PREREQUISITE', target: 'systems-thinking' });

    expect(query(fixture, 'epr-rel-card-peers')).toBeNull();
  });

  // ── Malformed resilience (regression: stewardCount TypeError) ──────────────

  it('renders legibly and hides the steward badge when resilience is a truthy-but-malformed body', async () => {
    // Through the start:alpha proxy the resilience route can answer with a
    // truthy body that lacks `stewardship` (SPA fallback / partial view).
    // Reading `stewardship.stewardCount` off that threw the console TypeError
    // and blanked the card. The card must stay legible with only head data.
    mockResilience.getContentResilience.mockReturnValue(of({ contentId: 'systems-thinking' }));

    await apply({ type: 'PREREQUISITE', target: 'systems-thinking' });

    // Title (head data) still renders; the resilience badge is hidden.
    expect(query(fixture, 'epr-rel-card-title')!.textContent).toContain('Systems Thinking');
    expect(query(fixture, 'epr-rel-card-resilience')).toBeNull();
    expect(query(fixture, 'epr-rel-card-peers')).toBeNull();
  });

  it('hides the steward badge when stewardship is present but stewardCount is not a number', async () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({ ...resilienceView, stewardship: { allocations: [] } })
    );

    await apply({ type: 'PREREQUISITE', target: 'systems-thinking' });

    expect(query(fixture, 'epr-rel-card-resilience')).toBeNull();
  });

  // ── Resilience tooltip enrichment ─────────────────────────────────────────

  it('enriches the resilience tooltip with distinct peers, k-of-n shards and survivable failures', () => {
    const el = query(fixture, 'epr-rel-card-resilience');
    expect(el).toBeTruthy();
    const title = el!.getAttribute('title') ?? '';
    // distinctPeers = 3, shardsWithLocations/totalShards = 6/6, canSurviveFailures = 2
    expect(title).toContain('Distinct peers: 3');
    expect(title).toContain('Shards placed: 6/6');
    expect(title).toContain('Survives 2 failure(s)');
  });

  it('omits the distinct-peers tooltip fragment when distinctPeers is 0', async () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({
        ...resilienceView,
        distribution: { ...resilienceView.distribution, distinctPeers: 0 },
      })
    );

    await apply({ type: 'PREREQUISITE', target: 'systems-thinking' });

    const el = query(fixture, 'epr-rel-card-resilience');
    const title = el!.getAttribute('title') ?? '';
    expect(title).not.toContain('Distinct peers');
    // Survivable-failures fragment is always present.
    expect(title).toContain('Survives 2 failure(s)');
  });

  // ── Glyph regression guard ────────────────────────────────────────────────

  it('keeps ● glyph semantics for stewardCount >= 3', () => {
    const el = query(fixture, 'epr-rel-card-resilience');
    // Fixture stewardCount is 3 → solid glyph.
    expect(el!.textContent).toContain('●');
  });

  it('renders ◐ glyph for 1 <= stewardCount < 3', async () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({
        ...resilienceView,
        stewardship: { stewardCount: 1, allocations: [] },
      })
    );

    await apply({ type: 'PREREQUISITE', target: 'systems-thinking' });

    const el = query(fixture, 'epr-rel-card-resilience');
    expect(el!.textContent).toContain('◐');
  });

  it('renders ○ glyph for stewardCount === 0', async () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({
        ...resilienceView,
        stewardship: { stewardCount: 0, allocations: [] },
      })
    );

    await apply({ type: 'PREREQUISITE', target: 'systems-thinking' });

    const el = query(fixture, 'epr-rel-card-resilience');
    expect(el!.textContent).toContain('○');
  });

  // ── Routing ───────────────────────────────────────────────────────────────

  it('links to the resolved route', () => {
    const card = query(fixture, 'epr-relationship-card');
    expect(card).toBeTruthy();
    const href = card!.getAttribute('href');
    expect(href).toContain('systems-thinking');
  });

  it('uses the href fallback anchor when route is null (cross-bundle)', async () => {
    mockResolution.resolveHead.mockResolvedValue({
      state: 'resolved',
      head: { ...headResolution.head, route: null, href: '/epr/systems-thinking' },
    });

    await apply({ type: 'PREREQUISITE', target: 'systems-thinking' });

    const card = query(fixture, 'epr-relationship-card');
    expect(card).toBeTruthy();
    // href anchor must carry the universal address
    expect(card!.getAttribute('href')).toBe('/epr/systems-thinking');
    // and must NOT have a routerLink attribute (which would produce an empty route)
    expect(card!.hasAttribute('ng-reflect-router-link')).toBe(false);
  });

  // ── Fallback ──────────────────────────────────────────────────────────────

  it('falls back to target id when the head does not resolve', async () => {
    mockResolution.resolveHead.mockResolvedValue({ state: 'missing' });
    mockResilience.getContentResilience.mockReturnValue(of(null));

    await apply({ type: 'REFERENCES', target: 'unknown-content' });

    const el = query(fixture, 'epr-rel-card-title');
    expect(el).toBeTruthy();
    expect(el!.textContent).toContain('unknown-content');
  });
});
