# SonarQube Code Duplication Analysis - elohim-app
## Complete Report

---

## Executive Summary

**Current State:**
- Duplicated Lines: 3,760 (6.6% of codebase)
- Duplicated Blocks: 26
- Duplicated Files: 18
- **Goal:** Reduce to below 3% (eliminate ~2,000 lines)

**Root Cause:**
The lamad pillar contains duplicate copies of models and services that should be imported from canonical locations (elohim, imagodei, qahal, shared). This happened because lamad grew to re-implement shared infrastructure rather than importing it.

**Key Finding:**
Four duplication patterns account for ~96% of all duplicated code:
1. trust-badge.model.ts (2 copies) - 1,031 lines
2. local-source-chain.service.ts (2 copies) - 1,164 lines
3. profile.model.ts (2 copies) - 784 lines
4. human-consent.model.ts (2 copies) - 647 lines

**Total:** ~3,626 lines (96.4% of duplication)

---

## Detailed Analysis

### 1. trust-badge.model.ts - HIGHEST PRIORITY
**Status:** 🔴 CRITICAL - Largest single duplication

**Files:**
- `/home/user/elohim/elohim-app/src/app/elohim/models/trust-badge.model.ts` (524 lines)
- `/home/user/elohim/elohim-app/src/app/lamad/models/trust-badge.model.ts` (507 lines)

**Analysis:**
- Files are 98% identical
- Only difference: Line 20 imports
  - elohim: Defines `ContentAttestationType` inline (lines 28-39)
  - lamad: Imports from `./content-attestation.model`
- Both export identical interfaces: TrustBadge, BadgeDisplay, TrustIndicator, etc.
- Both export identical functions: calculateTrustLevel(), generateTrustSummary(), etc.

**Why This Exists:**
- lamad needed trust badge functionality
- Instead of importing from elohim, created a duplicate copy
- Over time, both versions evolved slightly differently (import strategy)

**Evidence:**
```bash
$ diff -u elohim/models/trust-badge.model.ts lamad/models/trust-badge.model.ts | wc -l
22  # Only 22 lines differ out of 524!
```

**Shared Config Exists:**
- `/home/user/elohim/elohim-app/src/app/shared/models/trust-badge-config.ts` already imports from `@app/elohim/models/trust-badge.model`
- This confirms elohim version is canonical

**Impact:** 524 duplicated lines (13.9% of total duplication)

**Recommendation:**
1. Delete `/home/user/elohim/elohim-app/src/app/lamad/models/trust-badge.model.ts`
2. Update lamad imports to use `@app/elohim/models/trust-badge.model`
3. Move `ContentAttestationType` definition to elohim version (it's already in content-attestation.model)

---

### 2. local-source-chain.service.ts - HIGHEST PRIORITY
**Status:** 🔴 CRITICAL - Completely identical service

**Files:**
- `/home/user/elohim/elohim-app/src/app/elohim/services/local-source-chain.service.ts` (582 lines)
- `/home/user/elohim/elohim-app/src/app/lamad/services/local-source-chain.service.ts` (582 lines)

**Analysis:**
- Files are 99.8% identical
- Only difference: Line 12 import path
  - elohim: `from '../models/source-chain.model'`
  - lamad: `from '../models'`
- Service implements browser-based source chain using IndexedDB
- This is pure infrastructure, not pillar-specific

**Evidence:**
```bash
$ diff elohim/services/local-source-chain.service.ts lamad/services/local-source-chain.service.ts
12c12
< } from '../models/source-chain.model';
---
> } from '../models';
```

**Impact:** 582 duplicated lines (15.5% of total duplication)

**Recommendation:**
1. Move to `/home/user/elohim/elohim-app/src/app/shared/services/local-source-chain.service.ts`
2. Delete both elohim and lamad copies
3. Update all imports to use `@app/shared/services/local-source-chain.service`
4. Add to shared/services/index.ts barrel export

**Rationale:** Source chain is infrastructure used by all pillars, belongs in shared

---

### 3. profile.model.ts - HIGH PRIORITY
**Status:** 🟡 HIGH - Nearly identical models

**Files:**
- `/home/user/elohim/elohim-app/src/app/imagodei/models/profile.model.ts` (393 lines)
- `/home/user/elohim/elohim-app/src/app/lamad/models/profile.model.ts` (391 lines)

**Analysis:**
- Files are 99% identical
- Only differences: Import paths
  - imagodei: Uses relative paths `../../lamad/models/...`
  - lamad: Uses relative paths `./...` and `@app/elohim/...`
- Both define identical interfaces: HumanProfile, JourneyStats, CurrentFocus, etc.
- Profile is human-centered identity (Imago Dei pillar)

**Evidence:**
```bash
$ diff -u imagodei/models/profile.model.ts lamad/models/profile.model.ts | head -30
# Only import paths differ
```

**Impact:** 392 duplicated lines (10.4% of total duplication)

**Recommendation:**
1. Keep imagodei version as canonical (it's the Identity pillar)
2. Delete `/home/user/elohim/elohim-app/src/app/lamad/models/profile.model.ts`
3. Update lamad imports to use `@app/imagodei/models/profile.model`
4. Note: lamad/models/index.ts already re-exports from imagodei (line 222), so consumers won't break

**Alternative:** Move to @app/shared/models if profile is truly cross-pillar

---

### 4. human-consent.model.ts - MEDIUM PRIORITY
**Status:** 🟡 MEDIUM - Some structural differences

**Files:**
- `/home/user/elohim/elohim-app/src/app/qahal/models/human-consent.model.ts` (308 lines)
- `/home/user/elohim/elohim-app/src/app/lamad/models/human-consent.model.ts` (339 lines)

**Analysis:**
- Files are ~85% identical
- Key difference: Type definitions
  - qahal: Defines `IntimacyLevel` and `ConsentState` locally (lines 32-87)
  - lamad: Imports from `@app/elohim/models/protocol-core.model`
- lamad version is more aligned with protocol-core architecture
- Both define identical: HumanConsent interface, utility functions

**Evidence:**
```bash
$ diff qahal/models/human-consent.model.ts lamad/models/human-consent.model.ts | wc -l
50  # About 50 lines differ
```

**Protocol-Core Status:**
- `IntimacyLevel` and `ConsentState` ARE defined in protocol-core.model.ts
- They should NOT be redefined in qahal version

**Impact:** ~320 duplicated lines (8.5% of total duplication)

**Recommendation:**
1. Keep qahal version as canonical (it's the Community pillar - owns consent)
2. Update qahal to import IntimacyLevel/ConsentState from protocol-core
3. Delete `/home/user/elohim/elohim-app/src/app/lamad/models/human-consent.model.ts`
4. Update lamad to import from `@app/qahal/models/human-consent.model`

---

## Additional Findings

### 5. Barrel Export Complexity
**File:** `/home/user/elohim/elohim-app/src/app/lamad/models/index.ts` (293 lines)

**Issue:**
- Massive barrel export re-exporting from all pillars
- Lines 182-206: Re-exports trust-badge from @app/elohim
- Line 222: Re-exports profile from @app/imagodei
- This creates a facade but the actual files still exist in lamad

**Not a Duplication Issue Itself:**
- The barrel exports don't duplicate code
- They just re-export from other locations
- The problem is that lamad has BOTH the barrel export AND duplicate files

---

## Remaining Duplication (~140 lines)

After eliminating the 4 major patterns, ~140 lines remain. Likely sources:

1. **Test Fixtures** - Mock data duplicated across spec files
2. **Utility Functions** - Small helper functions repeated in different services
3. **Configuration Objects** - Similar config patterns in different pillars
4. **Index.ts Files** - Some barrel exports might have overlap

**Strategy:** Address these after the major 4 are resolved and re-run SonarQube to identify specifics.

---

## Implementation Roadmap

### Phase 1: Quick Wins - Services (30 minutes)
**Target:** -582 lines (15.5% reduction)

```bash
# 1. Move local-source-chain.service.ts to shared
mv elohim-app/src/app/elohim/services/local-source-chain.service.ts \
   elohim-app/src/app/shared/services/local-source-chain.service.ts

# 2. Delete lamad copy
rm elohim-app/src/app/lamad/services/local-source-chain.service.ts

# 3. Update imports (automated with IDE find/replace)
# Find: from '../services/local-source-chain.service'
# Replace: from '@app/shared/services/local-source-chain.service'

# 4. Add to shared/services/index.ts
echo "export * from './local-source-chain.service';" >> \
  elohim-app/src/app/shared/services/index.ts

# 5. Test
npm test

# 6. Commit
git add -A
git commit -m "Consolidate local-source-chain.service to @app/shared (-582 lines)"
```

**Expected Result:** Duplication drops from 6.6% → ~5.6%

---

### Phase 2: Model Consolidation - Trust Badge (45 minutes)
**Target:** -524 lines (13.9% reduction)

```bash
# 1. Verify elohim version is canonical
# (Already confirmed - shared/models/trust-badge-config.ts imports from it)

# 2. Delete lamad copy
rm elohim-app/src/app/lamad/models/trust-badge.model.ts

# 3. Update lamad/models/index.ts
# Change line 194: from './trust-badge.model' → from '@app/elohim/models/trust-badge.model'

# 4. Update any direct imports in lamad
# Find: from './trust-badge.model'
# Replace: from '@app/elohim/models/trust-badge.model'

# 5. Test
npm test

# 6. Commit
git add -A
git commit -m "Remove duplicate trust-badge.model from lamad, use @app/elohim (-524 lines)"
```

**Expected Result:** Duplication drops from ~5.6% → ~4.7%

---

### Phase 3: Model Consolidation - Profile (30 minutes)
**Target:** -392 lines (10.4% reduction)

```bash
# 1. Delete lamad copy
rm elohim-app/src/app/lamad/models/profile.model.ts

# 2. Verify lamad/models/index.ts already re-exports from imagodei
# Line 222: export * from '@app/imagodei/models/profile.model'; ✓

# 3. Update any direct imports in lamad components/services
# Find: from './models/profile.model'
# Replace: from '@app/imagodei/models/profile.model'

# 4. Test
npm test

# 5. Commit
git add -A
git commit -m "Remove duplicate profile.model from lamad, use @app/imagodei (-392 lines)"
```

**Expected Result:** Duplication drops from ~4.7% → ~4.0%

---

### Phase 4: Model Consolidation - Human Consent (45 minutes)
**Target:** -320 lines (8.5% reduction)

```bash
# 1. Update qahal version to import from protocol-core
# Edit qahal/models/human-consent.model.ts
# Add: import { IntimacyLevel, ConsentState } from '@app/elohim/models/protocol-core.model';
# Remove: local type definitions (lines 32-87)

# 2. Delete lamad copy
rm elohim-app/src/app/lamad/models/human-consent.model.ts

# 3. Update lamad/models/index.ts
# Lines 255-258: Change from './human-consent.model' → from '@app/qahal/models/human-consent.model'

# 4. Update any direct imports
# Find: from './human-consent.model'
# Replace: from '@app/qahal/models/human-consent.model'

# 5. Test
npm test

# 6. Commit
git add -A
git commit -m "Consolidate human-consent.model to @app/qahal (-320 lines)"
```

**Expected Result:** Duplication drops from ~4.0% → ~3.4%

---

### Phase 5: Final Cleanup (1-2 hours)
**Target:** -140 lines (remaining duplication)

```bash
# 1. Re-run SonarQube
npm run sonar  # or however you trigger SonarQube

# 2. Review new duplication report
# SonarQube will now show the remaining ~140 lines

# 3. Address case-by-case
# Likely: test fixtures, small utility functions, config objects

# 4. Commit incrementally
git commit -m "Final cleanup: eliminate remaining duplications"
```

**Expected Result:** Duplication drops from ~3.4% → <3.0% ✓

---

## Expected Results Timeline

| Phase | Action | Lines Saved | Cumulative | New % | Time |
|-------|--------|-------------|------------|-------|------|
| 0 | Current state | 0 | 0 | 6.6% | - |
| 1 | local-source-chain.service | 582 | 582 | 5.6% | 30m |
| 2 | trust-badge.model | 524 | 1,106 | 4.7% | 45m |
| 3 | profile.model | 392 | 1,498 | 4.0% | 30m |
| 4 | human-consent.model | 320 | 1,818 | 3.4% | 45m |
| 5 | Final cleanup | 140+ | 2,000+ | **<3.0%** ✓ | 1-2h |
| **TOTAL** | | **~2,000** | **2,000** | **<3.0%** | **~4h** |

---

## Risk Assessment & Mitigation

### Risks

1. **Breaking Changes in Tests**
   - Impact: Medium
   - Likelihood: High
   - Mitigation: Run test suite after each phase, fix incrementally

2. **Circular Dependencies**
   - Impact: High
   - Likelihood: Low
   - Mitigation: Use @app/* path aliases, check for cycles with madge or similar

3. **IDE Import Auto-Complete**
   - Impact: Low
   - Likelihood: Medium
   - Mitigation: Clear IDE cache, restart language server

4. **Build Cache Issues**
   - Impact: Low
   - Likelihood: Medium
   - Mitigation: Run `npm run clean` or `rm -rf .angular/cache`

### Mitigation Strategy

1. **Incremental Commits** - One phase at a time, commit after each
2. **Test Coverage** - Run full test suite after each change
3. **Branch Strategy** - Work in feature branch, can rollback easily
4. **Pair Review** - Have teammate review before merging
5. **Monitoring** - Watch SonarQube metrics after each phase

---

## Architecture Alignment

This refactoring aligns with the intended pillar architecture:

**Current (Wrong):**
```
elohim/models/trust-badge.model.ts  ← Canonical
lamad/models/trust-badge.model.ts   ← Duplicate (DELETE)
shared/models/trust-badge-config.ts ← Config (imports from elohim) ✓
```

**Target (Correct):**
```
elohim/models/trust-badge.model.ts  ← Canonical (protocol-core owns trust)
shared/models/trust-badge-config.ts ← Config (imports from elohim) ✓
lamad/models/index.ts → re-exports from @app/elohim ✓
```

**Pillar Ownership:**
- **elohim:** Protocol-core (trust, attestations, agents, source-chain)
- **imagodei:** Identity (profile, session-human)
- **qahal:** Community (consent, governance, affinity)
- **lamad:** Content (content-node, learning-path, exploration)
- **shared:** Cross-cutting concerns (services, configs)

---

## Success Criteria

✅ **Primary Goal:** Duplication < 3.0%
✅ **Secondary Goals:**
- All tests pass
- No new TypeScript errors
- Build succeeds
- Architecture aligns with pillar responsibilities
- Import paths use @app/* aliases consistently

---

## Next Steps

1. Review this report with team
2. Get approval for consolidation strategy
3. Create feature branch: `feature/eliminate-model-duplication`
4. Execute Phase 1-5 sequentially
5. Submit PR with SonarQube metrics comparison
6. Merge to main

---

## Files to Delete (Summary)

These 4 files will be deleted:
1. `/home/user/elohim/elohim-app/src/app/lamad/models/trust-badge.model.ts` (507 lines)
2. `/home/user/elohim/elohim-app/src/app/lamad/models/profile.model.ts` (391 lines)
3. `/home/user/elohim/elohim-app/src/app/lamad/models/human-consent.model.ts` (339 lines)
4. `/home/user/elohim/elohim-app/src/app/lamad/services/local-source-chain.service.ts` (582 lines)

**Total files deleted:** 4
**Total lines removed:** 1,819

One file will be moved:
1. `/home/user/elohim/elohim-app/src/app/elohim/services/local-source-chain.service.ts` → `/home/user/elohim/elohim-app/src/app/shared/services/local-source-chain.service.ts`

**Net reduction:** ~1,818 duplicate lines eliminated

---

## Appendix: File Locations

### Canonical Locations (Keep These)
```
/home/user/elohim/elohim-app/src/app/elohim/models/trust-badge.model.ts       ← Keep (protocol-core)
/home/user/elohim/elohim-app/src/app/imagodei/models/profile.model.ts         ← Keep (identity)
/home/user/elohim/elohim-app/src/app/qahal/models/human-consent.model.ts      ← Keep (community)
/home/user/elohim/elohim-app/src/app/shared/services/local-source-chain.service.ts ← Move here
```

### Duplicates (Delete These)
```
/home/user/elohim/elohim-app/src/app/lamad/models/trust-badge.model.ts        ← DELETE
/home/user/elohim/elohim-app/src/app/lamad/models/profile.model.ts            ← DELETE
/home/user/elohim/elohim-app/src/app/lamad/models/human-consent.model.ts      ← DELETE
/home/user/elohim/elohim-app/src/app/lamad/services/local-source-chain.service.ts ← DELETE
/home/user/elohim/elohim-app/src/app/elohim/services/local-source-chain.service.ts ← MOVE to shared
```

---

**Report Generated:** 2025-12-07
**Analysis Tool:** Manual code inspection + SonarQube API
**Codebase:** elohim-app (commit: 218eb8d)
**Analyst:** Claude Code AI Assistant
