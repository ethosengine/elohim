# SonarQube Issues Report - elohim-app

**Generated:** 2025-11-18 22:41:51  
**SonarQube Server:** https://sonarqube.ethosengine.com  
**Project:** elohim-app

## Executive Summary

- **Total Issues:** 30 open issues (out of 107 total)
- **Total Technical Debt:** 365 minutes (~6h 5m)

### Issues by Severity

| Severity | Count | Percentage |
|----------|-------|------------|
| MAJOR | 14 | 46.7% |
| MINOR | 16 | 53.3% |

### Issues by Type

| Type | Count | Percentage |
|------|-------|------------|
| CODE_SMELL | 25 | 83.3% |
| BUG | 5 | 16.7% |

### Issues by Clean Code Impact

| Software Quality | High | Medium | Low |
|-----------------|------|--------|-----|
| MAINTAINABILITY | 0 | 14 | 11 |
| RELIABILITY | 0 | 0 | 5 |

## Top Issues by Rule

### typescript:S2933 (14 occurrences)

- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Effort per fix:** 2min
- **Tags:** type-dependent, confusing
- **Message:** Member 'pathSubject' is never reassigned; mark it as `readonly`.

**Affected files:**
- `src/app/lamad/components/content-viewer/content-viewer.component.ts` (5 occurrences)
- `src/app/lamad/components/meaning-map/meaning-map.component.ts` (4 occurrences)
- `src/app/lamad/services/learning-path.service.ts` (2 occurrences)
- `src/app/lamad/services/affinity-tracking.service.ts` (2 occurrences)
- `src/app/lamad/components/lamad-home/lamad-home.component.ts` (1 occurrence)

### typescript:S6606 (7 occurrences)

- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Effort per fix:** 5min
- **Tags:** type-dependent, nullish-coalescing, es2020
- **Message:** Prefer using nullish coalescing operator (`??`) instead of a logical or (`||`), as it is a safer operator.

**Affected files:**
- `src/app/lamad/components/lamad-home/lamad-home.component.ts` (2 occurrences)
- `src/app/lamad/components/meaning-map/meaning-map.component.ts` (2 occurrences)
- `src/app/lamad/services/learning-path.service.ts` (1 occurrence)
- `src/app/lamad/adapters/document-node.adapter.ts` (1 occurrence)
- `src/app/lamad/services/affinity-tracking.service.ts` (1 occurrence)

### Web:MouseEventWithoutKeyboardEquivalentCheck (5 occurrences)

- **Severity:** MINOR
- **Type:** BUG
- **Effort per fix:** 5min
- **Tags:** accessibility
- **Message:** Add a 'onKeyPress|onKeyDown|onKeyUp' attribute to this <div> tag.

**Affected files:**
- `src/app/lamad/components/lamad-home/lamad-home.component.html` (2 occurrences)
- `src/app/lamad/components/meaning-map/meaning-map.component.html` (2 occurrences)
- `src/app/lamad/components/content-viewer/content-viewer.component.html` (1 occurrence)

### typescript:S4325 (3 occurrences)

- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Effort per fix:** 1min
- **Tags:** type-dependent, redundant
- **Message:** This assertion is unnecessary since it does not change the type of the expression.

**Affected files:**
- `src/app/lamad/components/content-viewer/content-viewer.component.ts` (2 occurrences)
- `src/app/lamad/components/meaning-map/meaning-map.component.ts` (1 occurrence)

### typescript:S1128 (1 occurrences)

- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Effort per fix:** 1min
- **Tags:** es2015, type-dependent, unused
- **Message:** Remove this unused import of 'Observable'.

**Affected files:**
- `src/app/lamad/services/learning-path.service.ts` (1 occurrence)


## Issues by File

### `src/app/lamad/components/content-viewer/content-viewer.component.ts` (7 issues)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 26 | S2933 | MAJOR | Member 'destroy$' is never reassigned; mark it as `readonly`. | 2min |
| 30 | S2933 | MAJOR | Member 'route: ActivatedRoute' is never reassigned; mark it as `readonly`. | 2min |
| 31 | S2933 | MAJOR | Member 'router: Router' is never reassigned; mark it as `readonly`. | 2min |
| 32 | S2933 | MAJOR | Member 'graphService: DocumentGraphService' is never reassigned; mark it as `rea... | 2min |
| 33 | S2933 | MAJOR | Member 'affinityService: AffinityTrackingService' is never reassigned; mark it a... | 2min |
| 256 | S4325 | MINOR | This assertion is unnecessary since it does not change the type of the expressio... | 1min |
| 276 | S4325 | MINOR | This assertion is unnecessary since it does not change the type of the expressio... | 1min |

### `src/app/lamad/components/meaning-map/meaning-map.component.ts` (7 issues)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 43 | S2933 | MAJOR | Member 'destroy$' is never reassigned; mark it as `readonly`. | 2min |
| 46 | S2933 | MAJOR | Member 'graphService: DocumentGraphService' is never reassigned; mark it as `rea... | 2min |
| 47 | S2933 | MAJOR | Member 'affinityService: AffinityTrackingService' is never reassigned; mark it a... | 2min |
| 48 | S2933 | MAJOR | Member 'router: Router' is never reassigned; mark it as `readonly`. | 2min |
| 74 | S4325 | MINOR | This assertion is unnecessary since it does not change the type of the expressio... | 1min |
| 118 | S6606 | MINOR | Prefer using nullish coalescing operator (`??`) instead of a logical or (`\|\|`)... | 5min |
| 119 | S6606 | MINOR | Prefer using nullish coalescing operator (`??`) instead of a logical or (`\|\|`)... | 5min |

### `src/app/lamad/services/learning-path.service.ts` (4 issues)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 2 | S1128 | MINOR | Remove this unused import of 'Observable'. | 1min |
| 22 | S2933 | MAJOR | Member 'pathSubject' is never reassigned; mark it as `readonly`. | 2min |
| 46 | S2933 | MAJOR | Member 'graphService: DocumentGraphService' is never reassigned; mark it as `rea... | 2min |
| 138 | S6606 | MINOR | Prefer using nullish coalescing operator (`??`) instead of a logical or (`\|\|`)... | 5min |

### `src/app/lamad/components/lamad-home/lamad-home.component.ts` (3 issues)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 35 | S2933 | MAJOR | Member 'destroy$' is never reassigned; mark it as `readonly`. | 2min |
| 264 | S6606 | MINOR | Prefer using nullish coalescing operator (`??`) instead of a logical or (`\|\|`)... | 5min |
| 295 | S6606 | MINOR | Prefer using nullish coalescing operator (`??`) instead of a logical or (`\|\|`)... | 5min |

### `src/app/lamad/services/affinity-tracking.service.ts` (3 issues)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 27 | S2933 | MAJOR | Member 'affinitySubject' is never reassigned; mark it as `readonly`. | 2min |
| 30 | S2933 | MAJOR | Member 'changeSubject' is never reassigned; mark it as `readonly`. | 2min |
| 46 | S6606 | MINOR | Prefer using nullish coalescing operator (`??`) instead of a ternary expression,... | 5min |

### `src/app/lamad/components/lamad-home/lamad-home.component.html` (2 issues)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 24 | MouseEventWithoutKeyboardEquivalentCheck | MINOR | Add a 'onKeyPress\|onKeyDown\|onKeyUp' attribute to this <div> tag. | 5min |
| 71 | MouseEventWithoutKeyboardEquivalentCheck | MINOR | Add a 'onKeyPress\|onKeyDown\|onKeyUp' attribute to this <div> tag. | 5min |

### `src/app/lamad/components/meaning-map/meaning-map.component.html` (2 issues)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 33 | MouseEventWithoutKeyboardEquivalentCheck | MINOR | Add a 'onKeyPress\|onKeyDown\|onKeyUp' attribute to this <div> tag. | 5min |
| 57 | MouseEventWithoutKeyboardEquivalentCheck | MINOR | Add a 'onKeyPress\|onKeyDown\|onKeyUp' attribute to this <div> tag. | 5min |

### `src/app/lamad/adapters/document-node.adapter.ts` (1 issue)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 53 | S6606 | MINOR | Prefer using nullish coalescing operator (`??`) instead of a logical or (`\|\|`)... | 5min |

### `src/app/lamad/components/content-viewer/content-viewer.component.html` (1 issue)

| Line | Rule | Severity | Message | Effort |
|------|------|----------|---------|--------|
| 118 | MouseEventWithoutKeyboardEquivalentCheck | MINOR | Add a 'onKeyPress\|onKeyDown\|onKeyUp' attribute to this <div> tag. | 5min |


## Detailed Issue List

### Issue #1: typescript:S6606

- **File:** `src/app/lamad/components/lamad-home/lamad-home.component.ts`
- **Line:** 264
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Prefer using nullish coalescing operator (`??`) instead of a logical or (`||`), as it is a safer operator.
- **Effort:** 5min
- **Author:** noreply@anthropic.com
- **Created:** 2025-11-18T22:10:53+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #2: typescript:S6606

- **File:** `src/app/lamad/components/lamad-home/lamad-home.component.ts`
- **Line:** 295
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Prefer using nullish coalescing operator (`??`) instead of a logical or (`||`), as it is a safer operator.
- **Effort:** 5min
- **Author:** noreply@anthropic.com
- **Created:** 2025-11-18T22:10:53+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #3: Web:MouseEventWithoutKeyboardEquivalentCheck

- **File:** `src/app/lamad/components/lamad-home/lamad-home.component.html`
- **Line:** 24
- **Severity:** MINOR
- **Type:** BUG
- **Status:** OPEN
- **Message:** Add a 'onKeyPress|onKeyDown|onKeyUp' attribute to this <div> tag.
- **Effort:** 5min
- **Author:** noreply@anthropic.com
- **Created:** 2025-11-18T22:04:03+0000
- **Quick Fix Available:** No

**Impacts:**
- RELIABILITY: LOW

---

### Issue #4: Web:MouseEventWithoutKeyboardEquivalentCheck

- **File:** `src/app/lamad/components/lamad-home/lamad-home.component.html`
- **Line:** 71
- **Severity:** MINOR
- **Type:** BUG
- **Status:** OPEN
- **Message:** Add a 'onKeyPress|onKeyDown|onKeyUp' attribute to this <div> tag.
- **Effort:** 5min
- **Author:** noreply@anthropic.com
- **Created:** 2025-11-18T22:04:03+0000
- **Quick Fix Available:** No

**Impacts:**
- RELIABILITY: LOW

---

### Issue #5: typescript:S1128

- **File:** `src/app/lamad/services/learning-path.service.ts`
- **Line:** 2
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Remove this unused import of 'Observable'.
- **Effort:** 1min
- **Author:** noreply@anthropic.com
- **Created:** 2025-11-18T20:14:41+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #6: typescript:S2933

- **File:** `src/app/lamad/services/learning-path.service.ts`
- **Line:** 22
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'pathSubject' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** noreply@anthropic.com
- **Created:** 2025-11-18T20:14:41+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #7: typescript:S2933

- **File:** `src/app/lamad/services/learning-path.service.ts`
- **Line:** 46
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'graphService: DocumentGraphService' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** noreply@anthropic.com
- **Created:** 2025-11-18T20:14:41+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #8: typescript:S6606

- **File:** `src/app/lamad/services/learning-path.service.ts`
- **Line:** 138
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Prefer using nullish coalescing operator (`??`) instead of a logical or (`||`), as it is a safer operator.
- **Effort:** 5min
- **Author:** noreply@anthropic.com
- **Created:** 2025-11-18T20:14:41+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #9: typescript:S6606

- **File:** `src/app/lamad/adapters/document-node.adapter.ts`
- **Line:** 53
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Prefer using nullish coalescing operator (`??`) instead of a logical or (`||`), as it is a safer operator.
- **Effort:** 5min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #10: Web:MouseEventWithoutKeyboardEquivalentCheck

- **File:** `src/app/lamad/components/content-viewer/content-viewer.component.html`
- **Line:** 118
- **Severity:** MINOR
- **Type:** BUG
- **Status:** OPEN
- **Message:** Add a 'onKeyPress|onKeyDown|onKeyUp' attribute to this <div> tag.
- **Effort:** 5min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** No

**Impacts:**
- RELIABILITY: LOW

---

### Issue #11: typescript:S2933

- **File:** `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- **Line:** 26
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'destroy$' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #12: typescript:S2933

- **File:** `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- **Line:** 30
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'route: ActivatedRoute' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #13: typescript:S2933

- **File:** `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- **Line:** 31
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'router: Router' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #14: typescript:S2933

- **File:** `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- **Line:** 32
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'graphService: DocumentGraphService' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #15: typescript:S2933

- **File:** `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- **Line:** 33
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'affinityService: AffinityTrackingService' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #16: typescript:S4325

- **File:** `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- **Line:** 256
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** This assertion is unnecessary since it does not change the type of the expression.
- **Effort:** 1min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #17: typescript:S4325

- **File:** `src/app/lamad/components/content-viewer/content-viewer.component.ts`
- **Line:** 276
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** This assertion is unnecessary since it does not change the type of the expression.
- **Effort:** 1min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #18: typescript:S2933

- **File:** `src/app/lamad/components/lamad-home/lamad-home.component.ts`
- **Line:** 35
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'destroy$' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #19: Web:MouseEventWithoutKeyboardEquivalentCheck

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.html`
- **Line:** 33
- **Severity:** MINOR
- **Type:** BUG
- **Status:** OPEN
- **Message:** Add a 'onKeyPress|onKeyDown|onKeyUp' attribute to this <div> tag.
- **Effort:** 5min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** No

**Impacts:**
- RELIABILITY: LOW

---

### Issue #20: Web:MouseEventWithoutKeyboardEquivalentCheck

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.html`
- **Line:** 57
- **Severity:** MINOR
- **Type:** BUG
- **Status:** OPEN
- **Message:** Add a 'onKeyPress|onKeyDown|onKeyUp' attribute to this <div> tag.
- **Effort:** 5min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** No

**Impacts:**
- RELIABILITY: LOW

---

### Issue #21: typescript:S2933

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.ts`
- **Line:** 43
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'destroy$' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #22: typescript:S2933

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.ts`
- **Line:** 46
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'graphService: DocumentGraphService' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #23: typescript:S2933

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.ts`
- **Line:** 47
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'affinityService: AffinityTrackingService' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #24: typescript:S2933

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.ts`
- **Line:** 48
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'router: Router' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #25: typescript:S4325

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.ts`
- **Line:** 74
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** This assertion is unnecessary since it does not change the type of the expression.
- **Effort:** 1min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #26: typescript:S6606

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.ts`
- **Line:** 118
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Prefer using nullish coalescing operator (`??`) instead of a logical or (`||`), as it is a safer operator.
- **Effort:** 5min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #27: typescript:S6606

- **File:** `src/app/lamad/components/meaning-map/meaning-map.component.ts`
- **Line:** 119
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Prefer using nullish coalescing operator (`??`) instead of a logical or (`||`), as it is a safer operator.
- **Effort:** 5min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---

### Issue #28: typescript:S2933

- **File:** `src/app/lamad/services/affinity-tracking.service.ts`
- **Line:** 27
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'affinitySubject' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #29: typescript:S2933

- **File:** `src/app/lamad/services/affinity-tracking.service.ts`
- **Line:** 30
- **Severity:** MAJOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Member 'changeSubject' is never reassigned; mark it as `readonly`.
- **Effort:** 2min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: MEDIUM

---

### Issue #30: typescript:S6606

- **File:** `src/app/lamad/services/affinity-tracking.service.ts`
- **Line:** 46
- **Severity:** MINOR
- **Type:** CODE_SMELL
- **Status:** OPEN
- **Message:** Prefer using nullish coalescing operator (`??`) instead of a ternary expression, as it is simpler to read.
- **Effort:** 5min
- **Author:** mbd06b+github@gmail.com
- **Created:** 2025-11-18T19:57:59+0000
- **Quick Fix Available:** Yes

**Impacts:**
- MAINTAINABILITY: LOW

---


