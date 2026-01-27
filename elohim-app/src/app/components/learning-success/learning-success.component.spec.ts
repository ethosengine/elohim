import { ComponentFixture, TestBed } from '@angular/core/testing';

import { LearningSuccessComponent } from './learning-success.component';

describe('LearningSuccessComponent', () => {
  let component: LearningSuccessComponent;
  let fixture: ComponentFixture<LearningSuccessComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LearningSuccessComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(LearningSuccessComponent);
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
    expect(heading.textContent).toBe('Learning from Successful Models');
  });

  it('should render card grid with four cards', () => {
    const compiled = fixture.nativeElement;
    const grid = compiled.querySelector('.card-grid');
    expect(grid).toBeTruthy();

    const cards = compiled.querySelectorAll('.card');
    expect(cards.length).toBe(4);
  });

  it('should render all card headings', () => {
    const compiled = fixture.nativeElement;
    const cardHeadings = compiled.querySelectorAll('.card h3');
    expect(cardHeadings.length).toBe(4);

    const headingTexts = Array.from(cardHeadings).map(h => (h as HTMLElement).textContent);
    expect(headingTexts).toContain('The Scandinavian Insight');
    expect(headingTexts).toContain('Indigenous Wisdom');
    expect(headingTexts).toContain('Intergenerational Thinking');
    expect(headingTexts).toContain('Engineering Specifications');
  });

  it('should render closing paragraph', () => {
    const compiled = fixture.nativeElement;
    const paragraphs = compiled.querySelectorAll('p');
    expect(paragraphs.length).toBeGreaterThan(0);
  });
});
