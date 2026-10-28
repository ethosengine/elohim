import { CommonModule } from '@angular/common';
import { Component, OnInit, computed, inject, linkedSignal, signal } from '@angular/core';
import { FormsModule } from '@angular/forms';
import { ActivatedRoute } from '@angular/router';

import { DeclaredViewService } from './declared-view.service';
import { acceptance, burndown, field } from './view-reading';
import {
  claims,
  collection,
  evidenceSummary,
  movement,
  remainingActions,
  strings,
} from './work-progress';

import type { Subject } from './view-contract';

@Component({
  selector: 'app-declared-view',
  standalone: true,
  imports: [CommonModule, FormsModule],
  providers: [DeclaredViewService],
  templateUrl: './declared-view.component.html',
  styleUrl: './declared-view.component.css',
})
export class DeclaredViewComponent implements OnInit {
  readonly view = inject(DeclaredViewService);
  private readonly route = inject(ActivatedRoute);
  readonly tab = linkedSignal({
    source: () => Boolean(this.current()?.projection.composition),
    computation: (composed, previous): string => {
      if (previous?.source === composed) return previous.value;
      return composed ? 'Progress' : 'Board';
    },
  });
  readonly group = linkedSignal({
    source: () =>
      this.current()
        ?.projection.composition?.groups.map(g => g.id)
        .join(','),
    computation: (groups, previous): string => {
      if (previous && previous.source === groups) return previous.value;
      return this.current()?.projection.composition?.groups.find(g => g.role === 'work')?.id ?? '';
    },
  });
  selectTab(name: string): void {
    if (
      name === 'Burndown' &&
      !this.current()?.projection.composition?.groups.some(
        g => g.id === this.group() && g.role === 'work'
      )
    ) {
      this.group.set(
        this.current()?.projection.composition?.groups.find(g => g.role === 'work')?.id ?? ''
      );
    }
    this.tab.set(name);
  }
  readonly comparison = signal(-1);
  readonly previous = computed(
    () =>
      this.view.readings()[
        this.comparison() >= 0 && this.comparison() < this.view.selected()
          ? this.comparison()
          : this.view.selected() - 1
      ]
  );
  readonly tabs = computed(
    () => this.current()?.projection.composition?.views ?? ['Board', 'Burndown', 'Sources']
  );
  readonly board = computed(() =>
    this.current() ? collection(this.current(), this.group()) : undefined
  );
  readonly progressQuery = signal('');
  readonly focusSubjects = computed(
    () =>
      this.current()?.projection.subjects.filter(s =>
        this.current()?.projection.composition?.groups.some(
          g => g.role === 'focus' && field(s, '$group') === g.id
        )
      ) ?? []
  );
  readonly progressSubjects = computed(() => {
    const query = this.progressQuery().trim().toLowerCase();
    return this.focusSubjects().filter(subject =>
      `${field(subject, '$title')} ${subject.identity} ${subject.source}`
        .toLowerCase()
        .includes(query)
    );
  });
  readonly claims = claims;
  readonly strings = strings;
  readonly evidenceSummary = evidenceSummary;
  readonly remainingActions = remainingActions;
  change(subject: Subject): string {
    return movement(subject, this.current(), this.previous());
  }
  linked(subject: Subject): Subject[] {
    const links = this.current()?.projection.composition?.links ?? [];
    const ids = new Set(
      links.flatMap(l => {
        if (l.from === subject.identity && l.to) return [l.to];
        return l.to === subject.identity ? [l.from] : [];
      })
    );
    return this.current()?.projection.subjects.filter(s => ids.has(s.identity)) ?? [];
  }
  unresolved(subject: Subject): number {
    return (
      this.current()?.projection.composition?.links.filter(
        l => l.from === subject.identity && l.state !== 'resolved'
      ).length ?? 0
    );
  }
  compositionLinks(subject: Subject) {
    return (
      this.current()?.projection.composition?.links.filter(
        link => link.from === subject.identity || link.to === subject.identity
      ) ?? []
    );
  }
  readonly missing = computed(
    () =>
      this.previous()?.projection.subjects.filter(
        s => !this.current()?.projection.subjects.some(n => n.identity === s.identity)
      ) ?? []
  );
  readonly query = signal('');
  readonly status = signal('');
  readonly dense = signal(false);
  readonly baseline = signal(0);
  readonly current = computed(() => this.view.readings()[this.view.selected()]);
  readonly detail = linkedSignal({ source: this.current, computation: (): Subject | null => null });
  readonly sourcePath = linkedSignal({ source: this.current, computation: () => '' });
  readonly visible = computed(() => {
    const r = this.board();
    if (!r) return [];
    return r.projection.subjects.filter(
      s =>
        (!this.status() || field(s, r.presentation.statusField) === this.status()) &&
        `${field(s, r.presentation.titleField)} ${s.identity} ${s.source}`
          .toLowerCase()
          .includes(this.query().toLowerCase())
    );
  });
  readonly columns = computed(() => {
    const r = this.board();
    if (!r) return [];
    const declared = r.presentation.columns;
    const unmapped = [
      ...new Set(r.projection.subjects.map(s => field(s, r.presentation.statusField))),
    ].filter(v => !declared.some(c => c.value === v));
    return [
      ...declared,
      ...unmapped.map(value => ({ value, label: `Unmapped: ${value || 'missing status'}` })),
    ];
  });
  readonly points = computed(() => {
    const all = this.view.readings().map(r => collection(r, this.group()));
    return all.length ? burndown(all, all[this.baseline()] ?? all[0]) : [];
  });
  readonly sourcePaths = computed(() => Object.keys(this.current()?.projection.sources ?? {}));
  readonly sourceText = computed(() =>
    this.sourcePath() && this.current() ? this.view.source(this.current(), this.sourcePath()) : ''
  );
  readonly cohortSize = computed(() => {
    const r = this.view.readings()[this.baseline()];
    return r ? collection(r, this.group()).projection.subjects.length : 0;
  });
  readonly field = field;
  readonly acceptance = acceptance;
  readingState(subject: Subject, name: string): string {
    const reading = subject.readings[name];
    if (!reading || typeof reading !== 'object' || Array.isArray(reading)) return 'unavailable';
    return typeof reading['state'] === 'string' ? reading['state'] : 'unavailable';
  }
  ngOnInit(): void {
    void this.view.open(this.route.snapshot.paramMap.get('viewId') ?? '');
  }
  cards(value: string): Subject[] {
    return this.visible().filter(s => field(s, this.board()!.presentation.statusField) === value);
  }
  selectObservation(value: number): void {
    this.view.selected.set(Number(value));
    this.detail.set(null);
    this.sourcePath.set('');
  }
  showSource(path: string): void {
    this.sourcePath.set(path);
    this.tab.set('Sources');
  }
  y(value: number): number {
    return 180 - (150 * value) / Math.max(1, this.cohortSize());
  }
  x(index: number): number {
    return 45 + (index * 650) / Math.max(1, this.points().length - 1);
  }
}
