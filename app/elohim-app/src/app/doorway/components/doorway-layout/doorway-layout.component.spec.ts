import { TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { DoorwayLayoutComponent } from './doorway-layout.component';

describe('DoorwayLayoutComponent', () => {
  it('renders a doorway sidenav and a router outlet', async () => {
    await TestBed.configureTestingModule({
      imports: [DoorwayLayoutComponent],
      providers: [provideRouter([])],
    }).compileComponents();
    const fixture = TestBed.createComponent(DoorwayLayoutComponent);
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="doorway-sidenav"]')).toBeTruthy();
    expect(el.querySelector('router-outlet')).toBeTruthy();
  });

  it('exposes a Dashboard and Configuration nav link', async () => {
    await TestBed.configureTestingModule({
      imports: [DoorwayLayoutComponent],
      providers: [provideRouter([])],
    }).compileComponents();
    const fixture = TestBed.createComponent(DoorwayLayoutComponent);
    fixture.detectChanges();
    const links = (fixture.nativeElement as HTMLElement).querySelectorAll(
      '[data-testid="doorway-sidenav"] a'
    );
    const texts = Array.from(links).map(l => l.textContent?.trim() ?? '');
    expect(texts).toContain('Dashboard');
    expect(texts).toContain('Configuration');
  });
});
