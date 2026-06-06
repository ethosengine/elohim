/**
 * EprRelationshipCardComponent Tests
 *
 * Covers: title resolution, type label, reach badge, resilience badge,
 * link routing, and fallback when resolution returns null.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';
import { of } from 'rxjs';

import { EprRelationshipCardComponent } from './epr-relationship-card.component';
import { EprResolverService } from '../../services/epr-resolver.service';
import { ResilienceService } from '@app/lamad/services/resilience.service';
import type { EprRelationship } from '../../models/epr-head.model';

// ── Fixtures ──────────────────────────────────────────────────────────────────

const resolvedContent = {
  ref: { id: 'systems-thinking', tier: 'doc' as const },
  content: {
    id: 'systems-thinking',
    title: 'Systems Thinking',
    description: 'An introduction.',
    contentType: 'concept',
    contentFormat: 'markdown',
    reach: 'community',
    contentBody: null,
    blobHash: null,
    blobCid: null,
    metadataJson: null,
    tags: [],
    createdAt: '',
    updatedAt: '',
  },
  route: ['/epr', 'systems-thinking'],
  href: '/epr/systems-thinking',
  blobUrl: null,
};

const resilienceView = {
  contentId: 'systems-thinking',
  encoding: { strategy: 'rs', dataShards: 4, parityShards: 2, totalSizeBytes: 0, shardSizeBytes: 0 },
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

function query(fixture: ComponentFixture<EprRelationshipCardComponent>, testId: string): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${testId}"]`);
}

// ── Suite ─────────────────────────────────────────────────────────────────────

describe('EprRelationshipCardComponent', () => {
  let fixture: ComponentFixture<EprRelationshipCardComponent>;
  let component: EprRelationshipCardComponent;
  let mockResolver: { resolve: ReturnType<typeof vi.fn> };
  let mockResilience: { getContentResilience: ReturnType<typeof vi.fn> };

  beforeEach(async () => {
    mockResolver = { resolve: vi.fn().mockReturnValue(of(resolvedContent)) };
    mockResilience = { getContentResilience: vi.fn().mockReturnValue(of(resilienceView)) };

    await TestBed.configureTestingModule({
      imports: [EprRelationshipCardComponent, RouterModule.forRoot([])],
      providers: [
        { provide: EprResolverService, useValue: mockResolver },
        { provide: ResilienceService, useValue: mockResilience },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(EprRelationshipCardComponent);
    component = fixture.componentInstance;

    // Set input and manually trigger ngOnChanges (standard Angular test pattern
    // when inputs are set programmatically rather than via template binding).
    component.relationship = prerequisiteRelationship;
    component.ngOnChanges({ relationship: {} as any });
    fixture.detectChanges();
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

  it('hides the distinct-peers badge when distinctPeers is 0', () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({
        ...resilienceView,
        distribution: { ...resilienceView.distribution, distinctPeers: 0 },
      })
    );

    component.relationship = { type: 'PREREQUISITE', target: 'systems-thinking' };
    component.ngOnChanges({ relationship: {} as any });
    fixture.detectChanges();

    expect(query(fixture, 'epr-rel-card-peers')).toBeNull();
  });

  it('hides the distinct-peers badge when resilience is unavailable', () => {
    mockResilience.getContentResilience.mockReturnValue(of(null));

    component.relationship = { type: 'PREREQUISITE', target: 'systems-thinking' };
    component.ngOnChanges({ relationship: {} as any });
    fixture.detectChanges();

    expect(query(fixture, 'epr-rel-card-peers')).toBeNull();
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

  it('omits the distinct-peers tooltip fragment when distinctPeers is 0', () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({
        ...resilienceView,
        distribution: { ...resilienceView.distribution, distinctPeers: 0 },
      })
    );

    component.relationship = { type: 'PREREQUISITE', target: 'systems-thinking' };
    component.ngOnChanges({ relationship: {} as any });
    fixture.detectChanges();

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

  it('renders ◐ glyph for 1 <= stewardCount < 3', () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({
        ...resilienceView,
        stewardship: { stewardCount: 1, allocations: [] },
      })
    );

    component.relationship = { type: 'PREREQUISITE', target: 'systems-thinking' };
    component.ngOnChanges({ relationship: {} as any });
    fixture.detectChanges();

    const el = query(fixture, 'epr-rel-card-resilience');
    expect(el!.textContent).toContain('◐');
  });

  it('renders ○ glyph for stewardCount === 0', () => {
    mockResilience.getContentResilience.mockReturnValue(
      of({
        ...resilienceView,
        stewardship: { stewardCount: 0, allocations: [] },
      })
    );

    component.relationship = { type: 'PREREQUISITE', target: 'systems-thinking' };
    component.ngOnChanges({ relationship: {} as any });
    fixture.detectChanges();

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

  it('uses the href fallback anchor when route is null (cross-bundle)', () => {
    mockResolver.resolve.mockReturnValue(
      of({ ...resolvedContent, route: null, href: '/epr/systems-thinking' }),
    );

    component.relationship = { type: 'PREREQUISITE', target: 'systems-thinking' };
    component.ngOnChanges({ relationship: {} as any });
    fixture.detectChanges();

    const card = query(fixture, 'epr-relationship-card');
    expect(card).toBeTruthy();
    // href anchor must carry the universal address
    expect(card!.getAttribute('href')).toBe('/epr/systems-thinking');
    // and must NOT have a routerLink attribute (which would produce an empty route)
    expect(card!.hasAttribute('ng-reflect-router-link')).toBe(false);
  });

  // ── Fallback ──────────────────────────────────────────────────────────────

  it('falls back to target id when resolution returns null', () => {
    mockResolver.resolve.mockReturnValue(of(null));
    mockResilience.getContentResilience.mockReturnValue(of(null));

    component.relationship = { type: 'REFERENCES', target: 'unknown-content' };
    component.ngOnChanges({ relationship: {} as any });
    fixture.detectChanges();

    const el = query(fixture, 'epr-rel-card-title');
    expect(el).toBeTruthy();
    expect(el!.textContent).toContain('unknown-content');
  });
});
