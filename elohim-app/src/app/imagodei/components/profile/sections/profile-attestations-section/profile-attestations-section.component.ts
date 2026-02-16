import { Component, input } from '@angular/core';

@Component({
  selector: 'app-profile-attestations-section',
  standalone: true,
  templateUrl: './profile-attestations-section.component.html',
  styleUrls: ['./profile-attestations-section.component.css'],
})
export class ProfileAttestationsSectionComponent {
  readonly attestations = input.required<string[]>();
}
