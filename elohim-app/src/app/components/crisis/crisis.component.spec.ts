import { ComponentFixture, TestBed } from '@angular/core/testing';

import { CrisisComponent } from './crisis.component';

describe('CrisisComponent', () => {
  let component: CrisisComponent;
  let fixture: ComponentFixture<CrisisComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CrisisComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(CrisisComponent);
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
    expect(heading.textContent).toBe('The Architectural Crisis');
  });

  it('should render three subsection headings', () => {
    const compiled = fixture.nativeElement;
    const headings = compiled.querySelectorAll('h3');
    expect(headings.length).toBe(3);
    expect(headings[0].textContent).toContain('Economic Architecture');
    expect(headings[1].textContent).toContain('Collective Intelligence Architecture');
    expect(headings[2].textContent).toContain('Digital Architecture');
  });

  it('should render vision grids with items', () => {
    const compiled = fixture.nativeElement;
    const grids = compiled.querySelectorAll('.vision-grid');
    expect(grids.length).toBe(3);

    const items = compiled.querySelectorAll('.vision-item');
    expect(items.length).toBeGreaterThan(0);
  });

  it('should render quotes with authors', () => {
    const compiled = fixture.nativeElement;
    const quotes = compiled.querySelectorAll('.quote');
    expect(quotes.length).toBe(2);

    const authors = compiled.querySelectorAll('.quote-author');
    expect(authors.length).toBe(2);
    expect(authors[0].textContent).toContain('Edward O. Wilson');
    expect(authors[1].textContent).toContain('Marshall McLuhan');
  });
});
