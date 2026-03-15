import { describe, it, expect, beforeEach } from 'vitest';
import { TestBed } from '@angular/core/testing';
import { AvodahApiService } from './avodah-api.service';

describe('AvodahApiService', () => {
  let service: AvodahApiService;

  beforeEach(() => {
    TestBed.configureTestingModule({});
    service = TestBed.inject(AvodahApiService);
  });

  it('getProjects returns at least one mock project', async () => {
    const projects = await service.getProjects();
    expect(projects.length).toBeGreaterThan(0);
    expect(projects[0].contentType).toBe('work-project');
  });

  it('getStoriesForProject returns stories with matching projectId', async () => {
    const projects = await service.getProjects();
    const projectId = projects[0].id;
    const stories = await service.getStoriesForProject(projectId);
    expect(
      stories.every(s => {
        const meta = s.metadata as Record<string, unknown>;
        return meta['projectId'] === projectId;
      }),
    ).toBe(true);
  });
});
