import { Component, ChangeDetectionStrategy } from '@angular/core';

@Component({
  selector: 'app-third-party-apps-pane',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.Eager,
  template: `
    <h1 data-testid="pane-title-third-party-apps">Third-party apps</h1>
    <p>[M5 scaffold — content lands in a follow-on sprint]</p>
  `,
})
export class ThirdPartyAppsPaneComponent {}
