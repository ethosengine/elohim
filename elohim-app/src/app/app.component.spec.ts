import { TestBed } from '@angular/core/testing';
import { provideHttpClient } from '@angular/common/http';
import { provideRouter, Router } from '@angular/router';
import { AppComponent } from './app.component';
import { NavigationEnd } from '@angular/router';
import { Subject } from 'rxjs';

describe('AppComponent', () => {
  let routerEventsSubject: Subject<any>;
  let mockRouter: any;

  beforeEach(async () => {
    routerEventsSubject = new Subject();
    mockRouter = {
      events: routerEventsSubject.asObservable(),
      url: '/'
    };

    await TestBed.configureTestingModule({
      imports: [AppComponent],
      providers: [
        provideHttpClient(),
        provideRouter([]),
        { provide: Router, useValue: mockRouter }
      ]
    }).compileComponents();
  });

  it('should create the app', () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;
    expect(app).toBeTruthy();
  });

  it(`should have the 'elohim-app' title`, () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;
    expect(app.title).toEqual('elohim-app');
  });

  it('should render router outlet', () => {
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('router-outlet')).toBeTruthy();
  });

  it('should show floating toggle on root landing page (/)', () => {
    mockRouter.url = '/';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.showFloatingToggle).toBe(true);
  });

  it('should hide floating toggle on lamad routes', () => {
    mockRouter.url = '/lamad/home';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.showFloatingToggle).toBe(false);
  });

  it('should hide floating toggle on shefa routes', () => {
    mockRouter.url = '/shefa';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.showFloatingToggle).toBe(false);
  });

  it('should hide floating toggle on qahal routes', () => {
    mockRouter.url = '/qahal';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.showFloatingToggle).toBe(false);
  });

  it('should update showFloatingToggle when navigating away from root', () => {
    mockRouter.url = '/';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();
    expect(app.showFloatingToggle).toBe(true);

    routerEventsSubject.next(new NavigationEnd(1, '/lamad/search', '/lamad/search'));

    expect(app.showFloatingToggle).toBe(false);
  });

  it('should update showFloatingToggle when navigating to root', () => {
    mockRouter.url = '/lamad/home';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();
    expect(app.showFloatingToggle).toBe(false);

    routerEventsSubject.next(new NavigationEnd(2, '/', '/'));

    expect(app.showFloatingToggle).toBe(true);
  });

  it('should render theme toggle component on root page', () => {
    mockRouter.url = '/';
    const fixture = TestBed.createComponent(AppComponent);
    fixture.componentInstance.ngOnInit();
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('app-theme-toggle')).toBeTruthy();
  });

  it('should not render theme toggle component on non-root pages', () => {
    mockRouter.url = '/lamad';
    const fixture = TestBed.createComponent(AppComponent);
    fixture.componentInstance.ngOnInit();
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('app-theme-toggle')).toBeFalsy();
  });
});
