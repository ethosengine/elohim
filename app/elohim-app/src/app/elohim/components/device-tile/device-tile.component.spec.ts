import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import { DeviceTileComponent, DeviceSummary } from './device-tile.component';
import type { ComputeTriptychGql } from '@elohim/storage-client/graphql';

function mk(p: Partial<DeviceSummary> = {}): DeviceSummary {
  return {
    peerId: 'P',
    archetype: 'node',
    online: true,
    freshness: { state: 'live' },
    ...p,
  } as DeviceSummary;
}

describe('DeviceTileComponent', () => {
  let fixture: ComponentFixture<DeviceTileComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({ imports: [DeviceTileComponent] }).compileComponents();
    fixture = TestBed.createComponent(DeviceTileComponent);
  });

  it('renders the Home server label for the node archetype', () => {
    fixture.componentInstance.device = mk({ archetype: 'node', displayName: 'matthew' });
    fixture.detectChanges();
    const label = fixture.nativeElement.querySelector(
      '[data-testid="device-tile-archetype-label"]'
    );
    expect(label?.textContent).toMatch(/home server/i);
  });

  it('renders the Laptop label for the desktop archetype', () => {
    fixture.componentInstance.device = mk({ archetype: 'desktop', displayName: 'jessica' });
    fixture.detectChanges();
    const label = fixture.nativeElement.querySelector(
      '[data-testid="device-tile-archetype-label"]'
    );
    expect(label?.textContent).toMatch(/laptop/i);
  });

  it('shows asleep state for offline devices', () => {
    fixture.componentInstance.device = mk({
      online: false,
      freshness: { state: 'offline', staleSinceMs: 240000 },
    });
    fixture.detectChanges();
    const status = fixture.nativeElement.querySelector('[data-testid="device-tile-status"]');
    expect(status?.textContent).toMatch(/asleep/i);
  });

  it('marks the tile offline when not online', () => {
    fixture.componentInstance.device = mk({
      online: false,
      freshness: { state: 'offline', staleSinceMs: 60000 },
    });
    fixture.detectChanges();
    const tile = fixture.nativeElement.querySelector('[data-testid="device-tile"]');
    expect(tile?.classList.contains('offline')).toBe(true);
  });

  it('renders the compute triptych when compute is present', () => {
    const compute: ComputeTriptychGql = {
      free: '9500000',
      used: '500000',
      stewarded: '250000',
    };
    fixture.componentInstance.device = mk({ compute } as any);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="compute-free-bytes"]')?.textContent?.trim()).toContain(
      '9.5'
    );
    expect(el.querySelector('[data-testid="compute-used-bytes"]')?.textContent?.trim()).toContain(
      '500'
    );
    expect(
      el.querySelector('[data-testid="compute-stewarded-bytes"]')?.textContent?.trim()
    ).toContain('250');
  });

  it('hides the compute triptych when compute is null', () => {
    fixture.componentInstance.device = mk({ compute: null } as any);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('.compute-triptych')).toBeNull();
  });

  it('exposes the tile as a keyboard-operable button (role + tabindex + aria-expanded)', () => {
    fixture.componentInstance.device = mk();
    fixture.detectChanges();
    const tile = fixture.nativeElement.querySelector('[data-testid="device-tile"]');
    expect(tile?.getAttribute('role')).toBe('button');
    expect(tile?.getAttribute('tabindex')).toBe('0');
    expect(tile?.getAttribute('aria-expanded')).toBe('false');
  });

  it('expands and collapses the detail panel on click, flipping aria-expanded', () => {
    fixture.componentInstance.device = mk({
      peerId: 'peer-xyz',
      hostingCount: 12,
      projectingCount: 3,
    });
    fixture.detectChanges();
    const tile = fixture.nativeElement.querySelector('[data-testid="device-tile"]');
    expect(
      fixture.nativeElement.querySelector('[data-testid="device-tile-detail-panel"]')
    ).toBeNull();
    tile.click();
    fixture.detectChanges();
    const panel = fixture.nativeElement.querySelector('[data-testid="device-tile-detail-panel"]');
    expect(panel).toBeTruthy();
    expect(tile.getAttribute('aria-expanded')).toBe('true');
    expect(
      fixture.nativeElement.querySelector('[data-testid="device-tile-detail-peer-id"]')?.textContent
    ).toContain('peer-xyz');
    expect(
      fixture.nativeElement.querySelector('[data-testid="device-tile-detail-hosting"]')?.textContent
    ).toContain('12');
    tile.click();
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="device-tile-detail-panel"]')
    ).toBeNull();
  });

  it('expands the detail panel on Space keydown', () => {
    fixture.componentInstance.device = mk({ peerId: 'peer-kbd' });
    fixture.detectChanges();
    const tile = fixture.nativeElement.querySelector('[data-testid="device-tile"]');
    tile.dispatchEvent(new KeyboardEvent('keydown', { key: ' ', bubbles: true }));
    fixture.detectChanges();
    expect(
      fixture.nativeElement.querySelector('[data-testid="device-tile-detail-panel"]')
    ).toBeTruthy();
  });
});
