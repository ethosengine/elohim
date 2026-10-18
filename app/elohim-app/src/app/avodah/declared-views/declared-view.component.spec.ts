import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { ActivatedRoute } from '@angular/router';
import { describe, expect, it, vi } from 'vitest';

import { DeclaredViewComponent } from './declared-view.component';
import { DeclaredViewService } from './declared-view.service';
import { presentation, type Reading } from './view-reading';
import type { Subject } from './view-contract';

describe('observation-bound detail selection', () => {
  it('clears open card and source when refresh replaces the reading and removes that source', () => {
    const subject: Subject = {
      identity: 'north',
      source: 'north.md',
      version: 'v1',
      fields: { $group: 'goals', $title: 'North orchard' },
      readings: {},
      relationships: {},
    };
    const first: Reading = {
      observation: {
        sequence: 1,
        snapshotCid: 's1',
        declarationCid: 'd1',
        observedAt: '2026-10-13T00:00:00Z',
      },
      projection: {
        version: 2,
        composition: {
          authority: 'Local interpretation',
          views: ['Progress', 'Board', 'Burndown', 'Sources'],
          links: [],
          groups: [
            { id: 'goals', label: 'Goals', role: 'focus', presentation: presentation() },
            { id: 'plots', label: 'Plots', role: 'work', presentation: presentation() },
          ],
        },
        purpose: 'orchard',
        declaration: 'view.json',
        declaration_cid: 'd1',
        method: 'native',
        reader_versions: {},
        subjects: [subject],
        sources: { 'north.md': 'v1' },
        input_versions: {},
        governance: {},
        diagnostics: [],
      },
      presentation: presentation(),
    };
    const readings = signal([first]);
    const source = vi.fn(() => 'original captured bytes');
    TestBed.configureTestingModule({
      providers: [
        { provide: DeclaredViewService, useValue: { readings, selected: signal(0), source } },
        { provide: ActivatedRoute, useValue: {} },
      ],
    });
    const component = TestBed.runInInjectionContext(() => new DeclaredViewComponent());
    expect(component.tab()).toBe('Progress');
    expect(component.progressSubjects()).toHaveLength(1);
    component.progressQuery.set('missing');
    expect(component.progressSubjects()).toHaveLength(0);
    expect(component.focusSubjects()).toHaveLength(1);
    component.group.set('goals');
    component.detail.set(subject);
    component.showSource('north.md');
    expect(component.sourceText()).toBe('original captured bytes');
    readings.set([
      {
        ...first,
        observation: { ...first.observation, sequence: 2 },
        projection: { ...first.projection, subjects: [], sources: {} },
      },
    ]);
    expect(component.detail()).toBeNull();
    expect(component.sourceText()).toBe('');
    expect(component.tab()).toBe('Sources');
    expect(component.group()).toBe('goals');
    expect(source).toHaveBeenCalledTimes(1);
  });
});
