/**
 * Network Health Tab Component
 *
 * Renders P2P mesh posture metrics pulled from GET /api/v1/network/posture.
 * Gracefully degrades to an "unavailable" state when the endpoint has not
 * yet been deployed (F2 sprint task pending).
 */

import { CommonModule } from '@angular/common';
import { Component, OnInit, OnDestroy, inject, signal } from '@angular/core';

import { takeUntil } from 'rxjs/operators';

import { Subject } from 'rxjs';

import {
  NetworkPostureService,
  type NetworkPostureView,
} from '../../services/network-posture.service';

@Component({
  selector: 'app-network-health-tab',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './network-health-tab.component.html',
  styleUrl: './network-health-tab.component.scss',
})
export class NetworkHealthTabComponent implements OnInit, OnDestroy {
  private readonly networkPosture = inject(NetworkPostureService);
  private readonly destroy$ = new Subject<void>();

  readonly posture = signal<NetworkPostureView | null>(null);
  readonly loading = signal(true);

  ngOnInit(): void {
    this.networkPosture
      .get()
      .pipe(takeUntil(this.destroy$))
      .subscribe(data => {
        this.posture.set(data);
        this.loading.set(false);
      });
  }

  ngOnDestroy(): void {
    this.destroy$.next();
    this.destroy$.complete();
  }

  /** Format storagePressure (0..1) as a percentage string */
  formatPressure(pressure: number): string {
    return `${Math.round(pressure * 100)}%`;
  }

  /** CSS class for storage pressure bar fill — matches dashboard health palette */
  pressureClass(pressure: number): string {
    if (pressure < 0.6) return 'pressure-ok';
    if (pressure < 0.85) return 'pressure-warning';
    return 'pressure-critical';
  }
}
