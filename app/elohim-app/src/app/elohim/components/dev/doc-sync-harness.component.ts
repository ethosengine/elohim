/**
 * Doc-Sync Harness (dev surface)
 *
 * A minimal render target for the Automerge content-sync plane. It exists to
 * (a) UN-TREE-SHAKE the {@link ContentDocSyncService} by giving it a real
 * consumer, and (b) serve as the look-rail proof surface: render
 * `/dev/doc-sync?id=<contentId>` and assert the converged `node:{id}` doc's
 * fields appear (data-testids below), proving the full browser leg —
 * SDK → `/sync/v1/elohim/docs/...` (doorway-carried) → Automerge → signal → DOM.
 *
 * SERVICE GRAVITY: pure UX surface. It shows what the sync plane already
 * converged; it computes no truth. Hidden, debug-only, reads only the
 * already-public `/sync` surface — no guard needed.
 */

import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Signal, inject, signal } from '@angular/core';
import { ActivatedRoute } from '@angular/router';

import { ContentDocSyncService, type ContentDocFields } from '@elohim/service';

@Component({
  selector: 'app-doc-sync-harness',
  standalone: true,
  imports: [CommonModule],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <section class="doc-sync-harness" data-testid="docsync-harness">
      <h1>Content doc-sync harness</h1>
      <p>
        Content id: <code data-testid="docsync-content-id">{{ contentId || '(none)' }}</code>
      </p>

      <p>
        Status:
        <strong data-testid="docsync-status">{{ status }}</strong>
      </p>

      <dl *ngIf="doc() as d; else pending">
        <dt>Title</dt>
        <dd data-testid="docsync-title">{{ d.title || '(no title)' }}</dd>
        <dt>Reach</dt>
        <dd data-testid="docsync-reach">{{ d.reach || '(no reach)' }}</dd>
        <dt>Content type</dt>
        <dd data-testid="docsync-content-type">{{ d.contentType || '(none)' }}</dd>
        <dt>Content format</dt>
        <dd data-testid="docsync-content-format">{{ d.contentFormat || '(none)' }}</dd>
        <dt>Updated at</dt>
        <dd data-testid="docsync-updated-at">{{ d.updatedAt || '(none)' }}</dd>
        <dt>Body</dt>
        <dd data-testid="docsync-body">{{ d.body || '(no body)' }}</dd>
      </dl>

      <ng-template #pending>
        <p data-testid="docsync-pending">No doc yet — pending convergence (node:{{ contentId }}).</p>
      </ng-template>
    </section>
  `,
})
export class DocSyncHarnessComponent {
  private readonly route = inject(ActivatedRoute);
  private readonly docSync = inject(ContentDocSyncService);

  /** Content id from the `?id=` query param. */
  readonly contentId = this.route.snapshot.queryParamMap.get('id') ?? '';

  /**
   * The reactive doc-fields signal for the requested content id. Watching is
   * idempotent and self-managing in the service; an empty id yields a static
   * null signal (nothing to watch).
   */
  readonly doc: Signal<ContentDocFields | null> = this.contentId
    ? this.docSync.watchContent(this.contentId)
    : signal<ContentDocFields | null>(null);

  /** Honest per-content reading: a present doc is "synced"; null is "pending". */
  get status(): string {
    return this.doc() ? 'synced' : 'pending';
  }
}
