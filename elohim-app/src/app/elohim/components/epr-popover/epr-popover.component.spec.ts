import { vi } from 'vitest';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { RouterModule } from '@angular/router';

import { EprPopoverComponent } from './epr-popover.component';

import type { EprHead } from '../../models/epr-head.model';

describe('EprPopoverComponent', () => {
  let component: EprPopoverComponent;
  let fixture: ComponentFixture<EprPopoverComponent>;

  const mockHead: EprHead = {
    version: 1,
    id: 'test-concept',
    content: 'bafkreihdwdcefgh4dqkjv67uzcmw7ojee6xedzdetojuzjevtenxquvyku',
    lamad: {
      title: 'Test Concept',
      contentType: 'concept',
      description: 'A test concept for unit testing',
      contentFormat: 'markdown',
      tags: ['test', 'unit'],
    },
    shefa: {
      stewards: ['did:web:alice.example.com'],
      allocations: [100],
    },
    qahal: {
      reach: 'community',
      layer: 'communal',
    },
    relationships: [],
  };

  beforeEach(async () => {
    await TestBed.configureTestingModule({
      imports: [EprPopoverComponent, RouterModule.forRoot([])],
    }).compileComponents();

    fixture = TestBed.createComponent(EprPopoverComponent);
    component = fixture.componentInstance;
  });

  it('should create', () => {
    expect(component).toBeTruthy();
  });

  it('should not render when head is null', () => {
    fixture.detectChanges();
    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="epr-popover"]')).toBeNull();
  });

  it('should render title and content type when head is set', () => {
    component.head = mockHead;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const title = el.querySelector('[data-testid="epr-popover-title"]');
    const type = el.querySelector('[data-testid="epr-popover-type"]');

    expect(title?.textContent?.trim()).toBe('Test Concept');
    expect(type?.textContent?.trim()).toBe('concept');
  });

  it('should render description', () => {
    component.head = mockHead;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const desc = el.querySelector('[data-testid="epr-popover-desc"]');
    expect(desc?.textContent?.trim()).toBe('A test concept for unit testing');
  });

  it('should render content format', () => {
    component.head = mockHead;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const format = el.querySelector('[data-testid="epr-popover-format"]');
    expect(format?.textContent?.trim()).toBe('markdown');
  });

  it('should render tags', () => {
    component.head = mockHead;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const tags = el.querySelector('[data-testid="epr-popover-tags"]');
    expect(tags).toBeTruthy();

    const tagElements = el.querySelectorAll('.epr-popover-tag');
    expect(tagElements.length).toBe(2);
    expect(tagElements[0].textContent?.trim()).toBe('test');
    expect(tagElements[1].textContent?.trim()).toBe('unit');
  });

  it('should render shefa stewards count', () => {
    component.head = mockHead;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const shefa = el.querySelector('[data-testid="epr-popover-shefa"]');
    expect(shefa).toBeTruthy();
    expect(shefa?.textContent).toContain('Stewards');
    expect(shefa?.textContent).toContain('1');
  });

  it('should render qahal reach and layer', () => {
    component.head = mockHead;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const qahal = el.querySelector('[data-testid="epr-popover-qahal"]');
    expect(qahal).toBeTruthy();

    const reach = el.querySelector('.epr-popover-reach');
    const layer = el.querySelector('.epr-popover-layer');
    expect(reach?.textContent?.trim()).toBe('community');
    expect(layer?.textContent?.trim()).toBe('communal');
  });

  it('should render "Open resource" link when route is provided', () => {
    component.head = mockHead;
    component.route = ['/lamad/resource', 'test-concept'];
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const link = el.querySelector('[data-testid="epr-popover-link"]');
    expect(link).toBeTruthy();
    expect(link?.textContent?.trim()).toBe('Open resource');
  });

  it('should not render link when route is null', () => {
    component.head = mockHead;
    component.route = null;
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="epr-popover-link"]')).toBeNull();
  });

  it('should emit dismissed on mouse leave', () => {
    component.head = mockHead;
    fixture.detectChanges();

    const spy = vi.spyOn(component.dismissed, 'emit');
    component.onMouseLeave();
    expect(spy).toHaveBeenCalled();
  });

  it('should hide optional sections when context is empty', () => {
    component.head = {
      ...mockHead,
      lamad: { title: 'Minimal', contentType: 'concept' },
      shefa: {},
      qahal: {},
    };
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    expect(el.querySelector('[data-testid="epr-popover-desc"]')).toBeNull();
    expect(el.querySelector('[data-testid="epr-popover-format"]')).toBeNull();
    expect(el.querySelector('[data-testid="epr-popover-tags"]')).toBeNull();
    expect(el.querySelector('[data-testid="epr-popover-shefa"]')).toBeNull();
    expect(el.querySelector('[data-testid="epr-popover-qahal"]')).toBeNull();
  });

  it('should position at specified coordinates', () => {
    component.head = mockHead;
    component.position = { top: 200, left: 150 };
    fixture.detectChanges();

    const el = fixture.nativeElement as HTMLElement;
    const popover = el.querySelector('[data-testid="epr-popover"]') as HTMLElement;
    expect(popover.style.top).toBe('200px');
    expect(popover.style.left).toBe('150px');
  });
});
