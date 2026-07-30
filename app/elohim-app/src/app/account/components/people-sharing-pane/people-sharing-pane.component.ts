import { Component, ChangeDetectionStrategy } from '@angular/core';

@Component({
  selector: 'app-people-sharing-pane',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.Eager,
  template: `
    <h1 data-testid="pane-title-people-sharing">People &amp; sharing</h1>
    <p>[M5 scaffold — content lands in a follow-on sprint]</p>
  `,
})
export class PeopleSharingPaneComponent {}
