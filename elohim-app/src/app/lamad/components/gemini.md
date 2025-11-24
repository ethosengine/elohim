# Lamad UI Components Implementation Guide

Ref: `LAMAD_API_SPECIFICATION_v1.0.md` - Part 1: URL Strategy & UI

## Objective
Build the views that consume the Services and Models. The UI should reflect the "Journey" metaphor.

## Tasks

### 1. `PathNavigatorComponent` (New)
- [ ] Create `components/path-navigator/`.
- [ ] Display the "Step View" defined in Section 1.1.
- [ ] Inputs: `PathStepView` (from PathService).
- [ ] Features:
  - Next/Prev buttons (stay within path context).
  - Narrative Overlay ("Why this step matters").
  - Progress Indicator.

### 2. `LearningFrontierComponent` (New)
- [ ] Create `components/learning-frontier/`.
- [ ] Display "What am I ready to learn next?".
- [ ] Use `AgentService.getLearningFrontier()`.
- [ ] Show cards for unlocked content.

### 3. Update `LamadHomeComponent`
- [ ] Refactor to be "Path Centric".
- [ ] Show "My Active Paths".
- [ ] Show "Recommended Paths".
- [ ] Remove or hide the raw "Epic/Feature" list browsing (move to "Explore" section).

### 4. `AttestationWalletComponent` (New)
- [ ] Create `components/attestation-wallet/`.
- [ ] Display earned attestations (Badges/Credentials).
- [ ] Show locked/unlocked capabilities.

## Design Principles
- **Fog of War**: Visually indicate locked content (blur, lock icon) and explain *how* to earn access.
- **Sacred Attention**: Minimal distraction. Focus on the current step's content and narrative.
