import { Component, input } from '@angular/core';

// @coverage: 50.0% (2026-02-24)

@Component({
  selector: 'app-profile-attestations-section',
  standalone: true,
  templateUrl: './profile-attestations-section.component.html',
  styleUrls: ['./profile-attestations-section.component.css'],
})
export class ProfileAttestationsSectionComponent {
  readonly attestations = input.required<string[]>();
}
