import { Component, ChangeDetectionStrategy } from '@angular/core';

@Component({
  selector: 'app-personal-info-pane',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.Eager,
  template: `
    <h1 data-testid="pane-title-personal-info">Personal info</h1>
    <p>[M5 scaffold — content lands in a follow-on sprint]</p>
  `,
})
export class PersonalInfoPaneComponent {}
