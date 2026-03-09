import { CommonModule } from '@angular/common';
import { Component, OnInit, inject } from '@angular/core';
import { RouterLink } from '@angular/router';

// @coverage: 90.9% (2026-02-24)

import { ConfigService, AppConfig } from '../../services/config.service';

@Component({
  selector: 'app-debug-bar',
  imports: [CommonModule, RouterLink],
  templateUrl: './debug-bar.component.html',
  styleUrl: './debug-bar.component.css',
})
export class DebugBarComponent implements OnInit {
  config: AppConfig | null = null;
  showDebugBar = false;
  environmentLabel = '';

  private readonly configService = inject(ConfigService);

  ngOnInit() {
    this.configService.getConfig().subscribe({
      next: config => {
        this.config = config;
        const nonProdEnvironments = ['staging', 'alpha'];
        this.showDebugBar = nonProdEnvironments.includes(this.config.environment);
        this.environmentLabel = this.config.environment.toUpperCase();
      },
      error: () => {
        this.showDebugBar = false;
      },
    });
  }
}
