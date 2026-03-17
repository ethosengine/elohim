# Gate Artifact Card — Design Document

**Date:** 2026-03-15
**Status:** Approved

## Vision

The ElohimGate should feel like a conversation, not a permissions system. Every mutation — comment, journal entry, feedback — passes through the gate as an **artifact** that the learner authors and the elohim evaluates. The artifact is always centered; the elohim dialogue is always supporting.

Three surfaces will eventually compose this pattern:

| Surface | Container | Artifact Scale | Elohim Dialogue Position |
|---------|-----------|---------------|--------------------------|
| Comment | Inline in comment section | Small card | Below card |
| Feedback | Pop-up modal | Medium card | Below artifact in modal |
| Journal | Sidebar page layout | Full-width, rich editing | Persistent sidebar panel |

This design covers the **shared artifact card** and the **comment shell** as the first proof of the pattern.

---

## State Machine

The card transitions through five visual states:

```
DRAFT → EVALUATING → AFFIRM → POSTED
                  ↘ DIALOGUE → (re-evaluate) → AFFIRM
                  ↘ SETTLED
```

**DRAFT** — Editable textarea. The learner is composing.

**EVALUATING** — Text snaps to preview (non-editable). Subtle shimmer animation on the card border. Gate is thinking. No spinner, no progress bar — just the shimmer. If evaluation takes >3s (Deep/Constitutional tier), a small tier label appears in secondary text.

**AFFIRM** — Gate passed. Shows the computed reach as a small icon + word (tooltip for details). "Affirm & Post" button. The learner sees where their words will land before they confirm.

**DIALOGUE** — Gate paused. Elohim prompt appears below the artifact. Artifact becomes editable again. Learner revises in place and resubmits, cycling back to EVALUATING. Each round stores an observation in the backend (feedback loop).

**SETTLED** — Read-only card. Background shifts to `--surface-secondary`, text to `--lamad-text-tertiary`. Small link: "Settlement · {cooldown}" linking to the settlement EPR. No edit/resubmit until cooldown expires. When cooldown expires, a "Revisit" link appears.

**POSTED** — Brief confirmation. Shows where it landed and at what reach. Fades into the content list.

### Settlement Asymmetry

- **Comment settlement**: Closed with cooldown. The comment is frozen. Resubmission requires overcoming the judgment weight. Settlement adds negative trust_delta to behavioral observations.
- **Journal settlement**: Open-ended. The journal artifact stays editable — settlement means "not ready yet," not "no." The creative process continues with the elohim present.

---

## Reach Visualization

Reach is graduated from the trust context's composite trust score. Displayed as a single icon + word at the AFFIRM step. Tooltip on the icon for the curious. Minimal, typographic, not colorful.

| Tier | Composite Trust | Icon | Label |
|------|----------------|------|-------|
| Private | settlement | lock | Private |
| Close | 0.0 – 0.3 | person | Close |
| Community | 0.3 – 0.6 | people | Community |
| Network | 0.6 – 0.85 | globe | Network |
| Constitutional | 0.85+ | scales | Constitutional |

The learner sees reach BEFORE affirming. They can edit and resubmit to try for different reach, but the gate determines it — not them. They consent to what the gate computed.

---

## Visual Design

### Ambient Pulse (EVALUATING)

- Gradient shimmer traveling the card border perimeter
- Card text softens slightly (not greyed, just muted)
- No spinner, no "Evaluating..." text
- Pure ambient intelligence indicator

### Pause Dialogue

- Elohim text in `--lamad-text-secondary`, conversational tone
- Renders below the artifact (comment/modal) or in sidebar (journal)
- No chat bubbles, no avatars — just text
- Artifact above remains the focal point
- "Resubmit" action on the artifact, not in the dialogue

### Settlement Card

- Background: `--surface-secondary`
- Text: `--lamad-text-tertiary`
- Bottom link: `Settlement · {cooldown}` → settlement EPR
- No shadow banning — the learner sees their held artifact and knows why
- Cooldown timer visible, "Revisit" appears when expired

### General

- Follows existing component patterns: standalone, OnPush, signals, inline SCSS
- Uses CSS custom properties from global theme
- Responsive: card max-width scales per container
- No external icons — emoji consistent with trust badge system

---

## Component Architecture

### Layer 1: GateInteractionService

Pure logic service managing the artifact state machine. No UI.

- `state` signal: `'draft' | 'evaluating' | 'affirm' | 'dialogue' | 'settled' | 'posted'`
- `draftText` signal: current artifact text
- `gateResult` signal: latest GateEvaluationView from the gate
- `reachTier` computed: derived from gateResult.trustContext.compositeTrust
- `submit(text, mutationType, context)`: transitions to EVALUATING, calls the gated API
- `affirm()`: calls POST /api/v1/gate/confirm with confirmToken, transitions to POSTED
- `revise(newText)`: updates draftText, transitions back to DRAFT for re-editing
- `resubmit()`: transitions to EVALUATING with revised text
- `reset()`: clears all state back to DRAFT

Composes `GateService` (Sprint 5) internally. Multiple instances can coexist (one per artifact card).

### Layer 2: GateArtifactCardComponent

Shared visual component. The card IS the artifact.

**Inputs:**
- `placeholder`: textarea placeholder text
- `mutationType`: what kind of mutation this is (for gate evaluation)
- `contextMetadata`: EPR context for the mutation

**Outputs:**
- `posted`: emits when artifact successfully posts (with reach tier)
- `settled`: emits when gate settles (with settlement info)

**Internal:**
- Owns a `GateInteractionService` instance
- Renders five visual states via `@switch` on the state signal
- Contains the textarea (DRAFT), preview (EVALUATING/AFFIRM), reach badge (AFFIRM), dialogue slot (DIALOGUE), settlement display (SETTLED)
- `<ng-content select="[dialogue]">` slot for shells to position elohim dialogue differently

### Layer 3: GateCommentComponent (First Shell)

Wraps `GateArtifactCardComponent` for inline comment use.

- Renders in the comment section of a content page
- On POSTED, the card collapses into a normal comment entry in the list
- Dialogue renders below the card (default `<ng-content>` position)
- Minimal — mostly just the card with comment-specific context metadata

---

## Data Flow

```
GateArtifactCard (DRAFT)
  │ submit
  ▼
GateInteractionService.submit(text, mutationType, context)
  │ builds mutation_content JSON
  ▼
StorageApiService (gated POST to /api/v1/*)
  │
  ▼
Rust evaluate_gate() — queries 6 trust signals, routes through InferenceRouter
  │
  ▼
Response: { data, gate: GateEvaluationView }
  │
  ├─ 200/201 → GateService.handleGateResponse()
  │            → GateInteractionService → AFFIRM state
  │            → reach computed from trustContext.compositeTrust
  │
  ├─ 409     → handleGateError() → GateService paused
  │            → GateInteractionService → DIALOGUE state
  │            → pause prompt from gate.pausePrompt
  │
  └─ 403     → handleGateError() → GateService settled
               → GateInteractionService → SETTLED state
               → settlement link from gate.appealPath

On AFFIRM → "Affirm & Post" click:
  │
  GateInteractionService.affirm()
  │ POST /api/v1/gate/confirm with confirmToken
  ▼
  → POSTED state
```

---

## Not In Scope

- **Journal surface** — separate design, needs own route + sidebar layout + rich editing
- **Feedback modal** — follows from comment pattern, separate sprint
- **Inference sidecar content** — pause prompts currently from gate defaults, sidecar is Sprint 7
- **Reach persistence on posted content** — needs backend reach field on content nodes
- **Cooldown timer backend** — needs backend field, use client-side default for now
- **Rich text editing** — comment is plain text, journal will need it later

---

## Build Order

1. GateInteractionService (state machine + API composition)
2. GateArtifactCardComponent (five visual states)
3. GateCommentComponent (first shell)
4. Integration verification
