import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';

import { of } from 'rxjs';

import { EPR_RESOLUTION_PROVIDER } from '@app/elohim/providers/epr-resolution.provider';
import { ResilienceService } from '@app/lamad/services/resilience.service';

import { PathForwardComponent } from './path-forward.component';

// Embedded epr-relationship-card resolves live heads through the ambient
// EprResolutionProvider (I2); stub it + resilience so structural assertions
// stay network-free.
const resolutionStub = {
  resolveHead: () => Promise.resolve({ state: 'missing' }),
  resolveRoute: () => null,
  resolveBody: () => Promise.resolve({ state: 'missing' }),
};
const resilienceStub = { getContentResilience: () => of(null) };

describe('PathForwardComponent', () => {
  let component: PathForwardComponent;
  let fixture: ComponentFixture<PathForwardComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PathForwardComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: EPR_RESOLUTION_PROVIDER, useValue: resolutionStub },
        { provide: ResilienceService, useValue: resilienceStub },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(PathForwardComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render the main heading', () => {
    const compiled = fixture.nativeElement;
    const heading = compiled.querySelector('h2');
    expect(heading).toBeTruthy();
    expect(heading.textContent).toBe('The Path Forward');
  });

  it('should render three epr-linked audience cards', () => {
    const compiled = fixture.nativeElement;
    const grid = compiled.querySelector('.card-grid');
    expect(grid).toBeTruthy();

    const cards = compiled.querySelectorAll('app-epr-relationship-card');
    expect(cards.length).toBe(3);
    expect(component.audiences.length).toBe(3);
  });

  it('should point each card at a path-forward concept node', () => {
    const targets = component.audiences.map(a => a.target);
    expect(targets).toContain('concept-path-forward-policymakers');
    expect(targets).toContain('concept-path-forward-developers');
    expect(targets).toContain('concept-path-forward-communities');
  });
});
