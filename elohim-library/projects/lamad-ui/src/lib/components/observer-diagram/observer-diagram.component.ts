/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { CommonModule } from '@angular/common';
import { Component, OnDestroy, OnInit } from '@angular/core';

@Component({
  selector: 'lamad-observer-diagram',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './observer-diagram.component.html',
  styleUrls: ['./observer-diagram.component.css'],
})
export class ObserverDiagramComponent implements OnInit, OnDestroy {
  mode: 'witness' | 'private' = 'witness';
  dataStream: number[] = [];
  private intervalId: ReturnType<typeof setInterval> | undefined;

  ngOnInit() {
    this.startDataStream();
  }

  ngOnDestroy() {
    this.stopDataStream();
  }

  toggleMode() {
    this.mode = this.mode === 'witness' ? 'private' : 'witness';
    if (this.mode === 'witness') {
      this.startDataStream();
    } else {
      this.stopDataStream();
      this.dataStream = [];
    }
  }

  private startDataStream() {
    this.stopDataStream();
    this.intervalId = setInterval(() => {
      this.dataStream = [...this.dataStream, Date.now()].slice(-5);
    }, 800);
  }

  private stopDataStream() {
    if (this.intervalId) {
      clearInterval(this.intervalId);
    }
  }
}
