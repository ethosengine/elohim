---
name: feedback-async-in-computed-silently-dies
title: "void async inside computed() dies silently"
description: "Signal writes throw in computed(); an async trigger there dies at its first set() with no trace — use effect()."
metadata: 
  node_type: memory
  title: void async inside computed() dies silently
  type: feedback
  originSessionId: dcddc033-024b-4dfa-8d13-39fa5f75b9ef
  modified: 2026-09-13T04:46:35.344Z
cites:
  - app/elohim-app/src/app/imagodei/services/agency.service.ts
---

`void someAsyncFn()` called from inside an Angular `computed()` body silently eats a signal-write violation: signal writes throw in that reactive context, an `async` body runs synchronously to its first `await`, so the first `signal.set(...)` throws and `void` discards the rejected promise. There is no observable: no HTTP request, no error signal, no console entry, in-flight flags reset themselves. Found 2026-09-13 in `agency.service.ts` (the hosting account was never fetched, so "Hosted Steward" was unreachable for months) — cost two investigation rounds.

**Why:** the failure has no trace; the only smell is a service whose result signal AND error signal are both null after it "should have run".

**How to apply:** side-effect triggers belong in `effect()` keyed on the same signals, never inside `computed()`; when a service shows result null + error null + loading false, suspect a swallowed reactive-context throw before suspecting the backend. Related: [[feedback_zone_native_await_unhandled_rejection]], [[feedback_onpush_implicit_default_harness_blindness]].
