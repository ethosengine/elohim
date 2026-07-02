import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideHttpClientTesting } from '@angular/common/http/testing';

import { CallToActionComponent } from './call-to-action.component';

describe('CallToActionComponent', () => {
  let component: CallToActionComponent;
  let fixture: ComponentFixture<CallToActionComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CallToActionComponent],
      providers: [provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();

    fixture = TestBed.createComponent(CallToActionComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render the main heading', () => {
    const compiled = fixture.nativeElement;
    const heading = compiled.querySelector('h3');
    expect(heading).toBeTruthy();
    expect(heading.textContent).toBe('Love as Technology');
  });

  it('should render the CTA button', () => {
    const compiled = fixture.nativeElement;
    const button = compiled.querySelector('.cta-button');
    expect(button).toBeTruthy();
    expect(button.textContent?.trim()).toBe('Join the Protocol');
  });

  it('should render quotes with authors', () => {
    const compiled = fixture.nativeElement;
    const quotes = compiled.querySelectorAll('.quote');
    expect(quotes.length).toBe(2);

    const authors = compiled.querySelectorAll('.quote-author');
    expect(authors.length).toBe(2);
    expect(authors[0].textContent).toContain('William Gibson');
    expect(authors[1].textContent).toContain('Arundhati Roy');
  });

  it('should render section with correct class', () => {
    const compiled = fixture.nativeElement;
    const section = compiled.querySelector('section.section');
    expect(section).toBeTruthy();
  });

  it('should render the progressive stewardship ladder with all four rungs', () => {
    const compiled = fixture.nativeElement;
    const ladder = compiled.querySelector('[data-testid="landing-stewardship-ladder"]');
    expect(ladder).toBeTruthy();

    const text = ladder.textContent as string;
    expect(text).toContain('Visitor');
    expect(text).toContain('Hosted');
    expect(text).toContain('App Steward');
    expect(text).toContain('Node Steward');
  });

  it('should render the manifesto epr-linked card', () => {
    const compiled = fixture.nativeElement;
    const card = compiled.querySelector('[data-testid="landing-manifesto-card"]');
    expect(card).toBeTruthy();
    expect(card.getAttribute('epr')).toBe('epr:manifesto');
  });
});
