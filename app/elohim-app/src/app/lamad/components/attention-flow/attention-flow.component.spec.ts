import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';
import { of } from 'rxjs';

import { AttentionFlowComponent } from './attention-flow.component';
import { EventService } from '@app/shefa/services/event.service';
import { AgentService } from '@app/elohim/services/agent.service';

describe('AttentionFlowComponent', () => {
  let component: AttentionFlowComponent;
  let fixture: ComponentFixture<AttentionFlowComponent>;

  const mockEvents = [
    {
      id: 'evt-1',
      lamadEventType: 'content-view',
      contentId: 'concept-trust',
      createdAt: '2026-04-01T10:00:00Z',
    },
    {
      id: 'evt-2',
      lamadEventType: 'content-view',
      contentId: 'concept-governance',
      createdAt: '2026-04-01T11:00:00Z',
    },
    {
      id: 'evt-3',
      lamadEventType: 'content-complete',
      contentId: 'concept-trust',
      createdAt: '2026-04-01T10:15:00Z',
    },
  ];

  beforeEach(async () => {
    const eventServiceSpy = jasmine.createSpyObj('EventService', ['getRecentEvents']);
    const agentServiceSpy = jasmine.createSpyObj('AgentService', ['getCurrentAgentId']);

    eventServiceSpy.getRecentEvents.and.returnValue(of(mockEvents));
    agentServiceSpy.getCurrentAgentId.and.returnValue('agent-maya-123');

    await TestBed.configureTestingModule({
      imports: [AttentionFlowComponent, RouterModule.forRoot([])],
      providers: [
        { provide: EventService, useValue: eventServiceSpy },
        { provide: AgentService, useValue: agentServiceSpy },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(AttentionFlowComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('creates', () => {
    expect(component).toBeTruthy();
  });

  it('loads recent attention events', () => {
    expect(component.events.length).toBe(3);
  });

  it('calculates unique content count', () => {
    expect(component.uniqueContentCount).toBe(2);
  });

  it('returns correct icon for content-view', () => {
    expect(component.getEventIcon({ lamadEventType: 'content-view' } as any)).toBe('\u{1F441}');
  });

  it('returns correct label for content-complete', () => {
    expect(component.getEventLabel({ lamadEventType: 'content-complete' } as any)).toBe(
      'Completed',
    );
  });
});
