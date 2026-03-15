import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { of } from 'rxjs';
import { vi } from 'vitest';

import { GateCommentComponent } from './gate-comment.component';

describe('GateCommentComponent', () => {
  let component: GateCommentComponent;
  let fixture: ComponentFixture<GateCommentComponent>;

  beforeEach(async () => {
    const httpMock = { post: vi.fn().mockReturnValue(of({})) };

    await TestBed.configureTestingModule({
      imports: [GateCommentComponent],
      providers: [{ provide: HttpClient, useValue: httpMock }],
    }).compileComponents();

    fixture = TestBed.createComponent(GateCommentComponent);
    component = fixture.componentInstance;
    component.contentId = 'content-123';
    fixture.detectChanges();
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should render gate artifact card', () => {
    const card = fixture.nativeElement.querySelector('app-gate-artifact-card');
    expect(card).toBeTruthy();
  });

  it('should pass comment placeholder', () => {
    const textarea = fixture.nativeElement.querySelector('textarea');
    expect(textarea).toBeTruthy();
    expect(textarea.placeholder).toContain('comment');
  });

  it('should emit commentPosted on posted event', () => {
    const postedSpy = vi.fn();
    component.commentPosted.subscribe(postedSpy);

    component.onPosted({ reachTier: 'community' });

    expect(postedSpy).toHaveBeenCalledWith({ reachTier: 'community' });
  });

  it('should have gate-comment wrapper class', () => {
    const wrapper = fixture.nativeElement.querySelector('.gate-comment');
    expect(wrapper).toBeTruthy();
  });

  it('should have apiCall defined as a function', () => {
    expect(component.apiCall).toBeDefined();
    expect(typeof component.apiCall).toBe('function');
  });

  it('should delegate apiCall to StorageApiService.createComment', () => {
    const httpMock = TestBed.inject(HttpClient) as unknown as { post: ReturnType<typeof vi.fn> };
    const result = component.apiCall('hello world', { contentId: 'content-123' });

    expect(result).toBeDefined();
    expect(httpMock.post).toHaveBeenCalled();
  });
});
