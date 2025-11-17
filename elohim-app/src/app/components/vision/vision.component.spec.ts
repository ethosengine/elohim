import { ComponentFixture, TestBed } from '@angular/core/testing';

import { VisionComponent } from './vision.component';

describe('VisionComponent', () => {
  let component: VisionComponent;
  let fixture: ComponentFixture<VisionComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [VisionComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(VisionComponent);
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
    expect(heading.textContent).toBe('The Vision Realized');
  });

  it('should render vision grid with multiple items', () => {
    const compiled = fixture.nativeElement;
    const grid = compiled.querySelector('.vision-grid');
    expect(grid).toBeTruthy();

    const items = compiled.querySelectorAll('.vision-item');
    expect(items.length).toBe(8);
  });

  it('should render key vision statements', () => {
    const compiled = fixture.nativeElement;
    const items = compiled.querySelectorAll('.vision-item');
    const itemTexts = Array.from(items).map((item: any) => item.textContent);

    expect(itemTexts.some((text: string) => text.includes('Technology serves love'))).toBe(true);
    expect(itemTexts.some((text: string) => text.includes('Communities self-govern'))).toBe(true);
    expect(itemTexts.some((text: string) => text.includes('Dark patterns are impossible'))).toBe(true);
  });

  it('should render introductory and closing paragraphs', () => {
    const compiled = fixture.nativeElement;
    const paragraphs = compiled.querySelectorAll('p');
    expect(paragraphs.length).toBe(2);
  });
});
