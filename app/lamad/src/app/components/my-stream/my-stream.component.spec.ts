import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';
import { of, type Observable } from 'rxjs';
import { vi, type Mock } from 'vitest';

import type { ObservationStreamView } from '@elohim/storage-client/generated';

import {
  LAMAD_STORAGE_CLIENT,
  type LamadObservationStreamQuery,
} from '../../interfaces/storage.interface';

import { MyStreamComponent } from './my-stream.component';

const RECIPE_CID = 'blake3:4f1c0a9e7d2b6c85e3a1f0d9b8c7a6e5d4c3b2a1f0e9d8c7b6a5f4e3d2c1b0a9';

function streamView(overrides: Partial<ObservationStreamView> = {}): ObservationStreamView {
  return {
    asOf: 1_790_000_000,
    window: '7d',
    recipe: { name: 'observation-lifestream', cid: RECIPE_CID },
    lens: 'all',
    entries: [],
    omissions: [],
    totalCount: 0,
    ...overrides,
  };
}

describe('MyStreamComponent', () => {
  let fixture: ComponentFixture<MyStreamComponent>;
  let getObservationStream: Mock<
    (q: LamadObservationStreamQuery) => Observable<ObservationStreamView | null>
  >;

  const el = (): HTMLElement => fixture.nativeElement as HTMLElement;
  const all = (testid: string): HTMLElement[] =>
    Array.from(el().querySelectorAll<HTMLElement>(`[data-testid="${testid}"]`));
  const one = (testid: string): HTMLElement | null =>
    el().querySelector<HTMLElement>(`[data-testid="${testid}"]`);

  // `null` is the signed-out answer: the client asked the node nothing because
  // nobody is signed in (ruling R-A13).
  async function render(view: ObservationStreamView | null): Promise<void> {
    getObservationStream = vi.fn(() => of(view));
    await TestBed.configureTestingModule({
      imports: [MyStreamComponent],
      providers: [
        provideRouter([]),
        {
          provide: LAMAD_STORAGE_CLIENT,
          useValue: {
            getBlobUrl: vi.fn(),
            getStorageBaseUrl: vi.fn(),
            getContentEngagement: vi.fn(),
            getObservationStream,
          },
        },
      ],
    }).compileComponents();
    fixture = TestBed.createComponent(MyStreamComponent);
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();
  }

  it('renders_provenance_line_with_recipe_name_and_cid', async () => {
    await render(streamView());

    const provenance = one('stream-provenance');
    expect(provenance).not.toBeNull();
    expect(provenance!.textContent).toContain('observation-lifestream');
    expect(provenance!.textContent).toContain(RECIPE_CID);
    expect(provenance!.textContent).toContain('all');
    expect(getObservationStream).toHaveBeenCalledWith({ lens: 'all' });
  });

  it('lede_says_kept_on_your_node_not_only_you_can_read_it', async () => {
    // Ruling R-A10 (review W1): the node operator can read the rows and the log is
    // unsigned, so the page never promises "only you can read it".
    await render(streamView());

    const lede = el().querySelector('.stream-lede')!.textContent ?? '';
    expect(lede).toContain('kept on your node; not shared with other peers');
    expect(lede.toLowerCase()).not.toContain('only you');
  });

  it('renders_entries_newest_first_with_dwell_and_depth', async () => {
    // The node ranks (recency, then dwell); the page renders the recipe's order.
    await render(
      streamView({
        totalCount: 2,
        entries: [
          {
            observedAt: 1_789_999_900,
            kind: 'lamad:content-viewed',
            subjectCid: 'governance-epic',
            title: 'The Governance Epic',
            dwellMs: 65_000,
            scrollDepthPct: 80,
          },
          {
            observedAt: 1_789_990_000,
            kind: 'lamad:content-viewed',
            subjectCid: 'bafk-untitled',
            dwellMs: 3_400,
            scrollDepthPct: 100,
          },
        ],
      })
    );

    const entries = all('stream-entry');
    expect(entries).toHaveLength(2);
    expect(entries[0].textContent).toContain('The Governance Epic');
    expect(entries[0].textContent).toContain('1 min 5 s');
    expect(entries[0].textContent).toContain('80%');
    // An untitled subject prints its CID rather than nothing.
    expect(entries[1].textContent).toContain('bafk-untitled');
    expect(entries[1].textContent).toContain('3.4 s');
    expect(entries[1].textContent).toContain('100%');
    // observedAt is printed as a <time> with a machine-readable instant.
    const times = entries.map(e => e.querySelector('time')?.getAttribute('datetime'));
    expect(times[0]).toBe(new Date(1_789_999_900 * 1000).toISOString());
    expect(times[1]).toBe(new Date(1_789_990_000 * 1000).toISOString());
  });

  it('empty_stream_shows_honest_empty_state_with_provenance_still_printed', async () => {
    await render(streamView({ entries: [], totalCount: 0 }));

    expect(all('stream-entry')).toHaveLength(0);
    const empty = one('stream-empty');
    expect(empty).not.toBeNull();
    expect(empty!.textContent).toContain('7d');
    expect(one('stream-provenance')!.textContent).toContain(RECIPE_CID);
  });

  it('omissions_rendered', async () => {
    await render(
      streamView({
        omissions: [
          'signature: absent — observations are unsigned until the signing graduation',
          'window: 3 older observations are outside 7d',
        ],
      })
    );

    const omissions = all('stream-omission');
    expect(omissions).toHaveLength(2);
    expect(omissions[0].textContent).toContain('signature: absent');
    expect(omissions[1].textContent).toContain('3 older observations');
  });

  it('lens_select_refetches', async () => {
    await render(streamView());
    getObservationStream.mockReturnValue(of(streamView({ lens: 'long-dwell' })));

    const select = one('stream-lens') as HTMLSelectElement;
    expect(select).not.toBeNull();
    expect(Array.from(select.options).map(o => o.value)).toEqual(['all', 'content', 'long-dwell']);
    select.value = 'long-dwell';
    select.dispatchEvent(new Event('change'));
    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();

    expect(getObservationStream).toHaveBeenLastCalledWith({
      lens: 'long-dwell',
    });
    expect(getObservationStream).toHaveBeenCalledTimes(2);
    expect(one('stream-provenance')!.textContent).toContain('long-dwell');
  });

  // Ruling R-A13: a stream nobody is signed in for is not a broken node. The
  // page says so plainly and never dresses a signed-out read as a failure.
  it('signed_out_renders_the_honest_state_not_an_error_card', async () => {
    await render(null);

    const signedOut = one('stream-signed-out');
    expect(signedOut).not.toBeNull();
    expect(signedOut!.textContent).toContain('Sign in');
    expect(one('stream-error')).toBeNull();
    expect(one('stream-provenance')).toBeNull();
    expect(all('stream-entry')).toHaveLength(0);
  });
});
