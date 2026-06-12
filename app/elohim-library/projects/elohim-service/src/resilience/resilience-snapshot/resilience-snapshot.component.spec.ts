import { ComponentFixture, TestBed } from '@angular/core/testing';
import { ResilienceSnapshotComponent } from './resilience-snapshot.component';
import { ResilienceSnapshotView } from '../../generated/resilience-snapshot-view';

import type { ContextMenuItem } from 'elohim-core';

const sampleProtected: ResilienceSnapshotView = {
  contentId: 'c1',
  distributionState: 'measured',
  stewardingCollectives: 4,
  commitmentBackedCollectives: 4,
  diversityScore: 0.95,
  regionalDistribution: { local: 1, regional: 2, global: 1, unknown: 0 },
  placementGaps: [],
  protectionStatus: 'protected',
} as ResilienceSnapshotView;

const samplePartial: ResilienceSnapshotView = {
  ...sampleProtected,
  stewardingCollectives: 2,
  diversityScore: 0.5,
  protectionStatus: 'partial',
};

describe('ResilienceSnapshotComponent', () => {
  let fixture: ComponentFixture<ResilienceSnapshotComponent>;
  let component: ResilienceSnapshotComponent;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ResilienceSnapshotComponent],
    }).compileComponents();
    fixture = TestBed.createComponent(ResilienceSnapshotComponent);
    component = fixture.componentInstance;
  });

  const icon = (): HTMLButtonElement =>
    fixture.nativeElement.querySelector('[data-testid="resilience-icon"]');
  const panel = (): HTMLElement & { open?: boolean; actions?: ContextMenuItem[] } =>
    fixture.nativeElement.querySelector('[data-testid="resilience-hypercard"]');

  it('renders icon density with green indicator when protected', () => {
    component.snapshot = sampleProtected;
    component.density = 'icon';
    fixture.detectChanges();

    const el: HTMLElement = fixture.nativeElement;
    const iconEl = el.querySelector('[data-testid="resilience-icon"]');
    expect(iconEl?.classList.contains('status-protected')).toBe(true);
    const tooltip = el.querySelector('[data-testid="resilience-tooltip"]')?.textContent ?? '';
    expect(tooltip).toContain('4 collectives');
    expect(tooltip).toContain('protected');
  });

  it('renders yellow indicator when partial', () => {
    component.snapshot = samplePartial;
    component.density = 'icon';
    fixture.detectChanges();
    const iconEl = fixture.nativeElement.querySelector('[data-testid="resilience-icon"]');
    expect(iconEl?.classList.contains('status-partial')).toBe(true);
  });

  it('icon tooltip names the commitment-backed collective count', () => {
    // samplePartial: 2 stewarding vs 4 commitment-backed — distinct numbers so
    // the assertion can't pass off the stewarding segment by accident.
    component.snapshot = samplePartial;
    component.density = 'icon';
    fixture.detectChanges();
    const tooltip =
      fixture.nativeElement.querySelector('[data-testid="resilience-tooltip"]')?.textContent ?? '';
    expect(tooltip).toContain('2 collectives');
    expect(tooltip).toContain('4 commitment-backed');
  });

  it('icon tooltip names peers online when details carry a live peer count', () => {
    component.snapshot = {
      ...sampleProtected,
      details: { stewardingCollectives: [], onlinePeers: { live: 5, known: 8 } },
    };
    component.density = 'icon';
    fixture.detectChanges();
    const tooltip =
      fixture.nativeElement.querySelector('[data-testid="resilience-tooltip"]')?.textContent ?? '';
    expect(tooltip).toContain('5/8 peers live');
  });

  it('icon tooltip renders the unmeasured state instead of a fake at-risk verdict', () => {
    // Unmeasured ≠ zero (2026-06-12): content that never entered the
    // distribution plane must say so — never render at-risk counts.
    component.snapshot = {
      ...sampleProtected,
      distributionState: 'unmeasured',
      stewardingCollectives: 0,
      commitmentBackedCollectives: 0,
      diversityScore: 0,
      protectionStatus: 'at-risk',
    };
    component.density = 'icon';
    fixture.detectChanges();
    const iconEl = fixture.nativeElement.querySelector('[data-testid="resilience-icon"]');
    expect(iconEl?.classList.contains('status-unmeasured')).toBe(true);
    expect(iconEl?.classList.contains('status-at-risk')).toBe(false);
    const tooltip =
      fixture.nativeElement.querySelector('[data-testid="resilience-tooltip"]')?.textContent ?? '';
    expect(tooltip).toContain('not yet distributed');
    expect(tooltip).not.toContain('at-risk');
  });

  it('context density carries the unmeasured note', () => {
    component.snapshot = { ...sampleProtected, distributionState: 'unmeasured' };
    component.density = 'context';
    fixture.detectChanges();
    const note = fixture.nativeElement.querySelector(
      '[data-testid="resilience-unmeasured-note"]',
    );
    expect(note?.textContent).toContain('not entered the distribution plane');
  });

  it('icon tooltip stays silent about peers when details are absent', () => {
    // Honest signal: no liveness data → no liveness claim.
    component.snapshot = sampleProtected;
    component.density = 'icon';
    fixture.detectChanges();
    const tooltip =
      fixture.nativeElement.querySelector('[data-testid="resilience-tooltip"]')?.textContent ?? '';
    expect(tooltip).not.toContain('peers live');
  });

  it('context-menu density lists placement gap count', () => {
    const withGaps: ResilienceSnapshotView = {
      ...samplePartial,
      placementGaps: [
        {
          id: 'g1',
          contentId: 'c1',
          shardHash: 'h1',
          requestedStewardCount: 3,
          achievedStewardCount: 1,
          contractCoverage: 0.33,
          gapKind: 'peers-unavailable',
          firstSeenAt: '2026-04-19T00:00:00Z',
          lastSeenAt: '2026-04-19T00:00:00Z',
        },
      ],
    };
    component.snapshot = withGaps;
    component.density = 'context';
    fixture.detectChanges();
    const gapCount = fixture.nativeElement.querySelector('[data-testid="resilience-gap-count"]');
    expect(gapCount?.textContent).toContain('1');
  });

  it('full card lists steward collectives', () => {
    const withCollectives: ResilienceSnapshotView = {
      ...sampleProtected,
      details: {
        stewardingCollectives: [
          { id: 'home-alpha', kind: 'household', label: 'Alpha Household' },
          { id: 'home-beta', kind: 'household', label: 'Beta Household' },
          { id: 'home-gamma', kind: 'household', label: 'Gamma Household' },
          { id: 'home-delta', kind: 'household', label: 'Delta Household' },
        ],
        onlinePeers: { live: 4, known: 4 },
        healthScore: 0.95,
      },
    };
    component.snapshot = withCollectives;
    component.density = 'full';
    fixture.detectChanges();
    const rows = fixture.nativeElement.querySelectorAll('[data-testid^="resilience-collective-"]');
    expect(rows.length).toBe(4);
  });

  describe('icon hypercard panel', () => {
    beforeEach(() => {
      component.snapshot = sampleProtected;
      component.density = 'icon';
      fixture.detectChanges();
    });

    // The REAL <elohim-hypercard-panel> is registered via the component's
    // 'elohim-core/register' import, so these tests exercise the production
    // event path — shadow-DOM action clicks, Escape, the trigger's mousedown —
    // not synthetic CustomEvents. The synthetic shortcut previously hid an
    // auto-close race that made the in-place flip impossible.
    const panelUpdated = async (): Promise<void> => {
      await (panel() as unknown as { updateComplete: Promise<unknown> }).updateComplete;
    };
    const shadowAction = (id: string): HTMLButtonElement | null =>
      panel().shadowRoot?.querySelector<HTMLButtonElement>(`[data-action-id="${id}"]`) ?? null;

    it('renders the icon as a button advertising a dialog popup', () => {
      const btn = icon();
      expect(btn.tagName).toBe('BUTTON');
      expect(btn.getAttribute('aria-haspopup')).toBe('dialog');
      expect(btn.getAttribute('aria-expanded')).toBe('false');
    });

    it('opens the panel and flips aria-expanded on icon click', () => {
      icon().click();
      fixture.detectChanges();
      expect(component.panelOpen).toBe(true);
      expect(icon().getAttribute('aria-expanded')).toBe('true');
      expect(panel().open).toBe(true);
    });

    it('shows the context body in the panel before any flip', () => {
      icon().click();
      fixture.detectChanges();
      expect(panel().querySelector('.resilience-context-panel')).not.toBeNull();
      expect(panel().querySelector('.resilience-full-card')).toBeNull();
    });

    it('view-full action flips the body in place and the panel STAYS open', async () => {
      icon().click();
      fixture.detectChanges();
      await panelUpdated();
      shadowAction('view-full')!.click();
      fixture.detectChanges();
      expect(component.panelDensity).toBe('full');
      // The flip survives — selection is intent, not dismissal. An auto-close
      // in the primitive would reset density to 'context' in the same batch.
      expect(component.panelOpen).toBe(true);
      expect(panel().open).toBe(true);
      expect(panel().querySelector('.resilience-full-card')).not.toBeNull();
      expect(panel().querySelector('.resilience-context-panel')).toBeNull();
    });

    it('drops the built-in view-full action once flipped to full', async () => {
      icon().click();
      fixture.detectChanges();
      await panelUpdated();
      expect(shadowAction('view-full')).not.toBeNull();
      shadowAction('view-full')!.click();
      fixture.detectChanges();
      await panelUpdated();
      expect(component.panelActions.some(a => a.id === 'view-full')).toBe(false);
      expect(shadowAction('view-full')).toBeNull();
    });

    it('re-emits every selection id including view-full', async () => {
      const seen: string[] = [];
      component.actionSelect.subscribe(e => seen.push(e.id));
      component.actions = [{ id: 'steward-this', label: 'Steward this content' }];
      icon().click();
      fixture.detectChanges();
      await panelUpdated();
      shadowAction('view-full')!.click();
      fixture.detectChanges();
      await panelUpdated();
      shadowAction('steward-this')!.click();
      expect(seen).toEqual(['view-full', 'steward-this']);
    });

    it('Escape closes the panel and resets it to context density', async () => {
      icon().click();
      fixture.detectChanges();
      await panelUpdated();
      shadowAction('view-full')!.click();
      fixture.detectChanges();
      await panelUpdated();
      expect(component.panelDensity).toBe('full');

      const dialog = panel().shadowRoot!.querySelector('[role="dialog"]') as HTMLElement;
      dialog.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      fixture.detectChanges();
      expect(component.panelOpen).toBe(false);
      expect(component.panelDensity).toBe('context');
      expect(icon().getAttribute('aria-expanded')).toBe('false');
    });

    it('clicking the icon again closes the panel — trigger mousedown must not read as click-outside', async () => {
      // Regression: the panel's click-outside handler fires on document
      // mousedown; the icon is a SIBLING of the panel, so without
      // stopPropagation on the trigger, the icon's own mousedown closed the
      // panel and the subsequent click toggled it straight back open.
      icon().click();
      fixture.detectChanges();
      await panelUpdated();
      expect(component.panelOpen).toBe(true);

      // The real browser sequence on the trigger: mousedown, then click.
      icon().dispatchEvent(new MouseEvent('mousedown', { bubbles: true, composed: true }));
      icon().click();
      fixture.detectChanges();
      expect(component.panelOpen).toBe(false);
      expect(panel().open).toBe(false);
    });

    it('appends host-injected actions after the built-in', () => {
      const hostActions: ContextMenuItem[] = [
        { id: 'steward-this', label: 'Steward this content' },
        { id: 'view-network', label: 'View network' },
      ];
      component.actions = hostActions;
      // Opening goes through Angular's event binding, which dirties the OnPush
      // view so the [actions] binding re-evaluates on the next CD.
      icon().click();
      fixture.detectChanges();
      expect(component.panelActions.map(a => a.id)).toEqual([
        'view-full',
        'steward-this',
        'view-network',
      ]);
      // The panel element receives the composed list via the [actions] binding.
      expect((panel().actions ?? []).map(a => a.id)).toEqual([
        'view-full',
        'steward-this',
        'view-network',
      ]);
    });
  });
  // -------------------------------------------------------------------------
  // panelAlign — inline-axis pin passthrough (2026-06-12 phone regression:
  // the panel pinned inset-inline-start:0 to the icon wrap in end-side omni
  // chrome and projected ~173px off a 390px viewport). The HOST knows where
  // its chrome sits; this component forwards that knowledge to the panel's
  // align attribute and mirrors it as a wrap class so the hover tooltip flips
  // the same way.
  // -------------------------------------------------------------------------

  describe('panelAlign passthrough', () => {
    it('defaults to start — panel align attribute is start, wrap has no align-end class', () => {
      component.snapshot = sampleProtected;
      component.density = 'icon';
      fixture.detectChanges();
      expect(panel().getAttribute('align')).toBe('start');
      const wrap = fixture.nativeElement.querySelector('.resilience-icon-wrap');
      expect(wrap?.classList.contains('align-end')).toBe(false);
    });

    it('panelAlign="end" forwards align="end" to the hypercard panel', () => {
      component.snapshot = sampleProtected;
      component.density = 'icon';
      component.panelAlign = 'end';
      fixture.detectChanges();
      expect(panel().getAttribute('align')).toBe('end');
    });

    it('panelAlign="end" marks the wrap with align-end so the tooltip flips too', () => {
      component.snapshot = sampleProtected;
      component.density = 'icon';
      component.panelAlign = 'end';
      fixture.detectChanges();
      const wrap = fixture.nativeElement.querySelector('.resilience-icon-wrap');
      expect(wrap?.classList.contains('align-end')).toBe(true);
    });
  });
});
