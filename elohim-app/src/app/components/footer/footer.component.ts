import { HttpClient } from '@angular/common/http';
import { Component, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { environment } from '../../../environments/environment';

export interface BuildInfo {
  commit: string;
  version: string;
  buildTime: string;
  environment: string;
  service: string;
}

@Component({
  selector: 'app-footer',
  imports: [RouterLink],
  templateUrl: './footer.component.html',
  styleUrl: './footer.component.css',
})
export class FooterComponent {
  private readonly http = inject(HttpClient);

  gitHash = environment.gitHash;
  githubCommitUrl = `https://github.com/ethosengine/elohim/commit/${environment.gitHash}`;
  buildInfo = signal<BuildInfo | null>(null);

  constructor() {
    if (environment.gitHash !== 'local-dev') {
      this.http.get<BuildInfo>('/version.json').subscribe({
        next: info => {
          this.buildInfo.set(info);
          this.githubCommitUrl = `https://github.com/ethosengine/elohim/commit/${info.commit}`;
        },
        error: () => {
          // Silently fall back to environment.gitHash display
        },
      });
    }
  }
}
