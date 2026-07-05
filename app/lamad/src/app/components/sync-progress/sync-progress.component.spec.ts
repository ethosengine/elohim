import { TestBed, fakeAsync, tick } from '@angular/core/testing';
import { of } from 'rxjs';

import { SyncProgressComponent } from './sync-progress.component';
import { SyncStatusService } from '../../services/sync-status.service';

import type { P2PStatusView } from '../../../generated/p2p-status-view';

function p2p(over: Partial<P2PStatusView>): P2PStatusView {
  return {
    peerId: 't',
    listenAddresses: [],
    connectedPeers: 0,
    bootstrapNodes: [],
    syncDocuments: 0,
    natStatus: 'unknown',
    relayReservations: 0,
    announceAddresses: [],
    relayMode: 'disabled',
    replication: { pending: 0, completed: 0, failed: 0, caughtUp: false },
    syncPaused: false,
    dedupUniqueLen: 0,
    dedupTotalSeen: 0,
    reconcilePassesTotal: 0,
    kicksFiredTotal: 0,
    placementGapsEmittedTotal: 0,
    ...over,
  } as P2PStatusView;
}

function mount(status: P2PStatusView | null) {
  TestBed.configureTestingModule({
    imports: [SyncProgressComponent],
    providers: [{ provide: SyncStatusService, useValue: { fetch: () => of(status) } }],
  });
  const fixture = TestBed.createComponent(SyncProgressComponent);
  fixture.detectChanges(); // ngOnInit schedules timer(0)
  tick(0); // fire the first poll → fetch → progress signal set
  fixture.detectChanges();
  return fixture;
}

describe('SyncProgressComponent (render)', () => {
  afterEach(() => TestBed.resetTestingModule());

  it('renders an honest have/total strip while syncing', fakeAsync(() => {
    const fixture = mount(
      p2p({ connectedPeers: 3, pull: { total: 210, fetched: 142, pending: 60, failed: 8, caughtUp: false } })
    );
    const el = fixture.nativeElement as HTMLElement;

    expect(el.querySelector('[data-testid="lamad-sync-progress"]')).toBeTruthy();
    expect(el.querySelector('[data-testid="lamad-sync-progress-label"]')?.textContent).toContain('142/210');
    const fill = el.querySelector('[data-testid="lamad-sync-progress-fill"]') as HTMLElement;
    expect(fill.style.width).toBe('68%');

    fixture.destroy(); // unsubscribe the poll → clears the pending interval for fakeAsync
  }));

  it('self-hides the strip once caught up (no false "done" clutter)', fakeAsync(() => {
    const fixture = mount(
      p2p({ connectedPeers: 2, pull: { total: 10, fetched: 10, pending: 0, failed: 0, caughtUp: true } })
    );
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="lamad-sync-progress"]')).toBeNull();
    fixture.destroy();
  }));

  it('shows an honest unreachable state (never fake-done) when status is null', fakeAsync(() => {
    const fixture = mount(null);
    const strip = (fixture.nativeElement as HTMLElement).querySelector('[data-testid="lamad-sync-progress"]');
    expect(strip).toBeTruthy();
    expect(strip?.classList.contains('sync-progress--unreachable')).toBe(true);
    fixture.destroy();
  }));
});
