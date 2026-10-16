import { TestBed } from '@angular/core/testing';
import { signal } from '@angular/core';
import { describe, it, expect } from 'vitest';
import { ProfileAgencySectionComponent } from './profile-agency-section.component';
import { DoorwayRegistryService } from '../../../../services/doorway-registry.service';
import { AGENCY_STAGES } from '../../../../models/agency.model';

describe('Profile agency move to node entry', () => {
  it('links hosted browser humans to the selected doorway portal and hides in Tauri or other stages', () => {
    TestBed.configureTestingModule({
      imports: [ProfileAgencySectionComponent],
      providers: [
        {
          provide: DoorwayRegistryService,
          useValue: { selectedUrl: signal('https://doorway.example') },
        },
      ],
    });
    const fixture = TestBed.createComponent(ProfileAgencySectionComponent);
    fixture.componentRef.setInput('agencyInfo', AGENCY_STAGES.hosted);
    fixture.componentRef.setInput('agencyState', { currentStage: 'hosted' });
    fixture.componentRef.setInput('connectionStatus', { state: 'connected', label: 'Connected' });
    fixture.detectChanges();
    const entry = () =>
      fixture.nativeElement.querySelector(
        '[data-testid="move-to-node-entry"]'
      ) as HTMLAnchorElement | null;
    expect(entry()?.href).toBe('https://doorway.example/threshold/account#move-to-node');
    fixture.componentRef.setInput('isTauriApp', true);
    fixture.detectChanges();
    expect(entry()).toBeNull();
    fixture.componentRef.setInput('isTauriApp', false);
    for (const stage of ['visitor', 'hosted-steward', 'app-steward', 'node-steward']) {
      fixture.componentRef.setInput('agencyState', { currentStage: stage });
      fixture.detectChanges();
      expect(entry()).toBeNull();
    }
  });
});
