import { provideHttpClient } from '@angular/common/http';
import { HttpTestingController, provideHttpClientTesting } from '@angular/common/http/testing';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { environment } from '../../../environments/environment';

import { BuildInfo, FooterComponent } from './footer.component';

describe('FooterComponent', () => {
  let component: FooterComponent;
  let fixture: ComponentFixture<FooterComponent>;
  let httpTesting: HttpTestingController;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [FooterComponent],
      providers: [provideRouter([]), provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();

    httpTesting = TestBed.inject(HttpTestingController);
    fixture = TestBed.createComponent(FooterComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  afterEach(() => {
    httpTesting.verify();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should initialize gitHash from environment', () => {
    expect(component.gitHash).toBe(environment.gitHash);
  });

  it('should construct githubCommitUrl with gitHash', () => {
    const expectedUrl = `https://github.com/ethosengine/elohim/commit/${environment.gitHash}`;
    expect(component.githubCommitUrl).toBe(expectedUrl);
  });

  it('should render git hash link in template', () => {
    const compiled = fixture.nativeElement;
    const gitHashLink = compiled.querySelector('[data-cy="git-hash"]');
    expect(gitHashLink).toBeTruthy();
    expect(gitHashLink.textContent.trim()).toBe(environment.gitHash);
    expect(gitHashLink.getAttribute('href')).toBe(component.githubCommitUrl);
  });

  it('should not fetch version.json in local-dev mode', () => {
    // local-dev environment should not make any HTTP requests
    httpTesting.expectNone('/version.json');
  });

  it('should render footer structure', () => {
    const compiled = fixture.nativeElement;
    const footer = compiled.querySelector('footer');
    expect(footer).toBeTruthy();

    const container = compiled.querySelector('.footer-container');
    expect(container).toBeTruthy();
  });

  it('should render main message transition', () => {
    const compiled = fixture.nativeElement;
    const messageTransition = compiled.querySelector('.main-message-transition');
    expect(messageTransition).toBeTruthy();

    const primaryMessage = compiled.querySelector('.primary-message');
    expect(primaryMessage).toBeTruthy();
    expect(primaryMessage.textContent).toContain('technology organized around love');
  });

  it('should render project links section', () => {
    const compiled = fixture.nativeElement;
    const projectLinks = compiled.querySelector('.project-links');
    expect(projectLinks).toBeTruthy();

    const docsLink = compiled.querySelector('.docs-link');
    expect(docsLink).toBeTruthy();
  });

  it('should render support section', () => {
    const compiled = fixture.nativeElement;
    const supportSection = compiled.querySelector('.support-section');
    expect(supportSection).toBeTruthy();

    const socialLinks = compiled.querySelector('.social-links');
    expect(socialLinks).toBeTruthy();
  });

  it('should have null buildInfo initially', () => {
    expect(component.buildInfo()).toBeNull();
  });
});

describe('FooterComponent (with version.json)', () => {
  let component: FooterComponent;
  let fixture: ComponentFixture<FooterComponent>;
  let httpTesting: HttpTestingController;

  const mockBuildInfo: BuildInfo = {
    commit: 'abc1234f',
    version: '1.0.0',
    buildTime: '2026-02-17T12:00:00Z',
    environment: 'alpha',
    service: 'elohim-app',
  };

  beforeEach(async () => {
    // Temporarily override gitHash to trigger version.json fetch
    const originalGitHash = environment.gitHash;
    (environment as { gitHash: string }).gitHash = 'abc1234f';

    await TestBed.configureTestingModule({
      imports: [FooterComponent],
      providers: [provideRouter([]), provideHttpClient(), provideHttpClientTesting()],
    }).compileComponents();

    httpTesting = TestBed.inject(HttpTestingController);
    fixture = TestBed.createComponent(FooterComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();

    // Restore original gitHash
    (environment as { gitHash: string }).gitHash = originalGitHash;
  });

  afterEach(() => {
    httpTesting.verify();
  });

  it('should fetch version.json when not local-dev', () => {
    const req = httpTesting.expectOne('/version.json');
    expect(req.request.method).toBe('GET');
    req.flush(mockBuildInfo);

    expect(component.buildInfo()).toEqual(mockBuildInfo);
  });

  it('should update githubCommitUrl from version.json', () => {
    const req = httpTesting.expectOne('/version.json');
    req.flush(mockBuildInfo);

    expect(component.githubCommitUrl).toBe(
      'https://github.com/ethosengine/elohim/commit/abc1234f',
    );
  });

  it('should render enhanced build info when version.json loads', () => {
    const req = httpTesting.expectOne('/version.json');
    req.flush(mockBuildInfo);
    fixture.detectChanges();

    const compiled = fixture.nativeElement;
    const version = compiled.querySelector('.build-version');
    expect(version).toBeTruthy();
    expect(version.textContent).toContain('v1.0.0');

    const env = compiled.querySelector('.build-env');
    expect(env).toBeTruthy();
    expect(env.textContent).toContain('alpha');

    const gitLink = compiled.querySelector('[data-cy="git-hash"]');
    expect(gitLink.textContent.trim()).toBe('abc1234f');
    expect(gitLink.getAttribute('title')).toContain('2026-02-17T12:00:00Z');
  });

  it('should fall back to gitHash when version.json fails', () => {
    const req = httpTesting.expectOne('/version.json');
    req.error(new ProgressEvent('error'));
    fixture.detectChanges();

    expect(component.buildInfo()).toBeNull();
    const compiled = fixture.nativeElement;
    const gitLink = compiled.querySelector('[data-cy="git-hash"]');
    expect(gitLink).toBeTruthy();
  });
});
