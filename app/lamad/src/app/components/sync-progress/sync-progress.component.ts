import { CommonModule } from '@angular/common';
import {
  Component,
  OnDestroy,
  OnInit,
  computed,
  inject,
  signal,
  ChangeDetectionStrategy,
} from '@angular/core';
import { Subscription, timer } from 'rxjs';
import { map, switchMap, takeWhile } from 'rxjs/operators';

import { SyncStatusService, deriveSyncProgress } from '../../services/sync-status.service';
import type { SyncProgress } from '../../services/sync-status.service';

/**
 * Learner-facing sync strip: replaces the opaque "syncing" spinner with an honest
 * have/total bar over live `/p2p/status`. Non-blocking (content is never trapped
 * behind it) and self-hiding once the node reports caught up. A sensing surface —
 * it renders backend-authoritative state, it never mutates.
 */
@Component({
  selector: 'app-sync-progress',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './sync-progress.component.html',
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrl: './sync-progress.component.css',
})
export class SyncProgressComponent implements OnInit, OnDestroy {
  private readonly svc = inject(SyncStatusService);

  private readonly progress = signal<SyncProgress | null>(null);

  /** Shown only while there is something worth reporting — hidden once caught up. */
  readonly display = computed<SyncProgress | null>(() => {
    const p = this.progress();
    return p && p.phase !== 'ready' ? p : null;
  });

  private sub?: Subscription;

  ngOnInit(): void {
    // Poll every 4s; stop once caught up (a route remount re-arms it). 'unreachable'
    // keeps retrying so a recovering backend heals the strip on its own.
    this.sub = timer(0, 4000)
      .pipe(
        switchMap(() => this.svc.fetch()),
        map(deriveSyncProgress),
        takeWhile(p => p.phase !== 'ready', true)
      )
      .subscribe(p => this.progress.set(p));
  }

  ngOnDestroy(): void {
    this.sub?.unsubscribe();
  }
}
