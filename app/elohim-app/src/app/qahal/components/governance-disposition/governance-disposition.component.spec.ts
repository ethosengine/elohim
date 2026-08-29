import { ComponentFixture, TestBed } from '@angular/core/testing';

import { IdentityService } from '@app/imagodei/services/identity.service';
import { GovernanceApiService } from '@elohim/service';

import { GovernanceDispositionComponent } from './governance-disposition.component';
import { vi } from 'vitest';

describe('GovernanceDispositionComponent', () => {
  let component: GovernanceDispositionComponent;
  let fixture: ComponentFixture<GovernanceDispositionComponent>;
  let mockGovernanceApi: any;
  let signedInHumanId: string | null;

  /** Create the component with no bound input — exactly how the route loads it. */
  function mount(): void {
    fixture = TestBed.createComponent(GovernanceDispositionComponent);
    component = fixture.componentInstance;
  }

  beforeEach(async () => {
    signedInHumanId = 'human-42';
    mockGovernanceApi = {
      getDisposition: vi.fn().mockResolvedValue(null),
      updateDisposition: vi.fn().mockResolvedValue(null),
      computeDisposition: vi.fn().mockResolvedValue(null),
    };

    await TestBed.configureTestingModule({
      imports: [GovernanceDispositionComponent],
      providers: [
        { provide: GovernanceApiService, useValue: mockGovernanceApi },
        { provide: IdentityService, useValue: { humanId: () => signedInHumanId } },
      ],
    }).compileComponents();
  });

  afterEach(() => {
    fixture?.destroy();
  });

  // The route (community.routes.ts) loads this component without binding humanId, so the
  // signed-in human is the subject; a required input would throw NG0950 there.
  it('should load the signed-in human when no input is bound', async () => {
    mount();

    fixture.detectChanges();
    await fixture.whenStable();

    expect(mockGovernanceApi.getDisposition).toHaveBeenCalledWith('human-42');
    expect(component.loading()).toBe(false);
  });

  it('should stop loading and show the empty state when nobody is signed in', async () => {
    signedInHumanId = null;
    mount();

    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();

    expect(mockGovernanceApi.getDisposition).not.toHaveBeenCalled();
    expect(component.loading()).toBe(false);
    expect(fixture.nativeElement.querySelector('.empty')).not.toBeNull();
  });

  it('should show the empty state when the load rejects', async () => {
    mockGovernanceApi.getDisposition.mockRejectedValue(new Error('disposition unavailable'));
    mount();

    fixture.detectChanges();
    await fixture.whenStable();
    fixture.detectChanges();

    expect(component.disposition()).toBeNull();
    expect(component.loading()).toBe(false);
    expect(fixture.nativeElement.querySelector('.empty')).not.toBeNull();
  });
});
