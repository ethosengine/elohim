import { TestBed } from '@angular/core/testing';
import { describe, it, expect, beforeEach } from 'vitest';

import { CannedResponseService } from './canned-response.service';

describe('CannedResponseService', () => {
  let service: CannedResponseService;

  beforeEach(() => {
    TestBed.configureTestingModule({
      providers: [CannedResponseService],
    });
    service = TestBed.inject(CannedResponseService);
  });

  it('should respond to "what do you think" with reflection prompt', () => {
    const response = service.respond('So what do you think about this?');
    expect(response).toContain('working through something');
  });

  it('should respond to "help" with patience message', () => {
    const response = service.respond('I need help with this');
    expect(response).toContain('Take your time');
  });

  it('should respond to "publish" with routing hint', () => {
    const response = service.respond('Should I publish this?');
    expect(response).toContain('share');
  });

  it('should respond to "done" with affirmation', () => {
    const response = service.respond('I think I am done');
    expect(response).toContain('reads well');
  });

  it('should respond to "delete" with gentle pushback', () => {
    const response = service.respond('I want to delete this');
    expect(response).toContain('draft');
  });

  it('should return default for unrecognized input', () => {
    const response = service.respond('The sky is blue');
    expect(response).toContain('here when you need me');
  });

  it('should be case insensitive', () => {
    const response = service.respond('WHAT DO YOU THINK');
    expect(response).toContain('working through something');
  });
});
