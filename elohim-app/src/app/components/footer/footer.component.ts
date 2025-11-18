import { Component } from '@angular/core';
import { RouterLink } from '@angular/router';
import { environment } from '../../../environments/environment';

@Component({
  selector: 'app-footer',
  imports: [RouterLink],
  templateUrl: './footer.component.html',
  styleUrl: './footer.component.css'
})
export class FooterComponent {
  version = environment.version;
  gitHash = environment.gitHash;
  githubReleaseUrl = `https://github.com/ethosengine/elohim/releases/v${environment.version}`;
  githubCommitUrl = `https://github.com/ethosengine/elohim/commit/${environment.gitHash}`;
}
