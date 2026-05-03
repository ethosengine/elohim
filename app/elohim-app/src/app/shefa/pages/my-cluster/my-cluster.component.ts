import { CommonModule } from '@angular/common';
import {
  ChangeDetectionStrategy,
  Component,
  OnDestroy,
  OnInit,
  inject,
  signal,
} from '@angular/core';

import { DeviceTileComponent } from '@app/elohim/components/device-tile/device-tile.component';

import { ClusterService } from '../../services/cluster.service';

import type { MyClusterView } from '@app/generated/my-cluster-view';

@Component({
  selector: 'app-shefa-my-cluster',
  standalone: true,
  imports: [CommonModule, DeviceTileComponent],
  templateUrl: './my-cluster.component.html',
  styleUrls: ['./my-cluster.component.scss'],
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MyClusterComponent implements OnInit, OnDestroy {
  protected readonly cluster = inject(ClusterService);
  protected readonly showDetails = signal(false);
  private stopPolling?: () => void;

  ngOnInit(): void {
    void this.cluster.getMyCluster();
  }

  ngOnDestroy(): void {
    this.stopPolling?.();
  }

  startPolling(intervalMs = 5000): void {
    this.stopPolling?.();
    this.stopPolling = this.cluster.startPolling(intervalMs);
  }

  toggleDetails(): void {
    this.showDetails.update(v => !v);
  }

  onlineCount(devices: MyClusterView['devices']): number {
    return devices.filter(d => d.online).length;
  }

  offlineCount(devices: MyClusterView['devices']): number {
    return devices.filter(d => !d.online).length;
  }

  formatBytes(n: number): string {
    if (n === 0) return '0 B';
    const gb = n / 1e9;
    if (gb >= 1) return `${gb.toFixed(1)} GB`;
    const mb = n / 1e6;
    if (mb >= 1) return `${mb.toFixed(1)} MB`;
    const kb = n / 1e3;
    return `${kb.toFixed(1)} KB`;
  }

  statusText(view: MyClusterView): string {
    return `${view.freshness.state}`;
  }

  reciprocityInBytes(net: number): number {
    return Math.abs(net);
  }
}
