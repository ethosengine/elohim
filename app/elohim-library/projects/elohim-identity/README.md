# @elohim/identity

The Elohim Protocol identity SDK surface. Owns identity primitives that multiple
pillar bundles (lamad, elohim-app, future splits) consume — session management,
route guards, profile models, and attestation types.

This library is a Category C (operational) package — it holds no source-of-truth
state; everything it exposes is reconstructible from the substrate (DHT, localStorage).
Identity truth lives in the Holochain `imagodei` zome; this library holds the
Angular-layer operational client code that binds that truth to UI state.

Aligned with `cradle-to-grave-capability-gradient.md` §4 elohim mediation roles:
identity is a load-bearing distinct SDK concern, separate from shared services
(`@elohim/service`) and REA runtime (`@elohim/rea-runtime`).

## What belongs here

- `SessionHumanService` — localStorage-backed session identity, upgrade prompts, migration
- `SessionHuman` and related session types
- `ContentAccessMetadata` and content-access check types (shared across identity boundary)
- `Attestation`, `AttestationJourney`, `AttestationProgress` and related types
- Identity route guards (`identityGuard`, `sessionOrAuthGuard`, `attestationGuard`)
- `IdentityService` — unified identity abstraction (session + Holochain)
- Profile models (`HumanProfile`, `JourneyStats`, etc.) — pending Slice 2.1 L-slice

## What does NOT belong here

- `AuthService`, `AgencyService`, `RecoveryCoordinatorService` — remain in `@app/imagodei`
  until a future I-slice extension
- Business logic for mastery scoring, REA events, governance — those belong in
  `@elohim/rea-runtime` or Rust zomes
- Wire-format types mirroring Holochain entries — those belong in `@elohim/storage-client`

## Migration status (Slice 2.3)

Partially complete. Modules blocked by Slice 2.1 (L-slice) dependencies are
noted as PENDING in `src/public-api.ts`. See the sprint plan at
`genesis/docs/superpowers/plans/2026-05-25-cross-pillar-import-cleanup.md`.
