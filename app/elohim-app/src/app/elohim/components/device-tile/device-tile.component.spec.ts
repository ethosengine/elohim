import { ComponentFixture, TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import { DeviceTileComponent, DeviceSummary } from './device-tile.component';

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
      '[data-testid="device-tile-archetype-label"]',
    );
    expect(label?.textContent).toMatch(/home server/i);
  });

  it('renders the Laptop label for the desktop archetype', () => {
    fixture.componentInstance.device = mk({ archetype: 'desktop', displayName: 'jessica' });
    fixture.detectChanges();
    const label = fixture.nativeElement.querySelector(
      '[data-testid="device-tile-archetype-label"]',
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
});
