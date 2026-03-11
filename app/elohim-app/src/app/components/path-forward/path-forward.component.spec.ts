import { ComponentFixture, TestBed } from '@angular/core/testing';

import { PathForwardComponent } from './path-forward.component';

describe('PathForwardComponent', () => {
  let component: PathForwardComponent;
  let fixture: ComponentFixture<PathForwardComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [PathForwardComponent],
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

  it('should render card grid with three audience cards', () => {
    const compiled = fixture.nativeElement;
    const grid = compiled.querySelector('.card-grid');
    expect(grid).toBeTruthy();

    const cards = compiled.querySelectorAll('.card');
    expect(cards.length).toBe(3);
  });

  it('should render cards for different stakeholders', () => {
    const compiled = fixture.nativeElement;
    const cardHeadings = compiled.querySelectorAll('.card h3');
    expect(cardHeadings.length).toBe(3);

    const headingTexts = Array.from(cardHeadings).map(h => (h as HTMLElement).textContent);
    expect(headingTexts).toContain('For Policymakers');
    expect(headingTexts).toContain('For Developers');
    expect(headingTexts).toContain('For Communities');
  });

  it('should render descriptions for all cards', () => {
    const compiled = fixture.nativeElement;
    const descriptions = compiled.querySelectorAll('.card p');
    expect(descriptions.length).toBe(3);
  });
});
