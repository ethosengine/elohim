import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ElohimHostComponent } from './elohim-host.component';

describe('ElohimHostComponent', () => {
  let component: ElohimHostComponent;
  let fixture: ComponentFixture<ElohimHostComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ElohimHostComponent]
    })
    .compileComponents();

    fixture = TestBed.createComponent(ElohimHostComponent);
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
    expect(heading.textContent).toContain('elohim.host');
  });

  it('should render constitutional layered architecture section', () => {
    const compiled = fixture.nativeElement;
    const section = compiled.querySelector('.constitutional-architecture');
    expect(section).toBeTruthy();

    const layers = compiled.querySelectorAll('.layer-item');
    expect(layers.length).toBe(4);
  });

  it('should render technical architecture cards', () => {
    const compiled = fixture.nativeElement;
    const archCards = compiled.querySelectorAll('.architecture-card');
    expect(archCards.length).toBe(2);

    const constitutionalLayer = compiled.querySelector('.constitutional-layer');
    expect(constitutionalLayer).toBeTruthy();

    const runtimeLayer = compiled.querySelector('.runtime-layer');
    expect(runtimeLayer).toBeTruthy();
  });

  it('should render feature cards', () => {
    const compiled = fixture.nativeElement;
    const cards = compiled.querySelectorAll('.card-grid .card');
    expect(cards.length).toBe(3);

    const cardHeadings = compiled.querySelectorAll('.card-grid .card h3');
    const headingTexts = Array.from(cardHeadings).map((h: any) => h.textContent);
    expect(headingTexts).toContain('Redemptive Security Model');
    expect(headingTexts).toContain('Value-Generative Economics');
  });

  it('should render flourishing models section', () => {
    const compiled = fixture.nativeElement;
    const section = compiled.querySelector('.flourishing-models');
    expect(section).toBeTruthy();
  });
});
