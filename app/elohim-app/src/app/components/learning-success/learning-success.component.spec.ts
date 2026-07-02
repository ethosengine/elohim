import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';

import { of } from 'rxjs';

import { EPR_RESOLUTION_PROVIDER } from '@app/elohim/providers/epr-resolution.provider';
import { ResilienceService } from '@app/lamad/services/resilience.service';

import { LearningSuccessComponent } from './learning-success.component';

// Embedded epr-relationship-card resolves live heads through the ambient
// EprResolutionProvider (I2); stub it + resilience so structural assertions
// stay network-free.
const resolutionStub = {
  resolveHead: () => Promise.resolve({ state: 'missing' }),
  resolveRoute: () => null,
  resolveBody: () => Promise.resolve({ state: 'missing' }),
};
const resilienceStub = { getContentResilience: () => of(null) };

describe('LearningSuccessComponent', () => {
  let component: LearningSuccessComponent;
  let fixture: ComponentFixture<LearningSuccessComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LearningSuccessComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: EPR_RESOLUTION_PROVIDER, useValue: resolutionStub },
        { provide: ResilienceService, useValue: resilienceStub },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(LearningSuccessComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render the "find your way in" heading', () => {
    const compiled = fixture.nativeElement;
    const heading = compiled.querySelector('h2');
    expect(heading).toBeTruthy();
    expect(heading.textContent).toBe('Find your way in');
  });

  it('should carry the ways-in testid on the section', () => {
    const compiled = fixture.nativeElement;
    const section = compiled.querySelector('[data-testid="landing-ways-in"]');
    expect(section).toBeTruthy();
  });

  it('should offer the self-router quiz and trust-explorable doors', () => {
    const compiled = fixture.nativeElement;
    expect(compiled.querySelector('[data-testid="landing-wayin-quiz"]')).toBeTruthy();
    expect(compiled.querySelector('[data-testid="landing-wayin-trust"]')).toBeTruthy();
  });

  it('should link into /lamad as a plain cross-bundle href', () => {
    const compiled = fixture.nativeElement;
    const browse = compiled.querySelector('[data-testid="landing-wayin-browse"]');
    expect(browse).toBeTruthy();
    expect(browse.getAttribute('href')).toBe('/lamad');
  });

  it('should fold in proven-pattern sources including restorative justice', () => {
    const compiled = fixture.nativeElement;
    expect(compiled.textContent).toContain('restorative justice');
  });
});
