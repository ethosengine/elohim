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

  it('should initialize isLamadRoute based on current URL', () => {
    mockRouter.url = '/lamad/home';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.isLamadRoute).toBe(true);
  });

  it('should set isLamadRoute to false for non-lamad routes', () => {
    mockRouter.url = '/home';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    expect(app.isLamadRoute).toBe(false);
  });

  it('should update isLamadRoute when navigating to lamad route', () => {
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();

    routerEventsSubject.next(new NavigationEnd(1, '/lamad/search', '/lamad/search'));

    expect(app.isLamadRoute).toBe(true);
  });

  it('should update isLamadRoute when navigating from lamad route', () => {
    mockRouter.url = '/lamad/home';
    const fixture = TestBed.createComponent(AppComponent);
    const app = fixture.componentInstance;

    app.ngOnInit();
    expect(app.isLamadRoute).toBe(true);

    routerEventsSubject.next(new NavigationEnd(2, '/home', '/home'));

    expect(app.isLamadRoute).toBe(false);
  });

  it('should render theme toggle component', () => {
    const fixture = TestBed.createComponent(AppComponent);
    fixture.detectChanges();
    const compiled = fixture.nativeElement as HTMLElement;
    expect(compiled.querySelector('app-theme-toggle')).toBeTruthy();
  });
});
