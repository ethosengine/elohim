import { ComponentFixture, TestBed } from '@angular/core/testing';

import { CrisisComponent } from './crisis.component';

describe('CrisisComponent', () => {
  let component: CrisisComponent;
  let fixture: ComponentFixture<CrisisComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [CrisisComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(CrisisComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render the concrete grandma-trade heading', () => {
    const compiled = fixture.nativeElement;
    const heading = compiled.querySelector('h2');
    expect(heading).toBeTruthy();
    expect(heading.textContent).toBe('What Grandma Is Trading');
  });

  it('should carry the story-arc testid on the section', () => {
    const compiled = fixture.nativeElement;
    const section = compiled.querySelector('[data-testid="landing-story-arc"]');
    expect(section).toBeTruthy();
  });

  it('should render four diagnosis cards', () => {
    const compiled = fixture.nativeElement;
    const grid = compiled.querySelector('.card-grid');
    expect(grid).toBeTruthy();

    const cards = compiled.querySelectorAll('.card');
    expect(cards.length).toBe(4);
  });

  it('should keep the Edward O. Wilson quote', () => {
    const compiled = fixture.nativeElement;
    const quotes = compiled.querySelectorAll('.quote');
    expect(quotes.length).toBe(1);

    const authors = compiled.querySelectorAll('.quote-author');
    expect(authors.length).toBe(1);
    expect(authors[0].textContent).toContain('Edward O. Wilson');
  });

  it('should explicitly reject self-sovereign identity as the fix', () => {
    const compiled = fixture.nativeElement;
    expect(compiled.textContent).toContain('not solved by self-sovereign identity');
  });
});
