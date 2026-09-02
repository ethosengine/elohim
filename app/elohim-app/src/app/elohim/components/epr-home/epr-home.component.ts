import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { ActivatedRoute, RouterModule } from '@angular/router';

import { catchError, distinctUntilChanged, map, of, switchMap } from 'rxjs';

import { eprToUniversalHref } from '@elohim/service';

import { StorageClientService } from '../../services/storage-client.service';
import { EprFocalComponent } from '../epr-focal/epr-focal.component';

import {
  EprHomeAtom,
  anchorWords,
  dayWords,
  reachSubtitle,
  shortAnchor,
  toAtom,
} from './epr-home.model';

type LoadState =
  | { status: 'loading'; id: string }
  | { status: 'loaded'; id: string; atom: EprHomeAtom }
  | { status: 'not-found'; id: string }
  | { status: 'error'; id: string };

/**
 * EprHomeComponent — the shell-owned universal address (/epr/{id}).
 *
 * One frame for every atom: arrival · identity · focal render shaped by format
 * · the four legs · the address line. Reads the RAW ContentView for the frame
 * and hands only the slug to <app-epr-focal>; imports nothing from the lamad
 * bundle. An unreachable atom renders the designed gate (spec §2.5) — never a
 * wall, never the controls of a thing that cannot be seen.
 */
@Component({
  selector: 'app-epr-home',
  standalone: true,
  imports: [CommonModule, RouterModule, EprFocalComponent],
  changeDetection: ChangeDetectionStrategy.OnPush,
  templateUrl: './epr-home.component.html',
  styleUrl: './epr-home.component.css',
})
export class EprHomeComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly storage = inject(StorageClientService);

  readonly resourceId = toSignal(
    this.route.paramMap.pipe(
      map(p => p.get('resourceId') ?? ''),
      distinctUntilChanged()
    ),
    { initialValue: '' }
  );

  private readonly state = toSignal(
    this.route.paramMap.pipe(
      map(p => p.get('resourceId') ?? ''),
      distinctUntilChanged(),
      switchMap(id =>
        this.storage.getContent(id).pipe(
          map(
            (raw): LoadState =>
              raw === null
                ? { status: 'not-found', id }
                : { status: 'loaded', id, atom: toAtom(raw as unknown as Record<string, unknown>) }
          ),
          catchError(() => of<LoadState>({ status: 'error', id }))
        )
      )
    ),
    { initialValue: { status: 'loading', id: '' } as LoadState }
  );

  readonly status = computed(() => this.state().status);
  readonly atom = computed<EprHomeAtom | null>(() => {
    const s = this.state();
    return s.status === 'loaded' ? s.atom : null;
  });

  readonly reachLabel = computed(() => {
    const r = this.atom()?.reach ?? 'commons';
    return r.charAt(0).toUpperCase() + r.slice(1);
  });
  readonly reachSub = computed(() => reachSubtitle(this.atom()?.reach ?? 'commons'));
  readonly notarizedLabel = computed(() =>
    this.atom()?.trust === 'notarized' ? 'Notarized' : 'Not yet notarized'
  );
  readonly anchorSub = computed(() => {
    const a = this.atom();
    return a ? anchorWords(a.trust, a.dhtAnchorState) : '';
  });
  readonly eyebrow = computed<string[]>(() => {
    const a = this.atom();
    if (!a) return [];
    return [
      a.category ?? a.contentType,
      a.estimatedTime,
      a.author ? `by ${a.author}` : null,
      a.license,
    ].filter((x): x is string => !!x);
  });
  readonly address = computed(() => eprToUniversalHref({ id: this.resourceId(), tier: 'head' }));
  readonly rawHref = computed(() =>
    eprToUniversalHref({ id: this.resourceId(), tier: 'head', subview: 'raw' })
  );
  readonly anchorShort = computed(() => {
    const h = this.atom()?.dhtAnchorHash;
    return h ? shortAnchor(h) : null;
  });
  readonly addedOn = computed(() => dayWords(this.atom()?.createdAt ?? ''));
  readonly updatedOn = computed(() => dayWords(this.atom()?.updatedAt ?? ''));
}
