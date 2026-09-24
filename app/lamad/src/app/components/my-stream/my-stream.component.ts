import { DatePipe } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  OnInit,
  inject,
  signal,
} from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { RouterModule } from '@angular/router';

import { BehaviorSubject, catchError, map, of, switchMap } from 'rxjs';

import { eprToUniversalHref } from '@elohim/service';

import { LAMAD_STORAGE_CLIENT } from '../../interfaces/storage.interface';

import type {
  ObservationStreamEntryView,
  ObservationStreamView,
} from '@elohim/storage-client/generated';

/**
 * The lens table of the observation-lifestream recipe
 * (elohim/elohim-storage/.epr-meta/elohim/algorithms/observation-lifestream-recipe.json).
 * The node applies the lens; this list only names the choices.
 */
export const STREAM_LENSES = ['all', 'content', 'long-dwell'] as const;

type StreamState =
  | { status: 'loading' }
  | { status: 'ready'; view: ObservationStreamView }
  | { status: 'error' };

/**
 * MyStreamComponent — the person's own lifestream at /lamad/me/stream.
 *
 * Ruling R-A4: what they have looked at, witnessed as agent-private
 * observations on their own node, arranged by a governed recipe whose name
 * and content address print first. The page renders the recipe's order
 * (newest first, then longest dwell) and never re-ranks it; what the view
 * cannot show or vouch for prints as named omission lines. An empty stream
 * still prints the recipe it was read through.
 */
@Component({
  selector: 'app-my-stream',
  standalone: true,
  imports: [DatePipe, RouterModule],
  templateUrl: './my-stream.component.html',
  styleUrls: ['./my-stream.component.css'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MyStreamComponent implements OnInit {
  private readonly storageClient = inject(LAMAD_STORAGE_CLIENT);
  private readonly destroyRef = inject(DestroyRef);
  private readonly lens$ = new BehaviorSubject<string>('all');

  readonly lenses = STREAM_LENSES;
  readonly lens = signal<string>('all');
  readonly state = signal<StreamState>({ status: 'loading' });

  ngOnInit(): void {
    this.lens$
      .pipe(
        switchMap(lens =>
          this.storageClient.getObservationStream({ lens }).pipe(
            map((view): StreamState => ({ status: 'ready', view })),
            catchError(() => of<StreamState>({ status: 'error' }))
          )
        ),
        takeUntilDestroyed(this.destroyRef)
      )
      .subscribe(state => this.state.set(state));
  }

  onLensChange(event: Event): void {
    const lens = (event.target as HTMLSelectElement).value;
    if (lens === this.lens()) return;
    this.lens.set(lens);
    this.state.set({ status: 'loading' });
    this.lens$.next(lens);
  }

  /** Mint the universal EPR address for an entry's subject (never a literal). */
  eprHref(subjectCid: string | null): string | null {
    return subjectCid ? eprToUniversalHref({ id: subjectCid, tier: 'head' }) : null;
  }

  /** What the entry is about: its title, else its subject address. */
  label(entry: ObservationStreamEntryView): string {
    return entry.title ?? entry.subjectCid ?? 'No subject named';
  }

  /** Epoch seconds as an ISO instant for <time datetime>. */
  isoOf(epochSeconds: number): string {
    return new Date(epochSeconds * 1000).toISOString();
  }

  /** Dwell in words a person reads: "3.4 s", "42 s", "1 min 5 s". */
  formatDwell(ms: number): string {
    if (ms < 60_000) {
      const seconds = ms / 1000;
      return seconds < 10 ? `${seconds.toFixed(1)} s` : `${Math.round(seconds)} s`;
    }
    const minutes = Math.floor(ms / 60_000);
    const seconds = Math.round((ms % 60_000) / 1000);
    return seconds === 0 ? `${minutes} min` : `${minutes} min ${seconds} s`;
  }
}
