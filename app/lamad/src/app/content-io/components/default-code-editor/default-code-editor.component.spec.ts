import { ComponentFixture, TestBed } from '@angular/core/testing';

import { DefaultCodeEditorComponent } from './default-code-editor.component';
import { ContentNode } from '../../../models/content-node.model';

describe('DefaultCodeEditorComponent', () => {
  let component: DefaultCodeEditorComponent;
  let fixture: ComponentFixture<DefaultCodeEditorComponent>;

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [DefaultCodeEditorComponent],
    }).compileComponents();

    fixture = TestBed.createComponent(DefaultCodeEditorComponent);
    component = fixture.componentInstance;

    // Set required input
    component.node = {
      id: 'test-node',
      title: 'Test Content',
      contentFormat: 'markdown',
      content: '# Test Content',
    } as ContentNode;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });
});
