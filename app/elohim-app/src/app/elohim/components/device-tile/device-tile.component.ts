import { CommonModule } from '@angular/common';
import { ChangeDetectionStrategy, Component, Input } from '@angular/core';

import type { MyClusterView } from '@app/generated/my-cluster-view';
import type { ComputeTriptychGql } from '@elohim/storage-client/graphql';

export type DeviceSummary = MyClusterView['devices'][number] & {
  /** Compute capacity triptych. Present when the GraphQL path is active; absent on the REST path. */
  compute?: ComputeTriptychGql | null;
};
export type DeviceArchetype = MyClusterView['devices'][number]['archetype'];

const ARCHETYPE_LABEL: Record<DeviceArchetype, string> = {
  node: 'Home server',
  desktop: 'Laptop',
  mobile: 'Phone',
  steward: 'Steward process',
};

@Component({
  selector: 'app-device-tile',
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
        {{ archetypeLabel(device.archetype)
        }}{{ device.displayName ? ' (' + device.displayName + ')' : '' }}
      </span>
      @if (device.hostingCount != null) {
        <span class="hosting" data-testid="device-tile-hosting">
          {{ device.hostingCount }} files
        </span>
      }
      <span class="status" data-testid="device-tile-status">
        @if (device.online) {
          online
        } @else {
          asleep · {{ staleAgo(device.freshness.staleSinceMs) }}
        }
      </span>
      @if (device.compute) {
        <section class="compute-triptych" aria-label="Compute breakdown">
          <h4 class="compute-triptych__title">Compute</h4>
          <dl class="compute-triptych__values">
            <div class="compute-triptych__cell">
              <dt>Free</dt>
              <dd data-testid="compute-free-bytes">{{ formatBytes(device.compute.free) }}</dd>
            </div>
            <div class="compute-triptych__cell">
              <dt>Used</dt>
              <dd data-testid="compute-used-bytes">{{ formatBytes(device.compute.used) }}</dd>
            </div>
            <div class="compute-triptych__cell">
              <dt>Stewarded</dt>
              <dd data-testid="compute-stewarded-bytes">
                {{ formatBytes(device.compute.stewarded) }}
              </dd>
            </div>
          </dl>
        </section>
      }
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
    if (ms === undefined) return '';
    const sec = Math.floor(ms / 1000);
    if (sec < 60) return `${sec}s ago`;
    const min = Math.floor(sec / 60);
    if (min < 60) return `${min} min ago`;
    return `${Math.floor(min / 60)} h ago`;
  }

  formatBytes(value: string | null | undefined): string {
    if (!value) return '—';
    const n = Number(value);
    if (!Number.isFinite(n)) return '—';
    if (n >= 1e9) return `${(n / 1e9).toFixed(1)} GB`;
    if (n >= 1e6) return `${(n / 1e6).toFixed(1)} MB`;
    if (n >= 1e3) return `${(n / 1e3).toFixed(1)} KB`;
    return `${n} B`;
  }
}
