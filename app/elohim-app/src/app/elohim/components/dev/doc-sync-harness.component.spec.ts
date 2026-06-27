import { ActivatedRoute } from '@angular/router';
import { TestBed } from '@angular/core/testing';

import { vi } from 'vitest';

import { AUTOMERGE_SYNC_FACTORY, ContentDocSyncService } from '@elohim/service';

import { DocSyncHarnessComponent } from './doc-sync-harness.component';

/**
 * Deterministic browser-wiring proof: with a MOCKED AUTOMERGE_SYNC_FACTORY (no
 * automerge, no network), the harness renders the converged doc's title. This
 * is the unit-level twin of the live look-rail run (which the orchestrator does
 * separately). It proves DI → service → signal → DOM is wired end-to-end.
 */
function fakeSyncReturning(doc: Record<string, unknown>) {
  return {
    load: vi.fn().mockResolvedValue({}),
    sync: vi.fn().mockResolvedValue({ doc, changed: true, heads: ['h1'] }),
  };
}

function activatedRouteWithId(id: string | null): Partial<ActivatedRoute> {
  return {
    snapshot: {
      queryParamMap: { get: (k: string) => (k === 'id' ? id : null) },
    },
  } as unknown as ActivatedRoute;
}

describe('DocSyncHarnessComponent', () => {
  it('renders the converged doc title for the ?id query param', async () => {
    const fake = fakeSyncReturning({
      id: 'manifesto-foundations',
      title: 'Foundations of the Protocol',
      reach: 'commons',
      body: '# Hello',
      updatedAt: '2026-06-27T00:00:00Z',
    });

    TestBed.configureTestingModule({
      providers: [
        { provide: ActivatedRoute, useValue: activatedRouteWithId('manifesto-foundations') },
        { provide: AUTOMERGE_SYNC_FACTORY, useValue: () => fake },
      ],
    });

    const fixture = TestBed.createComponent(DocSyncHarnessComponent);
    fixture.detectChanges(); // watchContent kicks the immediate (async) refresh

    // Ensure the immediate sync round has resolved before asserting the DOM.
    await TestBed.inject(ContentDocSyncService).refreshContent('manifesto-foundations');
    fixture.detectChanges();

    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('[data-testid="docsync-content-id"]')?.textContent).toContain(
      'manifesto-foundations'
    );
    expect(el.querySelector('[data-testid="docsync-status"]')?.textContent).toContain('synced');
    expect(el.querySelector('[data-testid="docsync-title"]')?.textContent).toContain(
      'Foundations of the Protocol'
    );
    expect(el.querySelector('[data-testid="docsync-reach"]')?.textContent).toContain('commons');
    expect(el.querySelector('[data-testid="docsync-body"]')?.textContent).toContain('# Hello');
  });

  it('shows the pending state when no id query param is present', () => {
    TestBed.configureTestingModule({
      providers: [
        { provide: ActivatedRoute, useValue: activatedRouteWithId(null) },
        { provide: AUTOMERGE_SYNC_FACTORY, useValue: () => fakeSyncReturning({}) },
      ],
    });

    const fixture = TestBed.createComponent(DocSyncHarnessComponent);
    fixture.detectChanges();

    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('[data-testid="docsync-status"]')?.textContent).toContain('pending');
    expect(el.querySelector('[data-testid="docsync-pending"]')).toBeTruthy();
  });
});
