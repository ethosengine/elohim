import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { ConfigService, AppConfig } from '../../services/config.service';

@Component({
  selector: 'app-debug-bar',
  imports: [CommonModule],
  templateUrl: './debug-bar.component.html',
  styleUrl: './debug-bar.component.css'
})
export class DebugBarComponent implements OnInit {
  config: AppConfig | null = null;
  showDebugBar = false;

  constructor(private configService: ConfigService) {}

  ngOnInit() {
    try {
      this.config = this.configService.getConfig();
      this.showDebugBar = this.config.environment === 'staging';
    } catch (error) {
      this.showDebugBar = false;
    }
  }
}
