import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';

import { of } from 'rxjs';

import { EprResolverService } from '@app/elohim/services/epr-resolver.service';
import { ResilienceService } from '@app/lamad/services/resilience.service';

import { VisionComponent } from './vision.component';

// The embedded epr-relationship-card resolves live heads at runtime; in unit
// tests we stub the resolver + resilience services so the structural assertions
// (testids, counts, glosses) don't depend on the network.
const resolverStub = { resolve: () => of(null) };
const resilienceStub = { getContentResilience: () => of(null) };

describe('VisionComponent', () => {
  let component: VisionComponent;
  let fixture: ComponentFixture<VisionComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [VisionComponent],
      providers: [
        provideHttpClient(),
        provideHttpClientTesting(),
        { provide: EprResolverService, useValue: resolverStub },
        { provide: ResilienceService, useValue: resilienceStub },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(VisionComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render the "what is already here" heading', () => {
    const compiled = fixture.nativeElement;
    const heading = compiled.querySelector('h2');
    expect(heading).toBeTruthy();
    expect(heading.textContent).toBe("What's already here");
  });

  it('should render the seven pillar glosses under the pillars testid', () => {
    const compiled = fixture.nativeElement;
    const pillars = compiled.querySelector('[data-testid="landing-pillars"]');
    expect(pillars).toBeTruthy();

    const items = pillars.querySelectorAll('.vision-item');
    expect(items.length).toBe(7);

    const text = pillars.textContent;
    expect(text).toContain('imagodei');
    expect(text).toContain('mishpat');
    expect(text).toContain('doorway');
  });

  it('should render the six epic story cards', () => {
    const compiled = fixture.nativeElement;
    const container = compiled.querySelector('[data-testid="landing-epic-cards"]');
    expect(container).toBeTruthy();

    const cards = compiled.querySelectorAll('[data-testid="landing-epic-card"]');
    expect(cards.length).toBe(6);
    expect(component.epics.length).toBe(6);
  });
});
