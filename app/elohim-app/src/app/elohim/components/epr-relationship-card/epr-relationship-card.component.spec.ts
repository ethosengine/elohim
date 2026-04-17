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
  route: ['/resource', 'systems-thinking'],
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

  // ── Routing ───────────────────────────────────────────────────────────────

  it('links to the resolved route', () => {
    const card = query(fixture, 'epr-relationship-card');
    expect(card).toBeTruthy();
    const href = card!.getAttribute('href');
    expect(href).toContain('systems-thinking');
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
