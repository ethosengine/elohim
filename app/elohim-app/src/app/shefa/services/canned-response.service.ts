import { Injectable } from '@angular/core';

interface ResponseRule {
  keywords: string[];
  response: string;
}

const RULES: ResponseRule[] = [
  {
    keywords: ['what do you think', 'thoughts', 'opinion'],
    response:
      "I can see you're working through something here. When you're ready, I can help you find where this belongs.",
  },
  {
    keywords: ['help', 'stuck', "don't know", 'confused'],
    response: 'Take your time. Sometimes the writing itself is the point.',
  },
  {
    keywords: ['publish', 'share', 'post', 'send'],
    response:
      "When you're ready to share this, we can talk about where it would have the most impact. That's a conversation for when it feels right to you.",
  },
  {
    keywords: ['done', 'finished', 'ready', 'complete'],
    response: 'It reads well. What would you like to do with it?',
  },
  {
    keywords: ['delete', 'trash', 'scrap', 'throw away'],
    response: 'Your words, your call. Want to keep it as a draft instead?',
  },
];

const DEFAULT_RESPONSE = "I'm here when you need me.";

@Injectable({ providedIn: 'root' })
export class CannedResponseService {
  respond(text: string): string {
    const lower = text.toLowerCase();
    for (const rule of RULES) {
      if (rule.keywords.some(kw => lower.includes(kw))) {
        return rule.response;
      }
    }
    return DEFAULT_RESPONSE;
  }
}
