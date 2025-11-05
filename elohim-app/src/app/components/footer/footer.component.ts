import { Component } from '@angular/core';
import { environment } from '../../../environments/environment';

@Component({
  selector: 'app-footer',
  imports: [],
  templateUrl: './footer.component.html',
  styleUrl: './footer.component.css'
})
export class FooterComponent {
  gitHash = environment.gitHash;
  githubCommitUrl = `https://github.com/ethosengine/elohim/commit/${environment.gitHash}`;
}
