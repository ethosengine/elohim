import { CommonModule } from '@angular/common';
import { Component, OnInit } from '@angular/core';

import { ConfigService, AppConfig } from '../../services/config.service';

@Component({
  selector: 'app-debug-bar',
  imports: [CommonModule],
  templateUrl: './debug-bar.component.html',
  styleUrl: './debug-bar.component.css',
})
export class DebugBarComponent implements OnInit {
  config: AppConfig | null = null;
  showDebugBar = false;
  environmentLabel = '';

  constructor(private readonly configService: ConfigService) {}

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
