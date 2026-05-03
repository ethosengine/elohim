import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';

import type { MyClusterView } from '@app/generated/my-cluster-view';

export type DeviceSummary = MyClusterView['devices'][number];
export type DeviceArchetype = DeviceSummary['archetype'];

const ARCHETYPE_LABEL: Record<DeviceArchetype, string> = {
  node: 'Home server',
  desktop: 'Laptop',
  mobile: 'Phone',
  steward: 'Steward process',
};

@Component({
  selector: 'elohim-device-tile',
  standalone: true,
  imports: [CommonModule],
  template: `
    <div
      class="tile"
      [class.online]="device.online"
      [class.offline]="!device.online"
      data-testid="device-tile"
    >
      <span class="dot" data-testid="device-tile-dot"></span>
      <span class="label" data-testid="device-tile-archetype-label">
        {{ archetypeLabel(device.archetype) }}{{ device.displayName ? ' (' + device.displayName + ')' : '' }}
      </span>
      @if (device.hostingCount != null) {
        <span class="hosting" data-testid="device-tile-hosting">{{ device.hostingCount }} files</span>
      }
      <span class="status" data-testid="device-tile-status">
        @if (device.online) {
          online
        } @else {
          asleep · {{ staleAgo(device.freshness.staleSinceMs) }}
        }
      </span>
    </div>
  `,
  styleUrls: ['./device-tile.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DeviceTileComponent {
  @Input({ required: true }) device!: DeviceSummary;

  archetypeLabel(a: DeviceArchetype): string {
    return ARCHETYPE_LABEL[a];
  }

  staleAgo(ms?: number): string {
    if (ms == null) return '';
    const sec = Math.floor(ms / 1000);
    if (sec < 60) return `${sec}s ago`;
    const min = Math.floor(sec / 60);
    if (min < 60) return `${min} min ago`;
    return `${Math.floor(min / 60)} h ago`;
  }
}
