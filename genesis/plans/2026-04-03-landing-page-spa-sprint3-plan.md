# Landing Page as SPA ContentNode — Sprint 3

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Package the Elohim Protocol public landing page as a `spa-bundle` ContentNode, making the protocol's front door a protocol-delivered artifact — served by doorway through the same extraction cache as any other content.

**Architecture:** A lightweight static site (Vite + vanilla HTML/CSS/JS) in `genesis/landing/` builds from `genesis/docs/content/elohim-protocol/manifesto.md` and the domain epics. Builds to a ZIP, uploads as blob to storage, and is served by doorway when `ROOT_APP_SLUG=protocol-landing`. The protocol toolbar (Sprint 2) overlays it.

**Tech Stack:** Vite (static build), vanilla HTML/CSS/JS, marked (markdown rendering at build time), pnpm, existing CI blob upload pattern

**Design:** `genesis/plans/2026-04-02-doorway-spa-as-blob-design.md`

**Depends on:** Sprint 2 (protocol toolbar — the landing page is served via full-page delivery with toolbar), doorway SPA-as-blob infrastructure (root_app.rs — already implemented)

**Existing infrastructure:**
- `ROOT_APP_SLUG` env var → doorway root app resolution (root_app.rs)
- Extraction cache in MongoDB (apps.rs handler)
- `lamad-spa.json` seed node pattern (`genesis/data/lamad/content/lamad-spa.json`)
- `spa-bundle` content format in protocol schema and lamad manifest
- `stageSpaBlob` CI pattern conceptualized in design doc Section 5
- Manifesto: `genesis/docs/content/elohim-protocol/manifesto.md`
- Epics: `governance/epic.md`, `social_medium/epic.md`, `autonomous_entity/epic.md`, `global-orchestra.md`

---

## File Structure

| File | Responsibility |
|------|----------------|
| `genesis/landing/package.json` | **NEW** — Vite project for landing page build |
| `genesis/landing/vite.config.ts` | **NEW** — Vite config: static build, markdown plugin |
| `genesis/landing/index.html` | **NEW** — Landing page entry point (hero, manifesto summary, pillars, CTA) |
| `genesis/landing/src/main.ts` | **NEW** — Minimal JS: protocol stats fetch, smooth scroll, toolbar interaction |
| `genesis/landing/src/style.css` | **NEW** — Landing page styles |
| `genesis/landing/scripts/build-and-zip.sh` | **NEW** — Build, ZIP dist, compute SHA256 |
| `genesis/landing/scripts/render-content.ts` | **NEW** — Build-time markdown → HTML extraction from genesis docs |
| `genesis/data/lamad/content/protocol-landing.json` | **NEW** — Seed content node |
| `genesis/a2o/features/delivery/landing-page.feature` | **NEW** — BDD scenarios |

---

### Task 1: Write a2o scenarios for the landing page

**Files:**
- Create: `genesis/a2o/features/delivery/landing-page.feature`

- [ ] **Step 1: Write the scenario file**

```gherkin
@delivery @landing-page @protocol
Feature: Protocol Landing Page as SPA ContentNode
  As a visitor to elohim.host
  I want to see a fast, informative landing page
  That is itself delivered as a protocol content node

  Background:
    Given doorway is configured with ROOT_APP_SLUG "protocol-landing"
    And the protocol-landing SPA blob is extracted in the cache

  # --- Page Content ---

  Scenario: Landing page loads with hero section
    When I visit "/"
    Then I see a hero section with the protocol's vision statement
    And the page loads in under 1 second (no framework overhead)

  Scenario: Landing page shows manifesto summary
    When I visit "/"
    Then I see a summary of the manifesto's executive summary
    And there is a link to read the full manifesto

  Scenario: Landing page shows five pillars
    When I visit "/"
    Then I see five pillar cards: Lamad, ImagoDei, Qahal, Shefa, Elohim
    And each card has a title, icon, and one-sentence description

  Scenario: Landing page shows live protocol stats
    When I visit "/"
    And the doorway health endpoint reports 42 content nodes and 3 humans
    Then I see "42 content nodes" and "3 humans" in the stats section

  Scenario: Landing page has call-to-action to enter learning platform
    When I visit "/"
    Then I see a "Start Learning" button
    And clicking it navigates to "/lamad"

  # --- Protocol-Native Delivery ---

  Scenario: Landing page is a ContentNode
    When I inspect the HTTP response for "/"
    Then the response includes "X-Root-App: protocol-landing"
    And the response includes "X-Content-Address" header

  Scenario: Landing page has proper SEO meta tags
    When I visit "/"
    Then the page has og:title "Elohim Protocol"
    And the page has og:description containing "human flourishing"

  # --- Fallback ---

  Scenario: Bootstrap page shown when SPA not yet loaded
    Given the protocol-landing blob is NOT yet extracted
    When I visit "/"
    Then I see the bootstrap page "Connecting to the Elohim Protocol..."
    And the page auto-refreshes when the SPA becomes available
```

- [ ] **Step 2: Commit**

```bash
git add genesis/a2o/features/delivery/landing-page.feature
git commit -m "feat(a2o): add landing page as SPA ContentNode scenarios"
```

---

### Task 2: Create the seed content node

**Files:**
- Create: `genesis/data/lamad/content/protocol-landing.json`

- [ ] **Step 1: Write the seed JSON**

```json
{
  "id": "protocol-landing",
  "contentType": "application",
  "title": "Elohim Protocol",
  "name": "Elohim Protocol Landing Page",
  "description": "Digital infrastructure for human flourishing — the protocol's public landing page, served as a content node.",
  "content": {
    "slug": "protocol-landing",
    "entryPoint": "index.html"
  },
  "contentFormat": "spa-bundle",
  "tags": [
    "application",
    "spa",
    "protocol",
    "landing-page"
  ],
  "blobHash": "",
  "reach": "commons",
  "metadata": {
    "category": "application",
    "embedStrategy": "root"
  },
  "createdAt": "2026-04-03T00:00:00.000000",
  "updatedAt": "2026-04-03T00:00:00.000000"
}
```

- [ ] **Step 2: Validate against schema**

Run: `pnpm run schema:validate`
Expected: PASS — protocol-landing.json validates against content node schema

- [ ] **Step 3: Commit**

```bash
git add genesis/data/lamad/content/protocol-landing.json
git commit -m "feat(genesis): add protocol-landing seed content node"
```

---

### Task 3: Scaffold the Vite landing page project

**Files:**
- Create: `genesis/landing/package.json`
- Create: `genesis/landing/vite.config.ts`
- Create: `genesis/landing/tsconfig.json`

- [ ] **Step 1: Create package.json**

```json
{
  "name": "@elohim/protocol-landing",
  "version": "0.1.0",
  "private": true,
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "node scripts/render-content.ts && vite build",
    "preview": "vite preview",
    "build:zip": "bash scripts/build-and-zip.sh"
  },
  "devDependencies": {
    "vite": "^6.0.0",
    "typescript": "~5.7.0",
    "marked": "^15.0.0"
  }
}
```

- [ ] **Step 2: Create vite.config.ts**

```typescript
import { defineConfig } from 'vite';

export default defineConfig({
  root: '.',
  build: {
    outDir: 'dist',
    emptyOutDir: true,
    // Minimal output — no framework, no code splitting
    rollupOptions: {
      input: 'index.html',
    },
  },
});
```

- [ ] **Step 3: Create tsconfig.json**

```json
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "outDir": "dist"
  },
  "include": ["src/**/*.ts", "scripts/**/*.ts"]
}
```

- [ ] **Step 4: Install dependencies**

Run: `cd genesis/landing && pnpm install`
Expected: Dependencies installed

- [ ] **Step 5: Commit**

```bash
git add genesis/landing/package.json genesis/landing/vite.config.ts genesis/landing/tsconfig.json genesis/landing/pnpm-lock.yaml
git commit -m "feat(landing): scaffold Vite landing page project"
```

---

### Task 4: Create the content rendering build script

**Files:**
- Create: `genesis/landing/scripts/render-content.ts`

This script runs at build time to extract manifesto content from the genesis docs and render it to HTML fragments that the landing page includes.

- [ ] **Step 1: Write the script**

```typescript
#!/usr/bin/env -S node --import tsx
// genesis/landing/scripts/render-content.ts
//
// Build-time script: reads manifesto.md and epic files from genesis/docs,
// renders them to HTML fragments, and writes to src/generated/ for
// inclusion in the landing page.

import { readFileSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { marked } from 'marked';

const GENESIS_DOCS = resolve(dirname(import.meta.url.replace('file://', '')), '../../docs/content/elohim-protocol');
const OUTPUT_DIR = resolve(dirname(import.meta.url.replace('file://', '')), '../src/generated');

// Ensure output directory exists
if (!existsSync(OUTPUT_DIR)) {
  mkdirSync(OUTPUT_DIR, { recursive: true });
}

// --- Manifesto summary ---
const manifestoMd = readFileSync(resolve(GENESIS_DOCS, 'manifesto.md'), 'utf-8');
// Extract executive summary (between ## Executive Summary and the next ##)
const summaryMatch = manifestoMd.match(/## \*\*Executive Summary\*\*\s*\n([\s\S]*?)(?=\n## )/);
const summaryHtml = summaryMatch
  ? marked.parse(summaryMatch[1].trim())
  : '<p>The manifesto summary could not be loaded.</p>';

// --- Pillar descriptions ---
const pillars = [
  {
    id: 'lamad',
    name: 'Lamad',
    icon: '\u{1F4DA}',
    description: 'Learning that transforms — paths curated by stewards, assessed through mastery, governed by the community.',
  },
  {
    id: 'imagodei',
    name: 'ImagoDei',
    icon: '\u{1F464}',
    description: 'Identity as relationship — you are known by your contributions, not your credentials.',
  },
  {
    id: 'qahal',
    name: 'Qahal',
    icon: '\u{1F3DB}',
    description: 'Governance as practice — consent, ranked choice, and constitutional limits experienced daily.',
  },
  {
    id: 'shefa',
    name: 'Shefa',
    icon: '\u{1F331}',
    description: 'Economics that flow — mutual credit, steward recognition, and value that decays without care.',
  },
  {
    id: 'elohim',
    name: 'Elohim',
    icon: '\u{1F525}',
    description: 'AI guardians that serve — agents bounded by constitutional law, carrying human interests into deliberation.',
  },
];

// Write generated content as JS module for import
const output = `// AUTO-GENERATED by render-content.ts — do not edit
export const manifestoSummary = ${JSON.stringify(summaryHtml)};
export const pillars = ${JSON.stringify(pillars, null, 2)};
`;

writeFileSync(resolve(OUTPUT_DIR, 'content.ts'), output, 'utf-8');
console.log('Generated landing page content to src/generated/content.ts');
```

- [ ] **Step 2: Add tsx as dev dependency for running TypeScript scripts**

In `package.json`, add to devDependencies:
```json
"tsx": "^4.0.0"
```

Run: `cd genesis/landing && pnpm install`

- [ ] **Step 3: Test the script**

Run: `cd genesis/landing && node --import tsx scripts/render-content.ts`
Expected: Creates `src/generated/content.ts` with manifesto summary HTML and pillar data

- [ ] **Step 4: Commit**

```bash
git add genesis/landing/scripts/render-content.ts genesis/landing/src/generated/content.ts genesis/landing/package.json genesis/landing/pnpm-lock.yaml
git commit -m "feat(landing): add build-time content rendering from genesis docs"
```

---

### Task 5: Build the landing page HTML and styles

**Files:**
- Create: `genesis/landing/index.html`
- Create: `genesis/landing/src/style.css`
- Create: `genesis/landing/src/main.ts`

- [ ] **Step 1: Write index.html**

```html
<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Elohim Protocol — Digital Infrastructure for Human Flourishing</title>
  <meta name="description" content="A distributed learning platform built on Holochain, governed by constitutional AI, and powered by mutual credit economics.">

  <!-- Open Graph -->
  <meta property="og:title" content="Elohim Protocol">
  <meta property="og:description" content="Digital infrastructure for human flourishing — learning, identity, governance, and economics woven into one constitutional substrate.">
  <meta property="og:type" content="website">

  <!-- Protocol meta -->
  <meta name="epr:content-type" content="application">
  <meta name="epr:content-format" content="spa-bundle">
  <meta name="epr:reach" content="commons">

  <link rel="stylesheet" href="/src/style.css">
</head>
<body>
  <!-- Hero -->
  <header class="hero" id="hero">
    <div class="hero-content">
      <h1 class="hero-title">Elohim Protocol</h1>
      <p class="hero-subtitle">Digital infrastructure for human flourishing</p>
      <p class="hero-tagline">
        Learning, identity, governance, and economics — woven into one
        constitutional substrate, guarded by AI that serves rather than rules.
      </p>
      <div class="hero-actions">
        <a href="/lamad" class="btn btn-primary" data-testid="cta-start-learning">Start Learning</a>
        <a href="#manifesto" class="btn btn-secondary">Read the Manifesto</a>
      </div>
    </div>
  </header>

  <!-- Live Stats -->
  <section class="stats" id="stats">
    <div class="stats-grid">
      <div class="stat">
        <span class="stat-value" id="stat-content">--</span>
        <span class="stat-label">Content Nodes</span>
      </div>
      <div class="stat">
        <span class="stat-value" id="stat-humans">--</span>
        <span class="stat-label">Humans</span>
      </div>
      <div class="stat">
        <span class="stat-value" id="stat-peers">--</span>
        <span class="stat-label">Peers Connected</span>
      </div>
    </div>
    <p class="stats-source">Live from the protocol network</p>
  </section>

  <!-- Manifesto Summary -->
  <section class="manifesto" id="manifesto">
    <div class="section-content">
      <h2>The Manifesto</h2>
      <div class="manifesto-body" id="manifesto-body">
        <!-- Populated at build time by render-content.ts, or at runtime as fallback -->
        <noscript>
          <p>We stand at a crossroads in the evolution of digital civilization.
          Current social media architectures, built on surveillance capitalism
          and engagement optimization, have failed to support human flourishing
          at scale.</p>
        </noscript>
      </div>
      <a href="/deliver/manifesto" class="read-more">Read the full manifesto</a>
    </div>
  </section>

  <!-- Five Pillars -->
  <section class="pillars" id="pillars">
    <div class="section-content">
      <h2>Five Pillars</h2>
      <p class="section-intro">The protocol organizes around five Hebrew-named domains, each coupling power with responsibility.</p>
      <div class="pillars-grid" id="pillars-grid">
        <!-- Populated by main.ts from generated content -->
      </div>
    </div>
  </section>

  <!-- Call to Action -->
  <section class="cta" id="cta">
    <div class="section-content">
      <h2>Enter the Protocol</h2>
      <p>The knowledge is accessible. The governance is participatory. The economics flow to contributors. Begin your path.</p>
      <a href="/lamad" class="btn btn-primary btn-large">Start Learning</a>
    </div>
  </section>

  <!-- Footer -->
  <footer class="footer">
    <p>Served as a protocol content node by doorway. <span id="footer-source"></span></p>
    <p class="footer-links">
      <a href="/deliver/manifesto">Manifesto</a>
      <a href="/deliver/constitution">Constitution</a>
      <a href="https://github.com/ethosengine/elohim">Source Code</a>
    </p>
  </footer>

  <script type="module" src="/src/main.ts"></script>
</body>
</html>
```

- [ ] **Step 2: Write styles**

```css
/* genesis/landing/src/style.css */

/* ============================================
   Reset & Base
   ============================================ */
*, *::before, *::after {
  box-sizing: border-box;
  margin: 0;
  padding: 0;
}

:root {
  --color-bg: #0f172a;
  --color-surface: #1e293b;
  --color-surface-hover: #334155;
  --color-text: #e2e8f0;
  --color-text-secondary: #94a3b8;
  --color-accent: #6366f1;
  --color-accent-light: #a5b4fc;
  --color-accent-glow: rgba(99, 102, 241, 0.15);
  --color-green: #22c55e;
  --max-width: 960px;
  --font-body: system-ui, -apple-system, 'Segoe UI', sans-serif;
  --font-mono: 'SF Mono', 'Fira Code', 'Cascadia Code', monospace;
}

html {
  scroll-behavior: smooth;
}

body {
  font-family: var(--font-body);
  background: var(--color-bg);
  color: var(--color-text);
  line-height: 1.6;
  -webkit-font-smoothing: antialiased;
}

a {
  color: var(--color-accent-light);
  text-decoration: none;
}

a:hover {
  text-decoration: underline;
}

/* ============================================
   Hero
   ============================================ */
.hero {
  min-height: 80vh;
  display: flex;
  align-items: center;
  justify-content: center;
  text-align: center;
  padding: 4rem 1.5rem;
  background:
    radial-gradient(ellipse at 30% 20%, var(--color-accent-glow), transparent 60%),
    radial-gradient(ellipse at 70% 80%, rgba(34, 197, 94, 0.08), transparent 60%),
    var(--color-bg);
}

.hero-content {
  max-width: var(--max-width);
}

.hero-title {
  font-size: clamp(2.5rem, 6vw, 4.5rem);
  font-weight: 800;
  letter-spacing: -0.02em;
  line-height: 1.1;
  margin-bottom: 0.75rem;
  background: linear-gradient(135deg, var(--color-text), var(--color-accent-light));
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
}

.hero-subtitle {
  font-size: clamp(1.125rem, 2.5vw, 1.5rem);
  color: var(--color-accent-light);
  font-weight: 500;
  margin-bottom: 1.5rem;
}

.hero-tagline {
  font-size: 1.125rem;
  color: var(--color-text-secondary);
  max-width: 600px;
  margin: 0 auto 2.5rem;
  line-height: 1.7;
}

.hero-actions {
  display: flex;
  gap: 1rem;
  justify-content: center;
  flex-wrap: wrap;
}

/* ============================================
   Buttons
   ============================================ */
.btn {
  display: inline-flex;
  align-items: center;
  padding: 0.75rem 1.75rem;
  border-radius: 8px;
  font-size: 1rem;
  font-weight: 600;
  transition: all 0.2s;
  text-decoration: none;
  cursor: pointer;
  border: none;
}

.btn-primary {
  background: var(--color-accent);
  color: white;
}

.btn-primary:hover {
  background: #4f46e5;
  transform: translateY(-1px);
  text-decoration: none;
}

.btn-secondary {
  background: transparent;
  color: var(--color-accent-light);
  border: 1px solid rgba(99, 102, 241, 0.4);
}

.btn-secondary:hover {
  background: var(--color-accent-glow);
  border-color: var(--color-accent);
  text-decoration: none;
}

.btn-large {
  padding: 1rem 2.5rem;
  font-size: 1.125rem;
}

/* ============================================
   Stats
   ============================================ */
.stats {
  padding: 2rem 1.5rem;
  text-align: center;
  border-top: 1px solid rgba(148, 163, 184, 0.1);
  border-bottom: 1px solid rgba(148, 163, 184, 0.1);
}

.stats-grid {
  display: flex;
  justify-content: center;
  gap: 3rem;
  max-width: var(--max-width);
  margin: 0 auto 0.75rem;
}

.stat {
  display: flex;
  flex-direction: column;
}

.stat-value {
  font-size: 2rem;
  font-weight: 800;
  font-family: var(--font-mono);
  color: var(--color-accent-light);
}

.stat-label {
  font-size: 0.75rem;
  color: var(--color-text-secondary);
  text-transform: uppercase;
  letter-spacing: 0.08em;
}

.stats-source {
  font-size: 0.6875rem;
  color: var(--color-text-secondary);
  font-style: italic;
}

/* ============================================
   Sections
   ============================================ */
.section-content {
  max-width: var(--max-width);
  margin: 0 auto;
  padding: 0 1.5rem;
}

.manifesto,
.pillars,
.cta {
  padding: 5rem 0;
}

.manifesto h2,
.pillars h2,
.cta h2 {
  font-size: 2rem;
  font-weight: 700;
  margin-bottom: 1.5rem;
}

.section-intro {
  color: var(--color-text-secondary);
  font-size: 1.125rem;
  margin-bottom: 2rem;
}

/* Manifesto */
.manifesto-body {
  color: var(--color-text-secondary);
  font-size: 1.0625rem;
  line-height: 1.8;
  margin-bottom: 1.5rem;
}

.manifesto-body p {
  margin-bottom: 1rem;
}

.read-more {
  font-weight: 600;
  font-size: 1rem;
}

/* Pillars */
.pillars {
  background: var(--color-surface);
}

.pillars-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 1.5rem;
}

.pillar-card {
  padding: 1.5rem;
  background: var(--color-bg);
  border-radius: 12px;
  border: 1px solid rgba(148, 163, 184, 0.1);
  transition: all 0.2s;
}

.pillar-card:hover {
  border-color: rgba(99, 102, 241, 0.3);
  transform: translateY(-2px);
}

.pillar-icon {
  font-size: 2rem;
  margin-bottom: 0.75rem;
  display: block;
}

.pillar-name {
  font-size: 1.25rem;
  font-weight: 700;
  margin-bottom: 0.5rem;
}

.pillar-desc {
  color: var(--color-text-secondary);
  font-size: 0.9375rem;
  line-height: 1.6;
}

/* CTA */
.cta {
  text-align: center;
  background:
    radial-gradient(ellipse at center, var(--color-accent-glow), transparent 70%),
    var(--color-bg);
}

.cta p {
  color: var(--color-text-secondary);
  font-size: 1.125rem;
  max-width: 600px;
  margin: 0 auto 2rem;
  line-height: 1.7;
}

/* Footer */
.footer {
  padding: 2rem 1.5rem;
  text-align: center;
  border-top: 1px solid rgba(148, 163, 184, 0.1);
  color: var(--color-text-secondary);
  font-size: 0.8125rem;
}

.footer-links {
  margin-top: 0.75rem;
  display: flex;
  gap: 1.5rem;
  justify-content: center;
}

.footer-links a {
  color: var(--color-text-secondary);
  font-size: 0.8125rem;
}

.footer-links a:hover {
  color: var(--color-accent-light);
}

/* ============================================
   Responsive
   ============================================ */
@media (max-width: 640px) {
  .stats-grid {
    flex-direction: column;
    gap: 1.5rem;
  }

  .hero {
    min-height: 70vh;
    padding: 3rem 1rem;
  }

  .hero-actions {
    flex-direction: column;
    align-items: center;
  }
}
```

- [ ] **Step 3: Write main.ts**

```typescript
// genesis/landing/src/main.ts
import './style.css';
import { manifestoSummary, pillars } from './generated/content';

// --- Populate manifesto summary ---
const manifestoBody = document.getElementById('manifesto-body');
if (manifestoBody) {
  manifestoBody.innerHTML = manifestoSummary;
}

// --- Populate pillars grid ---
const pillarsGrid = document.getElementById('pillars-grid');
if (pillarsGrid) {
  pillarsGrid.innerHTML = pillars
    .map(
      (p) => `
    <div class="pillar-card" data-epr-ref="${p.id}">
      <span class="pillar-icon">${p.icon}</span>
      <h3 class="pillar-name">${p.name}</h3>
      <p class="pillar-desc">${p.description}</p>
    </div>
  `,
    )
    .join('');
}

// --- Fetch live stats from doorway ---
async function loadStats(): Promise<void> {
  try {
    const response = await fetch('/health/startup');
    if (!response.ok) return;

    const data = await response.json();

    const contentEl = document.getElementById('stat-content');
    const humansEl = document.getElementById('stat-humans');
    const peersEl = document.getElementById('stat-peers');

    if (contentEl && data.projection?.content != null) {
      contentEl.textContent = String(data.projection.content);
    }
    if (humansEl && data.projection?.humans != null) {
      humansEl.textContent = String(data.projection.humans);
    }
    if (peersEl && data.projection?.relationships != null) {
      peersEl.textContent = String(data.projection.relationships);
    }
  } catch {
    // Stats are progressive enhancement — page works without them
  }
}

loadStats();

// --- Footer delivery source ---
const footerSource = document.getElementById('footer-source');
if (footerSource) {
  footerSource.textContent = `Served by ${window.location.hostname}`;
}
```

- [ ] **Step 4: Test the dev server**

Run: `cd genesis/landing && pnpm dev`
Expected: Opens at localhost:5173, shows landing page with hero, manifesto, pillars, CTA. Stats show "--" (no doorway available in dev).

- [ ] **Step 5: Build and verify output**

Run: `cd genesis/landing && pnpm build && ls -la dist/`
Expected: `dist/` contains `index.html`, `assets/` with CSS and JS bundles. Total size < 100KB.

- [ ] **Step 6: Commit**

```bash
git add genesis/landing/index.html genesis/landing/src/
git commit -m "feat(landing): build protocol landing page with hero, manifesto, pillars, and stats"
```

---

### Task 6: Create build-and-zip script

**Files:**
- Create: `genesis/landing/scripts/build-and-zip.sh`

- [ ] **Step 1: Write the build script**

```bash
#!/usr/bin/env bash
# genesis/landing/scripts/build-and-zip.sh
#
# Build the landing page, ZIP the dist, and compute the SHA256 hash.
# Output: dist/protocol-landing.zip and dist/protocol-landing.sha256

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "=== Building landing page ==="
node --import tsx scripts/render-content.ts
npx vite build

echo "=== Creating ZIP ==="
cd dist
zip -r ../dist/protocol-landing.zip . -x "protocol-landing.zip" "protocol-landing.sha256"
cd ..

echo "=== Computing SHA256 ==="
sha256sum dist/protocol-landing.zip | awk '{print $1}' > dist/protocol-landing.sha256
HASH=$(cat dist/protocol-landing.sha256)

echo ""
echo "Build complete:"
echo "  ZIP:    dist/protocol-landing.zip"
echo "  SHA256: $HASH"
echo "  Size:   $(du -h dist/protocol-landing.zip | awk '{print $1}')"
```

- [ ] **Step 2: Make executable**

```bash
chmod +x genesis/landing/scripts/build-and-zip.sh
```

- [ ] **Step 3: Test the full build**

Run: `cd genesis/landing && bash scripts/build-and-zip.sh`
Expected: Creates `dist/protocol-landing.zip` and `dist/protocol-landing.sha256`. ZIP < 150KB.

- [ ] **Step 4: Commit**

```bash
git add genesis/landing/scripts/build-and-zip.sh
git commit -m "feat(landing): add build-and-zip script for SPA blob packaging"
```

---

### Task 7: Add .gitignore and finalize project

**Files:**
- Create: `genesis/landing/.gitignore`

- [ ] **Step 1: Create .gitignore**

```
node_modules/
dist/
src/generated/
```

Note: `src/generated/` is build output from `render-content.ts` — regenerated each build, not committed.

- [ ] **Step 2: Add landing to pnpm workspace (if needed)**

Check if `genesis/landing` needs to be added to the root `pnpm-workspace.yaml`. If the workspace uses a glob like `genesis/*`, it may be auto-included. If not, add:

```yaml
  - genesis/landing
```

- [ ] **Step 3: Verify clean build from scratch**

```bash
cd genesis/landing
rm -rf node_modules dist src/generated
pnpm install
bash scripts/build-and-zip.sh
```

Expected: Clean build succeeds, ZIP created.

- [ ] **Step 4: Commit**

```bash
git add genesis/landing/.gitignore
git commit -m "chore(landing): add gitignore and finalize project scaffold"
```

---

### Task 8: Document ROOT_APP_SLUG configuration

**Files:**
- Modify: `genesis/plans/2026-04-02-doorway-spa-as-blob-design.md` (add operator guide section)

- [ ] **Step 1: Add configuration note to the design doc**

At the end of the design doc, add:

```markdown
---

## 8. Operator Configuration

### Switching the root app

Set `ROOT_APP_SLUG` environment variable in the doorway deployment:

| ROOT_APP_SLUG | What doorway serves at `/` |
|---------------|---------------------------|
| `lamad` | The Lamad learning platform (Angular SPA) |
| `protocol-landing` | The Elohim Protocol landing page (static SPA) |
| *(unset)* | Redirect to `/threshold` (operator dashboard) |

For alpha.elohim.host, the intended configuration is:
- `ROOT_APP_SLUG=protocol-landing` — the public-facing protocol site
- The Lamad app is accessible at `/lamad` (handled by the root SPA's routing or as a separate spa-bundle)

### Build and deploy the landing page

```bash
cd genesis/landing
pnpm install
bash scripts/build-and-zip.sh

# Upload blob
curl -X PUT "http://${STORAGE_URL}/blob/$(cat dist/protocol-landing.sha256)" \
  --data-binary @dist/protocol-landing.zip \
  -H "Content-Type: application/zip"

# Update seed node blobHash
# (via seeder or direct storage API call)
```
```

- [ ] **Step 2: Commit**

```bash
git add genesis/plans/2026-04-02-doorway-spa-as-blob-design.md
git commit -m "docs: add operator configuration guide for ROOT_APP_SLUG"
```

---

## Self-Review Checklist

1. **Spec coverage:** All 8 a2o scenarios addressed:
   - Hero section → Task 5 (index.html hero)
   - Manifesto summary → Task 4 (render-content.ts) + Task 5 (manifesto section)
   - Five pillars → Task 4 (pillar data) + Task 5 (pillars grid)
   - Live stats → Task 5 (main.ts loadStats)
   - CTA → Task 5 (CTA section)
   - ContentNode delivery → Task 2 (seed node) + doorway root_app.rs
   - SEO meta tags → Task 5 (og:* meta tags in head)
   - Bootstrap fallback → Already implemented in root_app.rs

2. **Placeholder scan:** No TBDs found.

3. **Type consistency:** `protocol-landing` slug used consistently in seed node (Task 2), content object (Task 2), build script (Task 6), and configuration (Task 8).

**Deferred:**
- CI pipeline stage for landing page build (follows same pattern as lamad-spa — add when CI blob upload is implemented)
- `epr-composite` landing page (future — the page body becomes a layout of EPR references)
- Multi-SPA doorway routing (one root app per doorway for now)
