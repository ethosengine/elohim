import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterTestingModule } from '@angular/router/testing';
import { LearnerDashboardComponent } from './learner-dashboard.component';
import { By } from '@angular/platform-browser';

describe('LearnerDashboardComponent', () => {
  let component: LearnerDashboardComponent;
  let fixture: ComponentFixture<LearnerDashboardComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [LearnerDashboardComponent, RouterTestingModule],
    }).compileComponents();

    fixture = TestBed.createComponent(LearnerDashboardComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should display main header', () => {
    const header = fixture.debugElement.query(By.css('h1'));
    expect(header.nativeElement.textContent).toBe('My Learning');
  });

  it('should display subtitle', () => {
    const subtitle = fixture.debugElement.query(By.css('.subtitle'));
    expect(subtitle.nativeElement.textContent).toBe("Track your progress and discover what's next");
  });

  it('should display active paths placeholder', () => {
    const placeholders = fixture.debugElement.queryAll(By.css('.placeholder-card'));
    expect(placeholders.length).toBe(3);

    const activePaths = placeholders[0].nativeElement;
    expect(activePaths.querySelector('h2').textContent).toBe('Active Paths');
  });

  it('should display completed paths placeholder', () => {
    const placeholders = fixture.debugElement.queryAll(By.css('.placeholder-card'));
    const completed = placeholders[1].nativeElement;
    expect(completed.querySelector('h2').textContent).toBe('Completed');
  });

  it('should display attestations placeholder', () => {
    const placeholders = fixture.debugElement.queryAll(By.css('.placeholder-card'));
    const attestations = placeholders[2].nativeElement;
    expect(attestations.querySelector('h2').textContent).toBe('Attestations');
  });

  it('should have link to explore paths', () => {
    const link = fixture.debugElement.query(By.css('a[routerLink="/lamad"]'));
    expect(link).toBeTruthy();
    expect(link.nativeElement.textContent).toBe('Explore Paths');
  });

  it('should display coming soon message', () => {
    const comingSoon = fixture.debugElement.query(By.css('.coming-soon'));
    expect(comingSoon.nativeElement.textContent).toBe('Full dashboard coming soon...');
  });
});
