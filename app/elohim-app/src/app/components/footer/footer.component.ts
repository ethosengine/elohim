import { HttpClient } from '@angular/common/http';
import { Component, inject, signal, ChangeDetectionStrategy } from '@angular/core';

// @coverage: 100.0% (2026-02-24)

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
  imports: [],
  templateUrl: './footer.component.html',
  changeDetection: ChangeDetectionStrategy.Eager,
  styleUrl: './footer.component.css',
})
export class FooterComponent {
  private readonly http = inject(HttpClient);

  readonly lamadHref = '/lamad'; // route-literal-ok: sanctioned cross-bundle mount link (elohim-app gospel), not a minted content link
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
