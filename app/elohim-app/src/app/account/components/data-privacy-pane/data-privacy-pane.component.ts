import { Component, ChangeDetectionStrategy } from '@angular/core';

@Component({
  selector: 'app-data-privacy-pane',
  standalone: true,
  changeDetection: ChangeDetectionStrategy.Eager,
  template: `
    <h1 data-testid="pane-title-data-privacy">Data &amp; privacy</h1>
    <p>[M5 scaffold — content lands in a follow-on sprint]</p>
  `,
})
export class DataPrivacyPaneComponent {}
