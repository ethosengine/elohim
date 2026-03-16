import { ComponentFixture, TestBed } from '@angular/core/testing';
import { HttpClient } from '@angular/common/http';
import { ActivatedRoute } from '@angular/router';
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { of } from 'rxjs';

import { JournalPageComponent } from './journal-page.component';

describe('JournalPageComponent', () => {
  let component: JournalPageComponent;
  let fixture: ComponentFixture<JournalPageComponent>;

  beforeEach(async () => {
    const mockHttp = {
      get: vi.fn().mockReturnValue(
        of({
          id: 'journal-1',
          title: 'Test',
          contentBody: 'Body',
          contentType: 'journal',
        }),
      ),
      patch: vi.fn().mockReturnValue(of({})),
      post: vi.fn().mockReturnValue(of({})),
    };

    const mockRoute = {
      paramMap: of(new Map([['id', 'journal-1']])),
    };

    await TestBed.configureTestingModule({
      imports: [JournalPageComponent],
      providers: [
        { provide: HttpClient, useValue: mockHttp },
        { provide: ActivatedRoute, useValue: mockRoute },
      ],
    }).compileComponents();

    fixture = TestBed.createComponent(JournalPageComponent);
    component = fixture.componentInstance;
    fixture.detectChanges();
  });

  it('should render two-panel layout', () => {
    const layout = fixture.nativeElement.querySelector(
      '[data-testid="journal-layout"]',
    );
    expect(layout).toBeTruthy();
  });

  it('should contain the editor', () => {
    const editor = fixture.nativeElement.querySelector('app-journal-editor');
    expect(editor).toBeTruthy();
  });

  it('should contain the sidebar', () => {
    const sidebar = fixture.nativeElement.querySelector('app-elohim-sidebar');
    expect(sidebar).toBeTruthy();
  });
});
