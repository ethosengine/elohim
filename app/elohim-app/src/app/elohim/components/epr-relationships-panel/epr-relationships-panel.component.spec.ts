/**
 * EprRelationshipsPanelComponent Tests
 *
 * Covers: empty state (no DOM), grouping by type, fixed ordering of known types,
 * unknown types trailing after known types.
 */

import { vi, describe, it, expect, beforeEach } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';
import { of } from 'rxjs';

import { EprRelationshipsPanelComponent } from './epr-relationships-panel.component';
import { EprResolverService } from '../../services/epr-resolver.service';
import { ResilienceService } from '@app/lamad/services/resilience.service';
import type { EprRelationship } from '../../models/epr-head.model';

// ── Helpers ───────────────────────────────────────────────────────────────────

function queryAll(fixture: ComponentFixture<EprRelationshipsPanelComponent>, testId: string): NodeListOf<Element> {
  return fixture.nativeElement.querySelectorAll(`[data-testid="${testId}"]`);
}

function query(fixture: ComponentFixture<EprRelationshipsPanelComponent>, testId: string): Element | null {
  return fixture.nativeElement.querySelector(`[data-testid="${testId}"]`);
}

// ── Suite ─────────────────────────────────────────────────────────────────────

describe('EprRelationshipsPanelComponent', () => {
  let fixture: ComponentFixture<EprRelationshipsPanelComponent>;
  let component: EprRelationshipsPanelComponent;

  beforeEach(async () => {
    const mockResolver = { resolve: vi.fn().mockReturnValue(of(null)) };
    const mockResilience = { getContentResilience: vi.fn().mockReturnValue(of(null)) };

    await TestBed.configureTestingModule({
      imports: [EprRelationshipsPanelComponent, RouterModule.forRoot([])],
      providers: [
        { provide: EprResolverService, useValue: mockResolver },
        { provide: ResilienceService, useValue: mockResilience },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(EprRelationshipsPanelComponent);
    component = fixture.componentInstance;
  });

  // ── Empty state ───────────────────────────────────────────────────────────

  it('renders nothing when relationships are empty', () => {
    component.relationships = [];
    fixture.detectChanges();

    const panel = query(fixture, 'epr-relationships-panel');
    expect(panel).toBeNull();
  });

  // ── Grouping ──────────────────────────────────────────────────────────────

  it('groups relationships by type with a section per group', () => {
    const rels: EprRelationship[] = [
      { type: 'PREREQUISITE', target: 'intro-a' },
      { type: 'PREREQUISITE', target: 'intro-b' },
      { type: 'TEACHES', target: 'concept-c' },
    ];
    component.relationships = rels;
    fixture.detectChanges();

    const groups = queryAll(fixture, 'epr-rel-group');
    expect(groups.length).toBe(2);

    // PREREQUISITE group is first; it should contain 2 cards
    const prerequisiteGroup = Array.from(groups).find(
      (g) => g.getAttribute('data-type') === 'PREREQUISITE',
    );
    expect(prerequisiteGroup).toBeTruthy();
    const cards = prerequisiteGroup!.querySelectorAll('[data-testid="epr-relationship-card"]');
    expect(cards.length).toBe(2);
  });

  // ── Ordering (known types) ─────────────────────────────────────────────────

  it('orders groups PREREQUISITE → TEACHES → CONTAINS → REFERENCES', () => {
    // Deliberately shuffled
    const rels: EprRelationship[] = [
      { type: 'REFERENCES', target: 'ref-1' },
      { type: 'CONTAINS', target: 'child-1' },
      { type: 'TEACHES', target: 'concept-1' },
      { type: 'PREREQUISITE', target: 'intro-1' },
    ];
    component.relationships = rels;
    fixture.detectChanges();

    const groups = queryAll(fixture, 'epr-rel-group');
    const types = Array.from(groups).map((g) => g.getAttribute('data-type'));
    expect(types).toEqual(['PREREQUISITE', 'TEACHES', 'CONTAINS', 'REFERENCES']);
  });

  // ── Unknown types trail ────────────────────────────────────────────────────

  it('renders unknown types trailing after known types', () => {
    const rels: EprRelationship[] = [
      { type: 'CITES', target: 'paper-1' },
      { type: 'TEACHES', target: 'concept-1' },
    ];
    component.relationships = rels;
    fixture.detectChanges();

    const groups = queryAll(fixture, 'epr-rel-group');
    const types = Array.from(groups).map((g) => g.getAttribute('data-type'));
    expect(types).toEqual(['TEACHES', 'CITES']);
  });
});
