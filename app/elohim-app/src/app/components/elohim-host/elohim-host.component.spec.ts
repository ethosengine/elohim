import { ComponentFixture, TestBed } from '@angular/core/testing';

import { ElohimHostComponent } from './elohim-host.component';

describe('ElohimHostComponent', () => {
  let component: ElohimHostComponent;
  let fixture: ComponentFixture<ElohimHostComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [ElohimHostComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(ElohimHostComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render the wisdom-in-the-fabric heading', () => {
    const compiled = fixture.nativeElement;
    const heading = compiled.querySelector('h2');
    expect(heading).toBeTruthy();
    expect(heading.textContent).toBe('Wisdom in the fabric');
  });

  it('should describe the three honest-architecture layers', () => {
    const compiled = fixture.nativeElement;
    const text = compiled.textContent;
    expect(text).toContain('Holochain DHT');
    expect(text).toContain('libp2p');
    expect(text).toContain('Doorways');
  });

  it('should render the three bounded elohim roles', () => {
    const compiled = fixture.nativeElement;
    const cardHeadings = compiled.querySelectorAll('.card-grid .card h3');
    const headingTexts = Array.from(cardHeadings).map(h => (h as HTMLElement).textContent);
    expect(headingTexts).toContain('Counsel');
    expect(headingTexts).toContain('Operator');
    expect(headingTexts).toContain('Adjuster');
  });

  it('should explicitly disclaim the blockchain / nation-state framing', () => {
    const compiled = fixture.nativeElement;
    const text = compiled.textContent as string;
    // The copy names blockchain/smart-contracts only to reject them.
    expect(text).toContain('No blockchain');
    expect(text).toContain('No smart contracts');
    // The old, factually-wrong framing must be gone.
    expect(text).not.toContain('Constitutional Blockchain');
    expect(text).not.toContain('Nation-State');
  });
});
