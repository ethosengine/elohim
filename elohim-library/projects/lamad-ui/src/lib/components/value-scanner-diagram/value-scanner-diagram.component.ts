/**
 * @license
 * SPDX-License-Identifier: Apache-2.0
 */

import { Component, OnDestroy, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';

interface Step {
  title: string;
  icon: string; // Emoji or custom
  desc: string;
}

@Component({
  selector: 'lamad-value-scanner-diagram',
  standalone: true,
  imports: [CommonModule],
  templateUrl: './value-scanner-diagram.component.html',
  styleUrls: ['./value-scanner-diagram.component.css']
})
export class ValueScannerDiagramComponent implements OnInit, OnDestroy {
  step = 0;
  private intervalId: any;

  steps: Step[] = [
    { title: "Scan", icon: "🍓", desc: "Tommy scans strawberries" },
    { title: "Negotiate", icon: "🤖", desc: "Agents align: Budget vs. Sister's Joy" },
    { title: "Bundle", icon: "{ }", desc: "Care + $ + Supply Chain" },
    { title: "Story", icon: "❤️", desc: "Value Visible: Care Token Earned" },
  ];

  ngOnInit() {
    this.intervalId = setInterval(() => {
      this.step = (this.step + 1) % 4;
    }, 2500);
  }

  ngOnDestroy() {
    if (this.intervalId) {
      clearInterval(this.intervalId);
    }
  }
}