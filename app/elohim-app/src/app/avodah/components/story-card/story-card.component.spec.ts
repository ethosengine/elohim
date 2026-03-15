import { describe, it, expect } from 'vitest';
import { TestBed } from '@angular/core/testing';
import { ContentNode, ContentMetadata } from '@app/lamad/models/content-node.model';
import { StoryCardComponent } from './story-card.component';

const MOCK_STORY: ContentNode = {
  id: 'story-test-001',
  contentType: 'work-story',
  title: 'Fix the kitchen faucet',
  description: '',
  content: '',
  contentFormat: 'markdown',
  tags: ['plumbing', 'home'],
  relatedNodeIds: [],
  metadata: {
    projectId: 'proj-001',
    status: 'backlog',
    visibility: 'exchange',
    priority: 'high',
    storyPoints: 3,
    attestationGates: ['path-plumbing-basics'],
  } as unknown as ContentMetadata,
  createdAt: new Date().toISOString(),
  updatedAt: new Date().toISOString(),
};

describe('StoryCardComponent', () => {
  it('creates', () => {
    const fixture = TestBed.createComponent(StoryCardComponent);
    fixture.componentRef.setInput('story', MOCK_STORY);
    expect(fixture.componentInstance).toBeTruthy();
  });

  it('shows attestation badge when gates are present', () => {
    const fixture = TestBed.createComponent(StoryCardComponent);
    fixture.componentRef.setInput('story', MOCK_STORY);
    fixture.detectChanges();
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('[data-testid="badge-attestation"]')).toBeTruthy();
  });

  it('shows exchange badge when visibility is exchange', () => {
    const fixture = TestBed.createComponent(StoryCardComponent);
    fixture.componentRef.setInput('story', MOCK_STORY);
    fixture.detectChanges();
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('[data-testid="badge-exchange"]')).toBeTruthy();
  });

  it('does not show cadence badge when no cadence', () => {
    const fixture = TestBed.createComponent(StoryCardComponent);
    fixture.componentRef.setInput('story', MOCK_STORY);
    fixture.detectChanges();
    const el: HTMLElement = fixture.nativeElement;
    expect(el.querySelector('[data-testid="badge-cadence"]')).toBeFalsy();
  });
});
