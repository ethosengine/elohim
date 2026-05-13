---
name: Schema-data enum drift cascades to fake auth-credential bugs
description: When seed data uses an enum value the protocol schema rejects, doorway 503s on registration cascade silently into 401 INVALID_CREDENTIALS at login — masquerading as fixture-credential bugs. Always check seed-humans.log before chasing 401s.
type: feedback
originSessionId: ccbf3926-fd47-4d11-a145-40de2ac32777
---
When a fixture human's login surfaces `POST /auth/login returned 401: INVALID_CREDENTIALS`, do NOT assume the credentials are wrong. Check the **seed-humans phase log first** for HTTP 503 / validation errors during their registration.

**Why:** Discovered 2026-05-06 in shift `rca-genesis-browser-failure-classes` iter-1. Genesis #977 had 3 visible Susan-401 scenarios that looked like a fixture-credential mismatch. The actual cause: `genesis/data/humans/humans.json` carried `profileReach: "familiar"` for 9 of 33 humans, but the protocol schema's `profile_reach` enum only accepts `["public", "community", "private"]`. Doorway rejected the registration with 503 + validation error; the human never got created; later login attempts naturally 401'd because the account didn't exist.

The cucumber failure shows `INVALID_CREDENTIALS` (the surface error), but the truth is in the seed log: `Invalid profile_reach 'familiar'. Must be one of: ["public", "community", "private"]`. Without seed-log discipline, debugging chases credential derivation logic in `genesis/a2o/src/framework/fixtures/humans.ts` for hours.

**How to apply:** Before any session debugging fixture-human auth failures, pull the seed-humans phase from the build's console log (or local `npm run hc:start:seed` output) and grep for the failing human + look for "Failed humans" or "X" markers. If the human failed to seed at all, the auth-failure is a downstream symptom — fix the data/schema mismatch, not the credentials. This pattern likely applies to any enum-bearing seed entity (humans, content nodes, presences, collectives) where doorway/zome validation diverges from the data file.

**The blast radius is bigger than the visible failures suggest.** Only 3 of the 9 broken humans surfaced as test failures in #977 because only those 3 had scenarios running. The other 6 are time-bombs — any future scenario that references one of them will surface as another mystery 401.
