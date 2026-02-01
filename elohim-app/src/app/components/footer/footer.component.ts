import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';

// @coverage: 100.0% (2026-01-31)

import { environment } from '../../../environments/environment';

@Component({
  selector: 'app-footer',
  imports: [RouterLink],
  templateUrl: './footer.component.html',
  styleUrl: './footer.component.css',
})
export class FooterComponent {
  gitHash = environment.gitHash;
  githubCommitUrl = `https://github.com/ethosengine/elohim/commit/${environment.gitHash}`;
}
