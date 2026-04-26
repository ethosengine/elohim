import { Component } from '@angular/core';

@Component({
  selector: 'app-personal-info-pane',
  standalone: true,
  template: `
    <h1 data-testid="pane-title-personal-info">Personal info</h1>
    <p>[M5 scaffold — content lands in a follow-on sprint]</p>
  `,
})
export class PersonalInfoPaneComponent {}
