import { ComponentFixture, TestBed } from '@angular/core/testing';

import { DesignPrinciplesComponent } from './design-principles.component';

describe('DesignPrinciplesComponent', () => {
  let component: DesignPrinciplesComponent;
  let fixture: ComponentFixture<DesignPrinciplesComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DesignPrinciplesComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(DesignPrinciplesComponent);
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
    expect(heading.textContent).toBe('Design Principles');
  });

  it('should render card grid with six principle cards', () => {
    const compiled = fixture.nativeElement;
    const grid = compiled.querySelector('.card-grid');
    expect(grid).toBeTruthy();

    const cards = compiled.querySelectorAll('.card');
    expect(cards.length).toBe(6);
  });

  it('should render all card headings', () => {
    const compiled = fixture.nativeElement;
    const cardHeadings = compiled.querySelectorAll('.card h3');
    expect(cardHeadings.length).toBe(6);

    const headingTexts = Array.from(cardHeadings).map(h => (h as HTMLElement).textContent);
    expect(headingTexts).toContain('Peer-to-Peer Architecture');
    expect(headingTexts).toContain('Graduated Intimacy');
    expect(headingTexts).toContain('Transparency as Immune System');
  });

  it('should render descriptions for all cards', () => {
    const compiled = fixture.nativeElement;
    const descriptions = compiled.querySelectorAll('.card p');
    expect(descriptions.length).toBe(6);
  });
});
