import { ComponentFixture, TestBed } from '@angular/core/testing';
import { provideRouter } from '@angular/router';

import { environment } from '../../../environments/environment';

import { FooterComponent } from './footer.component';

describe('FooterComponent', () => {
  let component: FooterComponent;
  let fixture: ComponentFixture<FooterComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [FooterComponent],
      providers: [provideRouter([])],
    }).compileComponents();

    fixture = TestBed.createComponent(FooterComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
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
});
