# Psephos Implementation Plan — All 5 Sprints

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build `@ethosengine/psephos` — the third Sophia pillar for governance ballot rendering — with 5 voting widgets, election hygiene system, web component distribution, and Angular wrapper.

**Architecture:** Psephos extends sophia-core's type system (adds `'governance'` purpose + `GovernanceResult` to `Recognition`). Input is `PsephosBallot` (the protocol-supplied ballot content). Output is standard `Recognition` via callback. Distribution mirrors sophia-element: React package → UMD web component → Angular plugin wrapper.

**Tech Stack:** React 18, TypeScript, sophia-core types, Rollup (UMD), Jest + @testing-library/react

**Design doc:** `genesis/plans/2026-03-15-psephos-governance-rendering-design.md`

---

## Sprint 1: Package Setup + Approval Widget (End-to-End)

The goal is a working approval ballot from React to Angular in the smallest possible vertical slice.

---

### Task 1.1: Extend sophia-core with governance types

**Files:**
- Modify: `sophia/packages/sophia-core/src/types.ts`
- Modify: `sophia/packages/sophia-core/src/scoring-strategy.ts`
- Modify: `sophia/packages/sophia-core/src/index.ts`
- Test: `sophia/packages/sophia-core/src/__tests__/types.test.ts` (create if needed)

**Step 1: Add governance to AssessmentPurpose**

In `sophia/packages/sophia-core/src/types.ts`, find:
```typescript
export type AssessmentPurpose = "mastery" | "discovery" | "reflection" | "invitation";
```

Change to:
```typescript
export type AssessmentPurpose = "mastery" | "discovery" | "reflection" | "invitation" | "governance";
```

**Step 2: Add GovernanceResult type**

In the same file, after `ReflectionResult`, add:

```typescript
/**
 * Result of a governance ballot submission.
 * Parallel to MasteryResult (learning) and ResonanceResult (assessment).
 */
export interface GovernanceResult {
    /** Which voting mechanism was used */
    mechanism: string;
    /** The voter's choices — one entry per option */
    ballots: BallotEntry[];
    /** Optional reasoning text */
    reasoning?: string;
    /** When the ballot was cast (ISO 8601) */
    timestamp: string;
    /** Proposal being voted on */
    proposalId: string;
}

/** A single entry in a governance ballot */
export interface BallotEntry {
    optionId: string;
    rank?: number | null;
    score?: number | null;
    dots?: number | null;
    approved?: boolean | null;
}
```

**Step 3: Add governance field to Recognition**

In `Recognition` interface, after `reflection?:`, add:
```typescript
    governance?: GovernanceResult;
```

**Step 4: Add type guard**

In `sophia/packages/sophia-core/src/scoring-strategy.ts`, after `hasReflectionResult`, add:

```typescript
export function hasGovernanceResult(
    recognition: Recognition,
): recognition is Recognition & {governance: GovernanceResult} {
    return recognition.governance !== undefined;
}
```

Import `GovernanceResult` at the top of the file.

**Step 5: Export new types**

In `sophia/packages/sophia-core/src/index.ts`, add exports:

```typescript
export type {GovernanceResult, BallotEntry} from "./types";
export {hasGovernanceResult} from "./scoring-strategy";
```

Also add factory functions:

```typescript
export function createGovernanceRecognition(
    momentId: string,
    governance: GovernanceResult,
    userInput: UserInputMap = {},
): Recognition {
    return {
        momentId,
        purpose: "governance",
        governance,
        userInput,
        timestamp: Date.now(),
    };
}

export function isGovernanceMoment(moment: Moment): boolean {
    return moment.purpose === "governance";
}
```

**Step 6: Write test**

Create `sophia/packages/sophia-core/src/__tests__/governance-types.test.ts`:

```typescript
import {
    createGovernanceRecognition,
    hasGovernanceResult,
    hasMasteryResult,
    type GovernanceResult,
    type Recognition,
} from "../index";

describe("governance types", () => {
    const governance: GovernanceResult = {
        mechanism: "approval",
        ballots: [
            {optionId: "opt-1", approved: true},
            {optionId: "opt-2", approved: false},
        ],
        timestamp: "2026-03-15T10:00:00Z",
        proposalId: "prop-123",
    };

    it("createGovernanceRecognition produces valid Recognition", () => {
        const rec = createGovernanceRecognition("ballot-1", governance);
        expect(rec.purpose).toBe("governance");
        expect(rec.governance).toBe(governance);
        expect(rec.momentId).toBe("ballot-1");
    });

    it("hasGovernanceResult returns true for governance recognition", () => {
        const rec = createGovernanceRecognition("ballot-1", governance);
        expect(hasGovernanceResult(rec)).toBe(true);
        expect(hasMasteryResult(rec)).toBe(false);
    });

    it("hasGovernanceResult returns false for mastery recognition", () => {
        const rec: Recognition = {
            momentId: "m-1",
            purpose: "mastery",
            mastery: {demonstrated: true, score: 1, total: 1},
            userInput: {},
        };
        expect(hasGovernanceResult(rec)).toBe(false);
    });
});
```

**Step 7: Run tests**

```bash
cd sophia && pnpm test -- --filter sophia-core
```

Expected: All tests pass including new governance type tests.

**Step 8: Commit**

```bash
git add sophia/packages/sophia-core/
git commit -m "feat(sophia-core): add governance purpose, GovernanceResult, and BallotEntry types"
```

---

### Task 1.2: Create psephos package skeleton

**Files:**
- Create: `sophia/packages/psephos/package.json`
- Create: `sophia/packages/psephos/tsconfig.json`
- Create: `sophia/packages/psephos/src/index.ts`
- Create: `sophia/packages/psephos/src/types.ts`

**Step 1: Create package.json**

Create `sophia/packages/psephos/package.json`:

```json
{
    "name": "@ethosengine/psephos",
    "version": "0.1.0",
    "description": "Governance ballot rendering — the third Sophia pillar",
    "module": "dist/es/index.js",
    "main": "dist/index.js",
    "source": "src/index.ts",
    "types": "dist/index.d.ts",
    "exports": {
        ".": {
            "import": "./dist/es/index.js",
            "require": "./dist/index.js",
            "types": "./dist/index.d.ts",
            "source": "./src/index.ts"
        },
        "./styles.css": "./dist/index.css"
    },
    "scripts": {
        "build": "rollup -c ../../config/build/rollup.config.js",
        "test": "jest"
    },
    "dependencies": {
        "@ethosengine/sophia-core": "workspace:*",
        "react": "catalog:peerDeps",
        "react-dom": "catalog:peerDeps"
    },
    "peerDependencies": {
        "react": "^18.0.0",
        "react-dom": "^18.0.0"
    },
    "sideEffects": false,
    "license": "UNLICENSED"
}
```

**Step 2: Create tsconfig.json**

Create `sophia/packages/psephos/tsconfig.json`:

```json
{
    "extends": "../../tsconfig-common.json",
    "compilerOptions": {
        "outDir": "dist",
        "rootDir": "src",
        "jsx": "react-jsx"
    },
    "include": ["src"]
}
```

**Step 3: Create types**

Create `sophia/packages/psephos/src/types.ts`:

```typescript
/**
 * Psephos types — the governance ballot input contract.
 *
 * PsephosBallot is to governance what Moment is to learning:
 * the protocol-supplied content that Psephos renders.
 */

/** Voting mechanism types supported by Psephos */
export type VotingMechanism =
    | "ranked-choice"
    | "approval"
    | "score-vote"
    | "dot-vote"
    | "consent";

/**
 * The governance equivalent of a Moment — what the protocol supplies.
 * The Angular wrapper transforms ProposalView + ProposalOptionView[] → PsephosBallot.
 */
export interface PsephosBallot {
    /** Unique ballot identifier (typically proposalId) */
    id: string;
    /** Always 'governance' — aligns with sophia-core AssessmentPurpose */
    purpose: "governance";
    /** The proposal being voted on */
    proposal: PsephosProposal;
    /** The options to vote on — supplied by the protocol */
    options: PsephosOption[];
    /** Which voting mechanism to render */
    mechanism: VotingMechanism;
    /** Mechanism-specific configuration */
    config: PsephosConfig;
    /** Election hygiene rules */
    hygiene: ElectionHygiene;
    /** Optional: existing ballot for review/amendment */
    previousBallot?: import("@ethosengine/sophia-core").BallotEntry[];
}

/** Proposal metadata for display */
export interface PsephosProposal {
    id: string;
    title: string;
    description: string;
    proposalType: string;
}

/** A single voting option */
export interface PsephosOption {
    id: string;
    label: string;
    description: string;
    /** Original position before randomization */
    position: number;
    /** Who proposed this option */
    source?: string;
    sourceJustification?: string;
}

/** Mechanism-specific configuration */
export interface PsephosConfig {
    /** score-vote: minimum score value */
    scoreMin?: number;
    /** score-vote: maximum score value */
    scoreMax?: number;
    /** dot-vote: total dots budget per voter */
    dotsPerVoter?: number;
    /** Required percentage of eligible voters */
    quorumPercentage?: number;
    /** Threshold for passage (e.g. 0.5 for majority) */
    passageThreshold?: number;
}

/**
 * Election hygiene configuration.
 * Every widget receives this. Defaults vary by mechanism.
 */
export interface ElectionHygiene {
    /** Shuffle option display order to prevent position bias */
    randomizeOrder: boolean;
    /** Seed for reproducible shuffle (proposalId + humanId) */
    randomSeed?: string;
    /** CSS constraints for visual parity between options */
    equalVisualWeight: boolean;
    /** Require text justification for votes */
    requireReasoning: boolean;
    /** Minimum characters for reasoning (default: 50 for blocks) */
    reasoningMinLength?: number;
    /** Only show tally after submission */
    showResultsAfterVote: boolean;
    /** Show confirmation interstitial before submit */
    confirmBeforeSubmit: boolean;
    /** Don't show "N people have voted" before voting */
    hideVoterCount: boolean;
}

/** Default election hygiene settings per mechanism */
export const DEFAULT_HYGIENE: Record<VotingMechanism, ElectionHygiene> = {
    "ranked-choice": {
        randomizeOrder: true,
        equalVisualWeight: true,
        requireReasoning: false,
        showResultsAfterVote: true,
        confirmBeforeSubmit: true,
        hideVoterCount: true,
    },
    approval: {
        randomizeOrder: true,
        equalVisualWeight: true,
        requireReasoning: false,
        showResultsAfterVote: true,
        confirmBeforeSubmit: false,
        hideVoterCount: true,
    },
    "score-vote": {
        randomizeOrder: true,
        equalVisualWeight: true,
        requireReasoning: false,
        showResultsAfterVote: true,
        confirmBeforeSubmit: true,
        hideVoterCount: false,
    },
    "dot-vote": {
        randomizeOrder: true,
        equalVisualWeight: true,
        requireReasoning: false,
        showResultsAfterVote: true,
        confirmBeforeSubmit: true,
        hideVoterCount: false,
    },
    consent: {
        randomizeOrder: false,
        equalVisualWeight: true,
        requireReasoning: false, // blocks require reasoning, but not all votes
        reasoningMinLength: 50,
        showResultsAfterVote: true,
        confirmBeforeSubmit: true,
        hideVoterCount: true,
    },
};

/**
 * Internal user input state tracked by widgets.
 * Maps option IDs to their current interaction state.
 */
export interface BallotUserInput {
    [optionId: string]: {
        rank?: number;
        score?: number;
        dots?: number;
        approved?: boolean;
    };
    /** Global reasoning text */
    reasoning?: string;
}
```

**Step 4: Create index.ts**

Create `sophia/packages/psephos/src/index.ts`:

```typescript
// Types
export type {
    PsephosBallot,
    PsephosProposal,
    PsephosOption,
    PsephosConfig,
    ElectionHygiene,
    VotingMechanism,
    BallotUserInput,
} from "./types";
export {DEFAULT_HYGIENE} from "./types";

// Re-export governance types from sophia-core for convenience
export type {GovernanceResult, BallotEntry} from "@ethosengine/sophia-core";
export {hasGovernanceResult} from "@ethosengine/sophia-core";

// GovernanceScoringStrategy will be registered here in Task 1.3
```

**Step 5: Verify workspace resolution**

```bash
cd sophia && pnpm install
```

Expected: pnpm resolves `@ethosengine/psephos` as a workspace package.

**Step 6: Commit**

```bash
git add sophia/packages/psephos/
git commit -m "feat(psephos): create package skeleton with ballot types and election hygiene defaults"
```

---

### Task 1.3: Implement GovernanceScoringStrategy

**Files:**
- Create: `sophia/packages/psephos/src/governance-strategy.ts`
- Create: `sophia/packages/psephos/src/__tests__/governance-strategy.test.ts`
- Modify: `sophia/packages/psephos/src/index.ts`

**Step 1: Write the failing test**

Create `sophia/packages/psephos/src/__tests__/governance-strategy.test.ts`:

```typescript
import type {Recognition} from "@ethosengine/sophia-core";
import {
    hasGovernanceResult,
    getScoringStrategy,
} from "@ethosengine/sophia-core";

// Import registers the strategy
import "../index";

import type {PsephosBallot, BallotUserInput} from "../types";

const makeBallot = (
    mechanism: PsephosBallot["mechanism"],
    options: PsephosBallot["options"],
    config: PsephosBallot["config"] = {},
): PsephosBallot => ({
    id: "ballot-1",
    purpose: "governance",
    proposal: {
        id: "prop-1",
        title: "Test Proposal",
        description: "A test",
        proposalType: "advice",
    },
    options,
    mechanism,
    config,
    hygiene: {
        randomizeOrder: false,
        equalVisualWeight: true,
        requireReasoning: false,
        showResultsAfterVote: true,
        confirmBeforeSubmit: false,
        hideVoterCount: true,
    },
});

describe("GovernanceScoringStrategy", () => {
    it("registers as 'governance' in the scoring registry", () => {
        const strategy = getScoringStrategy("governance");
        expect(strategy).toBeDefined();
        expect(strategy!.id).toBe("governance");
    });

    describe("getEmptyWidgetIds (approval)", () => {
        const ballot = makeBallot("approval", [
            {id: "opt-1", label: "A", description: "", position: 0},
            {id: "opt-2", label: "B", description: "", position: 1},
        ]);

        it("returns all option IDs when nothing approved", () => {
            const strategy = getScoringStrategy("governance")!;
            const empty = strategy.getEmptyWidgetIds(
                ballot as any,
                {},
                "en",
            );
            expect(empty).toContain("opt-1");
            expect(empty).toContain("opt-2");
        });

        it("returns empty array when at least one option approved", () => {
            const strategy = getScoringStrategy("governance")!;
            const input = {"opt-1": {approved: true}} as any;
            const empty = strategy.getEmptyWidgetIds(
                ballot as any,
                input,
                "en",
            );
            expect(empty).toHaveLength(0);
        });
    });

    describe("getEmptyWidgetIds (score-vote)", () => {
        const ballot = makeBallot(
            "score-vote",
            [
                {id: "opt-1", label: "A", description: "", position: 0},
                {id: "opt-2", label: "B", description: "", position: 1},
            ],
            {scoreMin: 1, scoreMax: 10},
        );

        it("returns unscored option IDs", () => {
            const strategy = getScoringStrategy("governance")!;
            const input = {"opt-1": {score: 5}} as any;
            const empty = strategy.getEmptyWidgetIds(
                ballot as any,
                input,
                "en",
            );
            expect(empty).toEqual(["opt-2"]);
        });
    });

    describe("recognize", () => {
        it("produces Recognition with governance result", () => {
            const ballot = makeBallot("approval", [
                {id: "opt-1", label: "A", description: "", position: 0},
                {id: "opt-2", label: "B", description: "", position: 1},
            ]);
            const input = {
                "opt-1": {approved: true},
                "opt-2": {approved: false},
            } as any;

            const strategy = getScoringStrategy("governance")!;
            const rec: Recognition = strategy.recognize(
                ballot as any,
                input,
                "en",
            );

            expect(rec.purpose).toBe("governance");
            expect(hasGovernanceResult(rec)).toBe(true);
            if (hasGovernanceResult(rec)) {
                expect(rec.governance.mechanism).toBe("approval");
                expect(rec.governance.proposalId).toBe("prop-1");
                expect(rec.governance.ballots).toHaveLength(2);
                expect(rec.governance.ballots[0]).toEqual({
                    optionId: "opt-1",
                    approved: true,
                });
            }
        });
    });
});
```

**Step 2: Write the implementation**

Create `sophia/packages/psephos/src/governance-strategy.ts`:

```typescript
import type {
    ScoringStrategy,
    Recognition,
    UserInputMap,
} from "@ethosengine/sophia-core";
import type {BallotEntry} from "@ethosengine/sophia-core";
import type {PsephosBallot, BallotUserInput} from "./types";

/**
 * Build BallotEntry[] from user input state.
 * Each option gets one entry with only the relevant field populated.
 */
function buildBallotEntries(
    ballot: PsephosBallot,
    userInput: BallotUserInput,
): BallotEntry[] {
    return ballot.options.map((opt) => {
        const input = userInput[opt.id] ?? {};
        const entry: BallotEntry = {optionId: opt.id};

        switch (ballot.mechanism) {
            case "ranked-choice":
                entry.rank = input.rank ?? null;
                break;
            case "approval":
            case "consent":
                entry.approved = input.approved ?? null;
                break;
            case "score-vote":
                entry.score = input.score ?? null;
                break;
            case "dot-vote":
                entry.dots = input.dots ?? null;
                break;
        }

        return entry;
    });
}

/**
 * Determine which options are "empty" (not yet voted on).
 * Rules vary by mechanism — see design doc Widget Specifications.
 */
function getEmptyOptionIds(
    ballot: PsephosBallot,
    userInput: BallotUserInput,
): string[] {
    const {mechanism, options} = ballot;

    switch (mechanism) {
        case "approval": {
            // At least 1 option must be approved
            const hasAny = options.some(
                (opt) => userInput[opt.id]?.approved === true,
            );
            return hasAny ? [] : options.map((o) => o.id);
        }
        case "ranked-choice": {
            // At least 1 option must be ranked
            const hasAny = options.some(
                (opt) => userInput[opt.id]?.rank != null,
            );
            return hasAny ? [] : options.map((o) => o.id);
        }
        case "score-vote": {
            // ALL options must be scored
            return options
                .filter((opt) => userInput[opt.id]?.score == null)
                .map((o) => o.id);
        }
        case "dot-vote": {
            // Zero dots is valid (intentional non-allocation)
            // But voter must have interacted (at least visited)
            return [];
        }
        case "consent": {
            // Must choose consent or block
            const hasChoice = options.some(
                (opt) => userInput[opt.id]?.approved != null,
            );
            return hasChoice ? [] : options.map((o) => o.id);
        }
        default:
            return options.map((o) => o.id);
    }
}

/**
 * GovernanceScoringStrategy — registered as 'governance' in sophia-core's registry.
 *
 * Validates ballot completeness and produces Recognition with GovernanceResult.
 * Parallel to perseus-score (mastery) and psyche-survey (discovery/reflection).
 */
export const GovernanceScoringStrategy: ScoringStrategy = {
    id: "governance",
    name: "Governance Ballot",

    getEmptyWidgetIds(
        content: unknown,
        userInput: UserInputMap,
        _locale: string,
    ): ReadonlyArray<string> {
        const ballot = content as PsephosBallot;
        return getEmptyOptionIds(ballot, userInput as BallotUserInput);
    },

    recognize(
        content: unknown,
        userInput: UserInputMap,
        _locale: string,
    ): Recognition {
        const ballot = content as PsephosBallot;
        const input = userInput as BallotUserInput;

        return {
            momentId: ballot.id,
            purpose: "governance",
            governance: {
                mechanism: ballot.mechanism,
                ballots: buildBallotEntries(ballot, input),
                reasoning: input.reasoning as string | undefined,
                timestamp: new Date().toISOString(),
                proposalId: ballot.proposal.id,
            },
            userInput,
            timestamp: Date.now(),
        };
    },
};
```

**Step 3: Register in index.ts**

Update `sophia/packages/psephos/src/index.ts` — add at the top:

```typescript
import {registerScoringStrategy} from "@ethosengine/sophia-core";
import {GovernanceScoringStrategy} from "./governance-strategy";

// Auto-register on import (same pattern as psyche-survey)
registerScoringStrategy(GovernanceScoringStrategy);

export {GovernanceScoringStrategy} from "./governance-strategy";
```

**Step 4: Run tests**

```bash
cd sophia && pnpm test -- --filter psephos
```

Expected: All governance-strategy tests pass.

**Step 5: Commit**

```bash
git add sophia/packages/psephos/
git commit -m "feat(psephos): implement GovernanceScoringStrategy with ballot validation"
```

---

### Task 1.4: Approval widget (React component)

**Files:**
- Create: `sophia/packages/psephos/src/widgets/approval.tsx`
- Create: `sophia/packages/psephos/src/__tests__/approval.test.tsx`

**Step 1: Write the failing test**

Create `sophia/packages/psephos/src/__tests__/approval.test.tsx`:

```tsx
import {render, screen, fireEvent} from "@testing-library/react";
import React from "react";

import {ApprovalWidget} from "../widgets/approval";
import type {PsephosOption, ElectionHygiene} from "../types";
import {DEFAULT_HYGIENE} from "../types";

const options: PsephosOption[] = [
    {id: "opt-1", label: "Option Alpha", description: "First option", position: 0},
    {id: "opt-2", label: "Option Beta", description: "Second option", position: 1},
    {id: "opt-3", label: "Option Gamma", description: "Third option", position: 2},
];

const hygiene: ElectionHygiene = {
    ...DEFAULT_HYGIENE.approval,
    randomizeOrder: false, // Deterministic for tests
};

describe("ApprovalWidget", () => {
    it("renders all options as checkboxes", () => {
        render(
            <ApprovalWidget
                options={options}
                hygiene={hygiene}
                onChange={jest.fn()}
            />,
        );
        expect(screen.getByLabelText("Option Alpha")).toBeInTheDocument();
        expect(screen.getByLabelText("Option Beta")).toBeInTheDocument();
        expect(screen.getByLabelText("Option Gamma")).toBeInTheDocument();
    });

    it("shows 'you may select multiple' instruction", () => {
        render(
            <ApprovalWidget
                options={options}
                hygiene={hygiene}
                onChange={jest.fn()}
            />,
        );
        expect(screen.getByText(/select multiple/i)).toBeInTheDocument();
    });

    it("no options are pre-checked", () => {
        render(
            <ApprovalWidget
                options={options}
                hygiene={hygiene}
                onChange={jest.fn()}
            />,
        );
        const checkboxes = screen.getAllByRole("checkbox");
        checkboxes.forEach((cb) => expect(cb).not.toBeChecked());
    });

    it("calls onChange with updated state when option toggled", () => {
        const onChange = jest.fn();
        render(
            <ApprovalWidget
                options={options}
                hygiene={hygiene}
                onChange={onChange}
            />,
        );

        fireEvent.click(screen.getByLabelText("Option Alpha"));

        expect(onChange).toHaveBeenCalledWith(
            expect.objectContaining({
                "opt-1": {approved: true},
            }),
        );
    });

    it("allows toggling multiple options", () => {
        const onChange = jest.fn();
        render(
            <ApprovalWidget
                options={options}
                hygiene={hygiene}
                onChange={onChange}
            />,
        );

        fireEvent.click(screen.getByLabelText("Option Alpha"));
        fireEvent.click(screen.getByLabelText("Option Gamma"));

        // Last call should have both approved
        const lastCall = onChange.mock.calls[onChange.mock.calls.length - 1][0];
        expect(lastCall["opt-1"]?.approved).toBe(true);
        expect(lastCall["opt-3"]?.approved).toBe(true);
    });

    it("shows descriptions when provided", () => {
        render(
            <ApprovalWidget
                options={options}
                hygiene={hygiene}
                onChange={jest.fn()}
            />,
        );
        expect(screen.getByText("First option")).toBeInTheDocument();
    });

    it("restores previous state when initialState provided", () => {
        render(
            <ApprovalWidget
                options={options}
                hygiene={hygiene}
                onChange={jest.fn()}
                initialState={{"opt-2": {approved: true}}}
            />,
        );
        expect(screen.getByLabelText("Option Beta")).toBeChecked();
        expect(screen.getByLabelText("Option Alpha")).not.toBeChecked();
    });
});
```

**Step 2: Write the implementation**

Create `sophia/packages/psephos/src/widgets/approval.tsx`:

```tsx
import React, {useCallback, useState} from "react";

import type {PsephosOption, ElectionHygiene, BallotUserInput} from "../types";

export interface ApprovalWidgetProps {
    options: PsephosOption[];
    hygiene: ElectionHygiene;
    onChange: (state: BallotUserInput) => void;
    initialState?: BallotUserInput;
}

export function ApprovalWidget({
    options,
    hygiene,
    onChange,
    initialState = {},
}: ApprovalWidgetProps): React.ReactElement {
    const [state, setState] = useState<BallotUserInput>(initialState);

    const handleToggle = useCallback(
        (optionId: string) => {
            setState((prev) => {
                const current = prev[optionId]?.approved ?? false;
                const next: BallotUserInput = {
                    ...prev,
                    [optionId]: {approved: !current},
                };
                onChange(next);
                return next;
            });
        },
        [onChange],
    );

    return (
        <div
            className="psephos-approval"
            role="group"
            aria-label="Approval vote"
        >
            <p className="psephos-instruction">
                You may select multiple options. Check each option you approve.
            </p>
            <div className="psephos-options">
                {options.map((opt) => (
                    <label
                        key={opt.id}
                        className="psephos-option"
                        style={
                            hygiene.equalVisualWeight
                                ? {display: "flex", alignItems: "flex-start"}
                                : undefined
                        }
                    >
                        <input
                            type="checkbox"
                            checked={state[opt.id]?.approved === true}
                            onChange={() => handleToggle(opt.id)}
                            aria-label={opt.label}
                        />
                        <div className="psephos-option-content">
                            <span className="psephos-option-label">
                                {opt.label}
                            </span>
                            {opt.description && (
                                <span className="psephos-option-description">
                                    {opt.description}
                                </span>
                            )}
                        </div>
                    </label>
                ))}
            </div>
        </div>
    );
}
```

**Step 3: Run tests**

```bash
cd sophia && pnpm test -- --filter psephos -- approval
```

Expected: All approval widget tests pass.

**Step 4: Commit**

```bash
git add sophia/packages/psephos/src/widgets/ sophia/packages/psephos/src/__tests__/approval.test.tsx
git commit -m "feat(psephos): add approval voting widget with tests"
```

---

### Task 1.5: Psephos renderer (main component)

**Files:**
- Create: `sophia/packages/psephos/src/psephos-renderer.tsx`
- Create: `sophia/packages/psephos/src/__tests__/psephos-renderer.test.tsx`
- Modify: `sophia/packages/psephos/src/index.ts`

**Step 1: Write the failing test**

Create `sophia/packages/psephos/src/__tests__/psephos-renderer.test.tsx`:

```tsx
import {render, screen, fireEvent} from "@testing-library/react";
import React from "react";

import {PsephosRenderer} from "../psephos-renderer";
import type {PsephosBallot} from "../types";
import {DEFAULT_HYGIENE} from "../types";

const approvalBallot: PsephosBallot = {
    id: "ballot-1",
    purpose: "governance",
    proposal: {
        id: "prop-1",
        title: "Test Proposal",
        description: "Should we do the thing?",
        proposalType: "advice",
    },
    options: [
        {id: "opt-1", label: "Yes", description: "Proceed", position: 0},
        {id: "opt-2", label: "No", description: "Don't proceed", position: 1},
    ],
    mechanism: "approval",
    config: {},
    hygiene: {...DEFAULT_HYGIENE.approval, randomizeOrder: false},
};

describe("PsephosRenderer", () => {
    it("renders proposal title and description", () => {
        render(
            <PsephosRenderer
                ballot={approvalBallot}
                onRecognition={jest.fn()}
            />,
        );
        expect(screen.getByText("Test Proposal")).toBeInTheDocument();
        expect(
            screen.getByText("Should we do the thing?"),
        ).toBeInTheDocument();
    });

    it("renders approval widget for approval mechanism", () => {
        render(
            <PsephosRenderer
                ballot={approvalBallot}
                onRecognition={jest.fn()}
            />,
        );
        expect(screen.getByLabelText("Yes")).toBeInTheDocument();
        expect(screen.getByLabelText("No")).toBeInTheDocument();
    });

    it("shows submit button", () => {
        render(
            <PsephosRenderer
                ballot={approvalBallot}
                onRecognition={jest.fn()}
            />,
        );
        expect(
            screen.getByRole("button", {name: /submit/i}),
        ).toBeInTheDocument();
    });

    it("submit button is disabled until ballot is valid", () => {
        render(
            <PsephosRenderer
                ballot={approvalBallot}
                onRecognition={jest.fn()}
            />,
        );
        expect(screen.getByRole("button", {name: /submit/i})).toBeDisabled();
    });

    it("submit button enables after selecting an option", () => {
        render(
            <PsephosRenderer
                ballot={approvalBallot}
                onRecognition={jest.fn()}
            />,
        );
        fireEvent.click(screen.getByLabelText("Yes"));
        expect(
            screen.getByRole("button", {name: /submit/i}),
        ).not.toBeDisabled();
    });

    it("fires onRecognition with GovernanceResult on submit", () => {
        const onRecognition = jest.fn();
        render(
            <PsephosRenderer
                ballot={approvalBallot}
                onRecognition={onRecognition}
            />,
        );

        fireEvent.click(screen.getByLabelText("Yes"));
        fireEvent.click(screen.getByRole("button", {name: /submit/i}));

        expect(onRecognition).toHaveBeenCalledTimes(1);
        const rec = onRecognition.mock.calls[0][0];
        expect(rec.purpose).toBe("governance");
        expect(rec.governance.mechanism).toBe("approval");
        expect(rec.governance.proposalId).toBe("prop-1");
        expect(rec.governance.ballots).toEqual(
            expect.arrayContaining([
                expect.objectContaining({optionId: "opt-1", approved: true}),
            ]),
        );
    });
});
```

**Step 2: Write the implementation**

Create `sophia/packages/psephos/src/psephos-renderer.tsx`:

```tsx
import React, {useCallback, useMemo, useRef, useState} from "react";

import type {Recognition, UserInputMap} from "@ethosengine/sophia-core";
import {getScoringStrategy} from "@ethosengine/sophia-core";

import type {PsephosBallot, BallotUserInput} from "./types";
import {ApprovalWidget} from "./widgets/approval";

export interface PsephosRendererProps {
    ballot: PsephosBallot;
    onRecognition?: (recognition: Recognition) => void;
    onAnswerChange?: (hasAnswer: boolean) => void;
    reviewMode?: boolean;
}

export function PsephosRenderer({
    ballot,
    onRecognition,
    onAnswerChange,
    reviewMode = false,
}: PsephosRendererProps): React.ReactElement {
    const [userInput, setUserInput] = useState<BallotUserInput>({});
    const prevHasAnswerRef = useRef(false);

    const strategy = useMemo(
        () => getScoringStrategy("governance"),
        [],
    );

    const emptyIds = useMemo(() => {
        if (!strategy) return ballot.options.map((o) => o.id);
        return strategy.getEmptyWidgetIds(
            ballot as any,
            userInput as UserInputMap,
            "en",
        );
    }, [strategy, ballot, userInput]);

    const isComplete = emptyIds.length === 0;

    const handleChange = useCallback(
        (state: BallotUserInput) => {
            setUserInput(state);
            const hasAnswer =
                Object.keys(state).length > 0 &&
                Object.values(state).some(
                    (v) =>
                        typeof v === "object" &&
                        v !== null &&
                        Object.values(v).some((x) => x != null),
                );
            if (hasAnswer !== prevHasAnswerRef.current) {
                prevHasAnswerRef.current = hasAnswer;
                onAnswerChange?.(hasAnswer);
            }
        },
        [onAnswerChange],
    );

    const handleSubmit = useCallback(() => {
        if (!strategy || !isComplete || reviewMode) return;

        const recognition = strategy.recognize(
            ballot as any,
            userInput as UserInputMap,
            "en",
        );
        onRecognition?.(recognition);
    }, [strategy, ballot, userInput, isComplete, reviewMode, onRecognition]);

    const renderWidget = () => {
        switch (ballot.mechanism) {
            case "approval":
                return (
                    <ApprovalWidget
                        options={ballot.options}
                        hygiene={ballot.hygiene}
                        onChange={handleChange}
                        initialState={
                            ballot.previousBallot
                                ? toBallotUserInput(ballot.previousBallot)
                                : undefined
                        }
                    />
                );
            // Other mechanisms added in Sprints 2-4
            default:
                return (
                    <div className="psephos-unsupported">
                        Mechanism "{ballot.mechanism}" not yet implemented.
                    </div>
                );
        }
    };

    return (
        <div className="psephos-ballot" data-mechanism={ballot.mechanism}>
            <div className="psephos-proposal">
                <h3 className="psephos-proposal-title">
                    {ballot.proposal.title}
                </h3>
                <p className="psephos-proposal-description">
                    {ballot.proposal.description}
                </p>
            </div>

            {renderWidget()}

            {!reviewMode && (
                <div className="psephos-actions">
                    <button
                        className="psephos-submit"
                        type="button"
                        disabled={!isComplete}
                        onClick={handleSubmit}
                        aria-label="Submit ballot"
                    >
                        Submit Ballot
                    </button>
                </div>
            )}
        </div>
    );
}

/** Convert BallotEntry[] from previousBallot to widget input state */
function toBallotUserInput(
    entries: import("@ethosengine/sophia-core").BallotEntry[],
): BallotUserInput {
    const state: BallotUserInput = {};
    for (const entry of entries) {
        state[entry.optionId] = {
            rank: entry.rank ?? undefined,
            score: entry.score ?? undefined,
            dots: entry.dots ?? undefined,
            approved: entry.approved ?? undefined,
        };
    }
    return state;
}
```

**Step 3: Export from index.ts**

Add to `sophia/packages/psephos/src/index.ts`:

```typescript
export {PsephosRenderer} from "./psephos-renderer";
export type {PsephosRendererProps} from "./psephos-renderer";
```

**Step 4: Run tests**

```bash
cd sophia && pnpm test -- --filter psephos
```

Expected: All tests pass.

**Step 5: Commit**

```bash
git add sophia/packages/psephos/
git commit -m "feat(psephos): add PsephosRenderer with submit validation and approval support"
```

---

### Task 1.6: Create psephos-element web component

**Files:**
- Create: `sophia/packages/psephos-element/package.json`
- Create: `sophia/packages/psephos-element/tsconfig.json`
- Create: `sophia/packages/psephos-element/src/psephos-ballot.tsx`
- Create: `sophia/packages/psephos-element/src/register.ts`
- Create: `sophia/packages/psephos-element/src/index.ts`
- Create: `sophia/packages/psephos-element/src/umd-entry.ts`
- Create: `sophia/packages/psephos-element/rollup.config.mjs`

**Step 1: Create package.json**

Create `sophia/packages/psephos-element/package.json`:

```json
{
    "name": "@ethosengine/psephos-element",
    "version": "0.1.0",
    "description": "Web component wrapper for Psephos governance ballot rendering",
    "module": "dist/es/index.js",
    "main": "dist/index.js",
    "source": "src/index.ts",
    "types": "dist/index.d.ts",
    "exports": {
        ".": {
            "import": "./dist/es/index.js",
            "require": "./dist/index.js",
            "types": "./dist/index.d.ts",
            "source": "./src/index.ts"
        },
        "./register": {
            "import": "./dist/es/register.js",
            "require": "./dist/register.js",
            "source": "./src/register.ts"
        },
        "./umd": "./dist/psephos-element.umd.js",
        "./styles.css": "./dist/index.css"
    },
    "scripts": {
        "build": "rollup -c ../../config/build/rollup.config.js",
        "build:umd": "rollup -c rollup.config.mjs"
    },
    "dependencies": {
        "@ethosengine/psephos": "workspace:*",
        "@ethosengine/sophia-core": "workspace:*",
        "react": "catalog:peerDeps",
        "react-dom": "catalog:peerDeps"
    },
    "sideEffects": [
        "./src/register.ts",
        "./dist/register.js",
        "./dist/es/register.js"
    ],
    "license": "UNLICENSED"
}
```

**Step 2: Create tsconfig.json**

Create `sophia/packages/psephos-element/tsconfig.json`:

```json
{
    "extends": "../../tsconfig-common.json",
    "compilerOptions": {
        "outDir": "dist",
        "rootDir": "src",
        "jsx": "react-jsx"
    },
    "include": ["src"]
}
```

**Step 3: Create the web component**

Create `sophia/packages/psephos-element/src/psephos-ballot.tsx`:

```tsx
import React from "react";
import {createRoot, type Root} from "react-dom/client";

import type {Recognition} from "@ethosengine/sophia-core";
import {PsephosRenderer} from "@ethosengine/psephos";
import type {PsephosBallot} from "@ethosengine/psephos";

/**
 * <psephos-ballot> custom element.
 *
 * Wraps PsephosRenderer (React) as a web component.
 * Follows the same pattern as <sophia-question> in sophia-element.
 *
 * Properties:
 *   - ballot: PsephosBallot (set via JS, not attribute)
 *   - reviewMode: boolean
 *   - onRecognition: callback
 *   - onAnswerChange: callback
 *
 * Methods:
 *   - getRecognition(): Recognition | null (pull-based)
 */
export class PsephosBallotElement extends HTMLElement {
    private root: Root | null = null;
    private container: HTMLDivElement | null = null;

    private _ballot: PsephosBallot | null = null;
    private _reviewMode = false;
    private _lastRecognition: Recognition | null = null;

    /** Callback fired when voter submits ballot */
    onRecognition: ((recognition: Recognition) => void) | null = null;

    /** Callback fired when ballot state changes */
    onAnswerChange: ((hasAnswer: boolean) => void) | null = null;

    connectedCallback(): void {
        // Shadow DOM for style encapsulation
        const shadow = this.attachShadow({mode: "open"});
        this.container = document.createElement("div");
        this.container.className = "psephos-root";
        shadow.appendChild(this.container);
        this.root = createRoot(this.container);
        this.render();
    }

    disconnectedCallback(): void {
        if (this.root) {
            this.root.unmount();
            this.root = null;
        }
    }

    get ballot(): PsephosBallot | null {
        return this._ballot;
    }

    set ballot(value: PsephosBallot | null) {
        this._ballot = value;
        this._lastRecognition = null;
        this.render();
    }

    get reviewMode(): boolean {
        return this._reviewMode;
    }

    set reviewMode(value: boolean) {
        this._reviewMode = value;
        this.render();
    }

    /** Pull-based: get the last Recognition produced */
    getRecognition(): Recognition | null {
        return this._lastRecognition;
    }

    private handleRecognition = (recognition: Recognition): void => {
        this._lastRecognition = recognition;
        this.onRecognition?.(recognition);

        // Also dispatch a DOM event for non-JS consumers
        this.dispatchEvent(
            new CustomEvent("recognition", {
                detail: recognition,
                bubbles: true,
                composed: true,
            }),
        );
    };

    private handleAnswerChange = (hasAnswer: boolean): void => {
        this.onAnswerChange?.(hasAnswer);
    };

    private render(): void {
        if (!this.root) return;

        if (!this._ballot) {
            this.root.render(null);
            return;
        }

        this.root.render(
            <PsephosRenderer
                ballot={this._ballot}
                onRecognition={this.handleRecognition}
                onAnswerChange={this.handleAnswerChange}
                reviewMode={this._reviewMode}
            />,
        );
    }
}
```

**Step 4: Create register.ts**

Create `sophia/packages/psephos-element/src/register.ts`:

```typescript
export const PSEPHOS_BALLOT_TAG = "psephos-ballot";

export function isPsephosElementRegistered(): boolean {
    return (
        typeof customElements !== "undefined" &&
        customElements.get(PSEPHOS_BALLOT_TAG) !== undefined
    );
}

export function registerPsephosElement(): void {
    if (isPsephosElementRegistered()) return;
    if (typeof customElements === "undefined") return;

    const {PsephosBallotElement} = require("./psephos-ballot");
    customElements.define(PSEPHOS_BALLOT_TAG, PsephosBallotElement);
}

// Auto-register on import
registerPsephosElement();
```

**Step 5: Create index.ts**

Create `sophia/packages/psephos-element/src/index.ts`:

```typescript
export {PsephosBallotElement} from "./psephos-ballot";
export {
    PSEPHOS_BALLOT_TAG,
    isPsephosElementRegistered,
    registerPsephosElement,
} from "./register";

// Re-export types consumers need
export type {
    PsephosBallot,
    PsephosOption,
    PsephosConfig,
    ElectionHygiene,
    VotingMechanism,
} from "@ethosengine/psephos";
export type {Recognition} from "@ethosengine/sophia-core";
```

**Step 6: Create umd-entry.ts**

Create `sophia/packages/psephos-element/src/umd-entry.ts`:

```typescript
import React from "react";
import ReactDOM from "react-dom";

// Expose React globally for UMD
(window as any).React = React;
(window as any).ReactDOM = ReactDOM;

// Auto-register the custom element
import "./register";

// Re-export everything
export * from "./index";
export {React, ReactDOM};
```

**Step 7: Create rollup.config.mjs**

Create `sophia/packages/psephos-element/rollup.config.mjs`:

```javascript
import resolve from "@rollup/plugin-node-resolve";
import commonjs from "@rollup/plugin-commonjs";
import replace from "@rollup/plugin-replace";
import swc from "@nicolo-ribaudo/rollup-plugin-swc";
import terser from "@rollup/plugin-terser";
import postcss from "rollup-plugin-postcss";

export default {
    input: "src/umd-entry.ts",
    output: {
        file: "dist/psephos-element.umd.js",
        format: "umd",
        name: "PsephosElement",
        banner: `if(typeof process==='undefined'){globalThis.process={env:{NODE_ENV:'production'}};}`,
        sourcemap: true,
    },
    plugins: [
        replace({
            preventAssignment: true,
            values: {
                "process.env.NODE_ENV": JSON.stringify("production"),
            },
        }),
        postcss({
            extract: "psephos-element.css",
            minimize: true,
        }),
        swc({
            minify: false,
            jsc: {
                parser: {syntax: "typescript", tsx: true},
                transform: {react: {runtime: "automatic"}},
                target: "es2020",
            },
        }),
        resolve({browser: true, preferBuiltins: false}),
        commonjs(),
        terser(),
    ],
};
```

**Step 8: Install and verify build**

```bash
cd sophia && pnpm install && pnpm build -- --filter psephos-element
```

**Step 9: Commit**

```bash
git add sophia/packages/psephos-element/
git commit -m "feat(psephos-element): create web component wrapper with UMD build"
```

---

### Task 1.7: Add build scripts to sophia root

**Files:**
- Modify: `sophia/package.json` (if needed — verify `pnpm build` picks up new packages)
- Modify: `sophia/tsconfig.json` (add path mapping for psephos packages)

**Step 1: Add path mappings**

In `sophia/tsconfig.json`, add to `compilerOptions.paths`:

```json
"@ethosengine/psephos": ["packages/psephos/src/index.ts"],
"@ethosengine/psephos-element": ["packages/psephos-element/src/index.ts"]
```

**Step 2: Add build:umd script for psephos-element**

In `sophia/package.json`, check if `build:umd` script exists. If it only builds sophia-element, update to also build psephos-element:

```json
"build:umd": "pnpm --filter @ethosengine/sophia-element build:umd && pnpm --filter @ethosengine/psephos-element build:umd"
```

**Step 3: Verify full build**

```bash
cd sophia && pnpm build && pnpm build:umd
```

Expected: Both sophia-element.umd.js and psephos-element.umd.js are produced.

**Step 4: Run all tests**

```bash
cd sophia && pnpm test
```

Expected: All existing tests still pass + new psephos tests pass.

**Step 5: Commit**

```bash
git add sophia/tsconfig.json sophia/package.json
git commit -m "build(sophia): add psephos to workspace path mappings and UMD build"
```

---

### Task 1.8: Create psephos-plugin Angular wrapper

**Files:**
- Create: `app/elohim-library/projects/psephos-plugin/package.json`
- Create: `app/elohim-library/projects/psephos-plugin/src/lib/psephos-ballot.component.ts`
- Create: `app/elohim-library/projects/psephos-plugin/src/public-api.ts`
- Create: `app/elohim-library/projects/psephos-plugin/src/lib/psephos-element-loader.ts`

This follows the exact same pattern as sophia-plugin and sophia-element-loader.

**Step 1: Create package.json**

Create `app/elohim-library/projects/psephos-plugin/package.json`:

```json
{
    "name": "@elohim/psephos-plugin",
    "version": "0.1.0",
    "description": "Angular wrapper for Psephos governance ballot web component",
    "main": "src/public-api.ts",
    "license": "UNLICENSED"
}
```

**Step 2: Create the element loader**

Create `app/elohim-library/projects/psephos-plugin/src/lib/psephos-element-loader.ts`:

```typescript
/**
 * Lazy loader for the <psephos-ballot> web component.
 * Mirrors sophia-element-loader.ts pattern.
 */

const PSEPHOS_ELEMENT_TAG = "psephos-ballot";
const CACHE_BUST = Date.now();

let isRegistered = false;
let loadPromise: Promise<void> | null = null;

function loadScript(url: string): Promise<void> {
    return new Promise((resolve, reject) => {
        const script = document.createElement("script");
        script.src = url;
        script.onload = () => resolve();
        script.onerror = () =>
            reject(new Error(`Failed to load script: ${url}`));
        document.head.appendChild(script);
    });
}

function loadCSS(url: string): void {
    if (document.querySelector(`link[href="${url}"]`)) return;
    const link = document.createElement("link");
    link.rel = "stylesheet";
    link.href = url;
    document.head.appendChild(link);
}

export async function registerPsephosElement(): Promise<void> {
    if (isRegistered || customElements.get(PSEPHOS_ELEMENT_TAG)) {
        isRegistered = true;
        return;
    }

    if (loadPromise) return loadPromise;

    loadPromise = (async () => {
        // Load CSS
        loadCSS(`/assets/psephos-plugin/psephos-element.css`);

        // Ensure React is loaded (shared with sophia-element)
        if (!(window as any).React) {
            await loadScript(
                `/assets/react/react.production.min.js?v=${CACHE_BUST}`,
            );
            await loadScript(
                `/assets/react/react-dom.production.min.js?v=${CACHE_BUST}`,
            );
        }

        // Load psephos UMD bundle
        await loadScript(
            `/assets/psephos-plugin/psephos-element.umd.js?v=${CACHE_BUST}`,
        );

        // Verify registration
        if (!customElements.get(PSEPHOS_ELEMENT_TAG)) {
            throw new Error(
                "psephos-ballot element not registered after loading bundle",
            );
        }

        isRegistered = true;
    })();

    return loadPromise;
}

export interface PsephosBallotElement extends HTMLElement {
    ballot: any | null;
    reviewMode: boolean;
    onRecognition: ((recognition: any) => void) | null;
    onAnswerChange: ((hasAnswer: boolean) => void) | null;
    getRecognition(): any | null;
}

export function getPsephosElement(
    container: HTMLElement,
): PsephosBallotElement | null {
    return container.querySelector(
        PSEPHOS_ELEMENT_TAG,
    ) as PsephosBallotElement | null;
}
```

**Step 3: Create the Angular wrapper component**

Create `app/elohim-library/projects/psephos-plugin/src/lib/psephos-ballot.component.ts`:

```typescript
import {
    AfterViewInit,
    Component,
    ElementRef,
    EventEmitter,
    Input,
    OnChanges,
    OnDestroy,
    Output,
    SimpleChanges,
    ViewChild,
    CUSTOM_ELEMENTS_SCHEMA,
} from "@angular/core";

import {
    registerPsephosElement,
    getPsephosElement,
    type PsephosBallotElement,
} from "./psephos-element-loader";

@Component({
    selector: "app-psephos-ballot",
    standalone: true,
    schemas: [CUSTOM_ELEMENTS_SCHEMA],
    template: `
        <div class="psephos-wrapper" #container>
            <psephos-ballot></psephos-ballot>
        </div>
    `,
    styles: [
        `
            .psephos-wrapper {
                width: 100%;
            }
        `,
    ],
})
export class PsephosBallotComponent
    implements AfterViewInit, OnChanges, OnDestroy
{
    @Input() ballot: any | null = null;
    @Input() reviewMode = false;

    @Output() recognized = new EventEmitter<any>();
    @Output() answerChanged = new EventEmitter<boolean>();

    @ViewChild("container") container!: ElementRef<HTMLElement>;

    private element: PsephosBallotElement | null = null;
    private ready = false;

    async ngAfterViewInit(): Promise<void> {
        await registerPsephosElement();
        this.element = getPsephosElement(this.container.nativeElement);

        if (this.element) {
            this.element.onRecognition = (rec) => this.recognized.emit(rec);
            this.element.onAnswerChange = (has) =>
                this.answerChanged.emit(has);

            // Apply initial inputs
            if (this.ballot) this.element.ballot = this.ballot;
            this.element.reviewMode = this.reviewMode;
        }

        this.ready = true;
    }

    ngOnChanges(changes: SimpleChanges): void {
        if (!this.ready || !this.element) return;

        if (changes["ballot"]) {
            this.element.ballot = this.ballot;
        }
        if (changes["reviewMode"]) {
            this.element.reviewMode = this.reviewMode;
        }
    }

    ngOnDestroy(): void {
        if (this.element) {
            this.element.onRecognition = null;
            this.element.onAnswerChange = null;
        }
    }

    getRecognition(): any | null {
        return this.element?.getRecognition() ?? null;
    }
}
```

**Step 4: Create public API**

Create `app/elohim-library/projects/psephos-plugin/src/public-api.ts`:

```typescript
export {PsephosBallotComponent} from "./lib/psephos-ballot.component";
export {registerPsephosElement} from "./lib/psephos-element-loader";
```

**Step 5: Commit**

```bash
git add app/elohim-library/projects/psephos-plugin/
git commit -m "feat(psephos-plugin): create Angular wrapper for <psephos-ballot> web component"
```

---

### Task 1.9: Asset setup and prebuild check

**Files:**
- Create: `app/elohim-app/src/assets/psephos-plugin/` (directory)
- Modify: prebuild script (if one exists) to check for psephos-element UMD

**Step 1: Create assets directory**

```bash
mkdir -p app/elohim-app/src/assets/psephos-plugin
```

**Step 2: Add copy script**

After building psephos-element, the UMD bundle needs to be copied:

```bash
cp sophia/packages/psephos-element/dist/psephos-element.umd.js app/elohim-app/src/assets/psephos-plugin/
cp sophia/packages/psephos-element/dist/psephos-element.css app/elohim-app/src/assets/psephos-plugin/ 2>/dev/null || true
```

**Step 3: Add .gitkeep** to keep directory in git:

```bash
touch app/elohim-app/src/assets/psephos-plugin/.gitkeep
```

**Step 4: Commit**

```bash
git add app/elohim-app/src/assets/psephos-plugin/
git commit -m "build: create psephos-plugin assets directory for UMD bundle"
```

---

## Sprint 2: Ranked-Choice Widget

---

### Task 2.1: Ranked-choice widget

**Files:**
- Create: `sophia/packages/psephos/src/widgets/ranked-choice.tsx`
- Create: `sophia/packages/psephos/src/__tests__/ranked-choice.test.tsx`

**Interaction:** Drag-to-rank or click-to-assign rank number per option.

**Key behaviors:**
- Options rendered in order (hygiene randomization applied by renderer)
- Two zones: "Ranked" (ordered list) and "Not Ranked" (unranked options)
- Click option to move between zones; drag to reorder within ranked zone
- Partial ranking allowed (rank your top N)
- No duplicate ranks

**Test cases:**
- Renders all options in "Not Ranked" zone initially
- Moving option to ranked zone assigns rank 1
- Moving second option ranks it after first
- Removing from ranked zone clears rank
- Reordering updates ranks
- At least 1 option must be ranked for validity
- Restores previous state

**Implementation notes:**
- Use HTML5 drag-and-drop API with keyboard fallback (arrow keys to reorder)
- `role="listbox"` with `aria-roledescription="ranking"` for accessibility
- Equal-height option cards (hygiene)

**Commit:** `feat(psephos): add ranked-choice widget with drag-to-rank interaction`

---

### Task 2.2: Wire ranked-choice into PsephosRenderer

**Files:**
- Modify: `sophia/packages/psephos/src/psephos-renderer.tsx`

Add `case "ranked-choice":` to the `renderWidget()` switch, importing and rendering `RankedChoiceWidget`.

**Commit:** `feat(psephos): wire ranked-choice widget into renderer`

---

### Task 2.3: IRV result visualization (post-vote display)

**Files:**
- Create: `sophia/packages/psephos/src/widgets/irv-result.tsx`
- Create: `sophia/packages/psephos/src/__tests__/irv-result.test.tsx`

**When ballot.hygiene.showResultsAfterVote is true**, after submission the renderer shows tally results. For ranked-choice, this is a round-by-round IRV elimination visualization:

- Horizontal stacked bar per round
- Eliminated option highlighted per round
- Winner highlighted with final percentage
- "Your ranking was: 1. X, 2. Y, 3. Z" summary

This component receives `TallyResult` (from the API response, passed back through the wrapper). Design the component to receive tally data as a prop — it does NOT call the API itself.

**Commit:** `feat(psephos): add IRV round-by-round result visualization`

---

## Sprint 3: Score-Vote + Dot-Vote Widgets

---

### Task 3.1: Score-vote widget

**Files:**
- Create: `sophia/packages/psephos/src/widgets/score-vote.tsx`
- Create: `sophia/packages/psephos/src/__tests__/score-vote.test.tsx`

**Interaction:** Slider or number input per option within `[scoreMin, scoreMax]`.

**Key behaviors:**
- All sliders start at midpoint (NOT min — prevents anchoring at zero)
- But midpoint is NOT counted as a vote until explicitly set
- Requires explicit interaction with each slider (click/drag/type)
- Score labels at endpoints ("1 = Strongly Oppose", "10 = Strongly Support")
- Equal-width slider tracks
- `getEmptyWidgetIds` returns IDs of unscored options

**Test cases:**
- Renders slider per option with labels
- Sliders start at visual midpoint but aren't "set"
- Clicking slider marks it as voted
- All options must be scored for validity
- Score within [min, max] range enforced

**Commit:** `feat(psephos): add score-vote widget with anti-anchoring midpoint`

---

### Task 3.2: Dot-vote widget

**Files:**
- Create: `sophia/packages/psephos/src/widgets/dot-vote.tsx`
- Create: `sophia/packages/psephos/src/__tests__/dot-vote.test.tsx`

**Interaction:** Increment/decrement buttons per option. Budget display: "N dots remaining."

**Key behaviors:**
- Budget constraint enforced visually (can't exceed `dotsPerVoter`)
- Zero dots is valid (intentional non-allocation)
- No negative dot counts
- Equal visual weight per option row
- ARIA live region for budget updates ("3 dots remaining")

**Test cases:**
- Shows "N dots remaining" label
- Increment adds dot, decrements budget
- Can't exceed budget
- Can't go below 0
- Zero allocation is valid (complete ballot)
- Budget label updates as dots allocated

**Commit:** `feat(psephos): add dot-vote widget with budget constraint`

---

### Task 3.3: Wire both into PsephosRenderer

**Files:**
- Modify: `sophia/packages/psephos/src/psephos-renderer.tsx`

Add `case "score-vote":` and `case "dot-vote":` to `renderWidget()`.

**Commit:** `feat(psephos): wire score-vote and dot-vote into renderer`

---

### Task 3.4: Result visualizations for score + dot

**Files:**
- Create: `sophia/packages/psephos/src/widgets/score-result.tsx`
- Create: `sophia/packages/psephos/src/widgets/dot-result.tsx`

Score result: mean score per option with distribution spread bar.
Dot result: dot distribution visualization (stacked dots or bar chart).

**Commit:** `feat(psephos): add score-vote and dot-vote result visualizations`

---

## Sprint 4: Consent Widget

---

### Task 4.1: Consent widget

**Files:**
- Create: `sophia/packages/psephos/src/widgets/consent.tsx`
- Create: `sophia/packages/psephos/src/__tests__/consent.test.tsx`

**Interaction:** Two primary buttons: **Consent** (green) and **Block** (amber, not red).

**Key behaviors:**
- Equal size and visual weight for both buttons
- Block requires reasoning text (minimum 50 characters from hygiene config)
- Consent optionally allows reasoning
- Clear explanation text: "Blocking does not veto — it triggers a facilitated conversation"
- Don't show current vote counts before submission (hygiene)

**Test cases:**
- Renders consent and block buttons with equal visual weight
- Selecting block shows reasoning textarea
- Block requires reasoning with minimum character count
- Consent is valid without reasoning
- Shows facilitation explanation text
- BallotEntry has `approved: true` for consent, `approved: false` for block

**Implementation notes:**
- The consent widget renders exactly 2 options (consent/block), not N options
- `options` array from ballot maps to: options[0] = the proposal itself
- The widget presents the binary consent/block choice

**Commit:** `feat(psephos): add consent widget with block reasoning requirement`

---

### Task 4.2: Wire consent into renderer + result view

**Files:**
- Modify: `sophia/packages/psephos/src/psephos-renderer.tsx`
- Create: `sophia/packages/psephos/src/widgets/consent-result.tsx`

Consent result display:
- Consent count vs Block count
- If blocked: "Escalation in progress — the elohim will engage with concerns"
- Block reasoning visible to all (transparency)

**Commit:** `feat(psephos): wire consent widget and escalation result view`

---

## Sprint 5: Election Hygiene System

---

### Task 5.1: Seeded randomization

**Files:**
- Create: `sophia/packages/psephos/src/hygiene/randomize-options.ts`
- Create: `sophia/packages/psephos/src/__tests__/randomize-options.test.ts`

**Implementation:** Fisher-Yates shuffle with seeded PRNG (proposalId + humanId produces same order per voter per proposal, but different voters see different orders).

**Test cases:**
- Same seed produces same order
- Different seeds produce different orders
- Distribution is uniform over many runs (chi-squared test)
- All options appear exactly once

**Commit:** `feat(psephos): add seeded Fisher-Yates option randomization`

---

### Task 5.2: Equal visual weight CSS

**Files:**
- Create: `sophia/packages/psephos/src/hygiene/equal-weight.ts`

CSS constraints applied when `hygiene.equalVisualWeight` is true:
- All option cards same height (CSS grid with `auto-fill` + `minmax`)
- Same font size, weight, color for all labels
- Same padding, border treatment
- No option pre-highlighted or visually emphasized

Export as a set of CSS class names or inline styles applied by widgets.

**Commit:** `feat(psephos): add equal visual weight CSS hygiene constraints`

---

### Task 5.3: Confirmation step

**Files:**
- Create: `sophia/packages/psephos/src/hygiene/confirmation-step.tsx`
- Create: `sophia/packages/psephos/src/__tests__/confirmation-step.test.tsx`

When `hygiene.confirmBeforeSubmit` is true, show an interstitial after clicking submit:

- "You chose: [summary of selections]. Submit?"
- Confirm / Go Back buttons
- Summary format varies by mechanism:
  - Approval: "You approved: X, Y"
  - Ranked-choice: "Your ranking: 1. X, 2. Y, 3. Z"
  - Score: "Your scores: X=7, Y=3"
  - Dot: "Your allocation: X=5 dots, Y=3 dots"
  - Consent: "You chose to [consent/block]"

**Test cases:**
- Shows summary matching user's selections
- Confirm triggers onRecognition
- Go Back returns to ballot without submitting

**Commit:** `feat(psephos): add confirmation interstitial with mechanism-specific summary`

---

### Task 5.4: Integrate hygiene into PsephosRenderer

**Files:**
- Modify: `sophia/packages/psephos/src/psephos-renderer.tsx`

Apply hygiene system:
1. Randomize options before passing to widget (when `hygiene.randomizeOrder`)
2. Apply equal-weight CSS classes (when `hygiene.equalVisualWeight`)
3. Insert confirmation step between submit click and onRecognition (when `hygiene.confirmBeforeSubmit`)
4. Hide voter count (when `hygiene.hideVoterCount`)

**Commit:** `feat(psephos): integrate election hygiene system into renderer`

---

### Task 5.5: Full integration test

**Files:**
- Create: `sophia/packages/psephos/src/__tests__/integration.test.tsx`

End-to-end test: create a PsephosBallot → render PsephosRenderer → interact → verify Recognition output for each mechanism.

**Commit:** `test(psephos): add full integration tests for all 5 mechanisms`

---

### Task 5.6: Build and verify UMD bundle

```bash
cd sophia && pnpm build && pnpm build:umd
ls -la sophia/packages/psephos-element/dist/psephos-element.umd.js
```

Verify bundle size is reasonable (< 500KB gzipped — React is the bulk).

Copy to elohim-app assets:
```bash
cp sophia/packages/psephos-element/dist/psephos-element.umd.js app/elohim-app/src/assets/psephos-plugin/
cp sophia/packages/psephos-element/dist/psephos-element.css app/elohim-app/src/assets/psephos-plugin/ 2>/dev/null || true
```

**Commit:** `build(psephos): verify UMD bundle and copy to elohim-app assets`

---

## Summary

| Sprint | Tasks | Delivers |
|--------|-------|----------|
| 1 | 1.1-1.9 | sophia-core governance types + psephos package + approval widget + web component + Angular wrapper |
| 2 | 2.1-2.3 | Ranked-choice widget + IRV result visualization |
| 3 | 3.1-3.4 | Score-vote + dot-vote widgets + result visualizations |
| 4 | 4.1-4.2 | Consent widget + block escalation messaging |
| 5 | 5.1-5.6 | Election hygiene system (randomization, equal weight, confirmation) + integration tests |

Each sprint produces a working UMD bundle that elohim-app can consume.
