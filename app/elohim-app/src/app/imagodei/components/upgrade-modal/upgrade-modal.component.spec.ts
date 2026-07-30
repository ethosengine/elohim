import { vi } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';

import { UpgradeModalComponent } from './upgrade-modal.component';
import { SessionHuman } from '../../models/session-human.model';

describe('UpgradeModalComponent', () => {
  let component: UpgradeModalComponent;
  let fixture: ComponentFixture<UpgradeModalComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [UpgradeModalComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(UpgradeModalComponent);
    component = fixture.componentInstance;
    // No detectChanges() here: several tests set @Input()s (open, session) before
    // their own single detectChanges() call. Under ChangeDetectionStrategy.Eager
    // (Angular 22), an initial render here establishes a checked baseline that a
    // later detectChanges() call — after the input value has changed — trips as
    // NG0100 ExpressionChangedAfterItHasBeenChecked. Each test renders exactly
    // once, from its own desired starting state.
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should not render dialog when open is false', () => {
    component.open = false;
    fixture.detectChanges();
    const dialog = fixture.nativeElement.querySelector('dialog');
    expect(dialog).toBeNull();
  });

  it('should render dialog when open is true', () => {
    component.open = true;
    fixture.detectChanges();
    const dialog = fixture.nativeElement.querySelector('dialog');
    expect(dialog).toBeTruthy();
  });

  it('should emit closed when close is called', () => {
    vi.spyOn(component.closed, 'emit');
    component.close();
    expect(component.closed.emit).toHaveBeenCalled();
  });

  it('should emit closed on overlay click', () => {
    vi.spyOn(component.closed, 'emit');
    component.onOverlayClick();
    expect(component.closed.emit).toHaveBeenCalled();
  });

  it('should emit closed on Escape keydown', () => {
    vi.spyOn(component.closed, 'emit');
    component.onOverlayKeydown(new KeyboardEvent('keydown', { key: 'Escape' }));
    expect(component.closed.emit).toHaveBeenCalled();
  });

  it('should not emit closed on non-Escape keydown', () => {
    vi.spyOn(component.closed, 'emit');
    component.onOverlayKeydown(new KeyboardEvent('keydown', { key: 'Enter' }));
    expect(component.closed.emit).not.toHaveBeenCalled();
  });

  it('should display session stats when session is provided', () => {
    component.open = true;
    component.session = {
      sessionId: 'test',
      displayName: 'Tester',
      createdAt: new Date().toISOString(),
      lastActiveAt: new Date().toISOString(),
      isAnonymous: true,
      accessLevel: 'visitor',
      sessionState: 'active',
      stats: {
        nodesViewed: 5,
        nodesWithAffinity: 2,
        pathsStarted: 1,
        pathsCompleted: 0,
        stepsCompleted: 3,
        totalSessionTime: 60000,
        averageSessionLength: 60000,
        sessionCount: 1,
      },
    } as SessionHuman;
    fixture.detectChanges();

    const statValues = fixture.nativeElement.querySelectorAll('.stat-value');
    expect(statValues.length).toBe(3);
    expect(statValues[0].textContent.trim()).toBe('5');
    expect(statValues[1].textContent.trim()).toBe('1');
    expect(statValues[2].textContent.trim()).toBe('3');
  });
});
