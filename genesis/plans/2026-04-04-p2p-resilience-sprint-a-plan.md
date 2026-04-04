# P2P Resilience Sprint A: Wire Existing Data

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stewardship allocations flow from DB to resilience tooltip. RS encoding proven correct via integration tests. Context menu z-index and signal empty state bugs fixed.

**Architecture:** No new backend infrastructure. Fix frontend wiring to use existing `StewardshipAllocationService` data instead of empty `node.stewardedBy`. Add RS encode/drop/reconstruct tests in `sharding.rs`. Quick CSS fixes for z-index and signal display.

**Tech Stack:** Angular 19, Rust (reed-solomon-erasure crate), Vitest, cargo test

---

### Task 1: Fix Resilience Tooltip to Use Allocation Data

The resilience tooltip currently reads `node.stewardedBy` which is always empty. The stewardship data is already loaded via `StewardshipAllocationService` into `this.stewardship` (line 106 of content-viewer.component.ts). Wire the tooltip and icon to read from that instead.

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts:942-974`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html:67-74`

- [ ] **Step 1: Update `getResilienceIcon()` to use allocation data**

Replace the method at line 942:

```typescript
getResilienceIcon(): string {
  const stewardCount = this.stewardship?.allocations?.length || 0;
  if (stewardCount >= 3) return '\u{1F7E2}'; // green circle
  if (stewardCount >= 1) return '\u{1F7E1}'; // yellow circle
  if (this.stewardship === null) return '\u{1F504}'; // loading (arrows)
  return '\u{26AA}'; // white circle (no stewards)
}
```

- [ ] **Step 2: Update `getResilienceTooltip()` to use allocation data**

Replace the method at line 952:

```typescript
getResilienceTooltip(): string {
  if (!this.node) return '';
  if (this.stewardship === null) return 'Loading stewardship data...';

  const lines: string[] = [];

  const allocs = this.stewardship?.allocations || [];
  if (allocs.length > 0) {
    const stewards = allocs
      .sort((a, b) => (b.allocation?.allocationRatio ?? 0) - (a.allocation?.allocationRatio ?? 0))
      .map(a => {
        const name = a.presence?.displayName || a.allocation?.stewardPresenceId || 'Unknown';
        const pct = Math.round((a.allocation?.allocationRatio ?? 0) * 100);
        const type = a.allocation?.contributionType || 'steward';
        return `${name} (${type}, ${pct}%)`;
      })
      .join(', ');
    lines.push(`Stewards: ${stewards}`);
  } else {
    lines.push('No stewards assigned');
  }

  if (this.node.trustScore != null) {
    lines.push(`Trust: ${Math.round(this.node.trustScore * 100)}%`);
  }

  lines.push(`Reach: ${this.node.reach || 'commons'}`);

  return lines.join('\n') || 'No resilience data available';
}
```

- [ ] **Step 3: Change resilience click target from 'trust' to 'network' tab**

In `content-viewer.component.html` at line 70, change:

```html
(click)="setActiveTab('network')"
```

This sends users to the network tab (where deeper topology will live in Sprint B) instead of the trust tab.

- [ ] **Step 4: Verify in browser**

Run: `cd app/elohim-app && pnpm start`

Navigate to any content resource page. The resilience icon should show:
- Spinning arrows while stewardship loads
- Yellow/green circle once allocations load (if stewards exist)
- Tooltip should show steward names, contribution types, and percentages
- Clicking should switch to the Network tab

- [ ] **Step 5: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html
git commit -m "fix(lamad): wire resilience tooltip to stewardship allocation data

Tooltip now reads from StewardshipAllocationService (already loaded)
instead of empty node.stewardedBy field. Shows real steward names,
contribution types, and allocation percentages."
```

---

### Task 2: Fix Context Menu Z-Index

The EPR context menu (`context-menu-wrapper`) slips behind other UI elements because `.menu-dropdown` is at `z-index: 100` which is too low.

**Files:**
- Modify: `app/elohim-app/src/app/qahal/components/context-menu-only/context-menu-only.component.ts:60-112`

- [ ] **Step 1: Add z-index to wrapper and bump menu z-indices**

In the component's inline styles (around line 60), find `.context-menu-wrapper` and add `position: relative; z-index: 9997;`. If `.context-menu-wrapper` doesn't have explicit styles, add them.

Update the existing styles:

```css
.menu-backdrop {
  position: fixed;
  inset: 0;
  z-index: 9998;
}

.menu-dropdown {
  position: absolute;
  right: 0;
  top: 100%;
  margin-top: 4px;
  min-width: 220px;
  padding: 4px 0;
  background: var(--surface, #fff);
  border: 1px solid var(--border, #e5e5e5);
  border-radius: 8px;
  box-shadow: 0 4px 16px rgba(0, 0, 0, 0.12);
  list-style: none;
  z-index: 9999;
}
```

- [ ] **Step 2: Verify in browser**

Navigate to a content page. Trigger the context menu (kebab/three-dot button). Menu items should render fully above all other UI elements including headers, sidebars, and overlays.

- [ ] **Step 3: Commit**

```bash
git add app/elohim-app/src/app/qahal/components/context-menu-only/context-menu-only.component.ts
git commit -m "fix(qahal): bump context menu z-index to prevent clipping

Menu backdrop to 9998, dropdown to 9999. Prevents EPR context menu
from slipping behind other UI elements."
```

---

### Task 3: Show Signal Empty State

The signal summary section is hidden when `reactionCounts.total === 0`. Show a muted empty state instead.

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html:298-333`
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.css`

- [ ] **Step 1: Add empty state to signal summary section**

Replace the signal summary section (lines 298-333) with:

```html
<!-- Signal Summary -->
<div
  class="signal-summary"
  *ngIf="aggregatedSignals"
  data-testid="viewer-signal-summary"
>
  <ng-container *ngIf="aggregatedSignals.reactionCounts.total > 0; else emptySignals">
    <div class="signal-metric">
      <span class="metric-label">Community Sentiment</span>
      <span
        class="metric-value"
        [ngClass]="{
          positive: aggregatedSignals.sentimentScore > 0.6,
          neutral:
            aggregatedSignals.sentimentScore >= 0.4 &&
            aggregatedSignals.sentimentScore <= 0.6,
          negative: aggregatedSignals.sentimentScore < 0.4,
        }"
      >
        {{ aggregatedSignals.sentimentScore * 100 | number: '1.0-0' }}%
      </span>
    </div>
    <div class="signal-metric" *ngIf="aggregatedSignals.effectivenessScore > 0">
      <span class="metric-label">Effectiveness</span>
      <span class="metric-value">
        {{ aggregatedSignals.effectivenessScore * 100 | number: '1.0-0' }}%
      </span>
    </div>
    <div class="signal-metric">
      <span class="metric-label">Consensus</span>
      <span
        class="metric-value consensus-badge"
        [ngClass]="aggregatedSignals.consensusState.level"
      >
        {{ aggregatedSignals.consensusState.level | titlecase }}
      </span>
    </div>
  </ng-container>
  <ng-template #emptySignals>
    <div class="signal-empty" data-testid="viewer-signal-empty">
      <span class="metric-label">{{ aggregatedSignals.reactionCounts.total }} signals from 0 participants</span>
    </div>
  </ng-template>
</div>
```

- [ ] **Step 2: Add empty state CSS**

Add to the component CSS file after the existing signal-summary styles:

```css
.signal-empty {
  padding: 0.5rem 0;
  opacity: 0.5;
  font-size: 0.85rem;
}
```

- [ ] **Step 3: Verify in browser**

Navigate to a content page. The signal summary should show "0 signals from 0 participants" in a muted style instead of being invisible.

- [ ] **Step 4: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.html app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.css
git commit -m "fix(lamad): show signal empty state instead of hiding section

Displays '0 signals from 0 participants' when no reactions exist,
rather than hiding the entire section."
```

---

### Task 4: RS Integration Tests — Prove Encode/Drop/Reconstruct

Add comprehensive tests that prove Reed-Solomon encoding actually works: encode data, drop shards, reconstruct, verify identical output. Test the boundary between "can reconstruct" and "cannot reconstruct."

**Files:**
- Modify: `elohim/elohim-storage/src/sharding.rs` (add tests to existing `mod tests`)

- [ ] **Step 1: Add test for RS reconstruction after dropping max parity shards (3)**

Add to the `mod tests` block at the bottom of `sharding.rs`:

```rust
#[test]
fn test_rs_reconstruct_after_dropping_max_parity() {
    // Prove: RS 4+3 can survive loss of all 3 parity shards
    let encoder = ShardEncoder::new(ShardConfig {
        shard_size: 25,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 50,
        single_shard_max: 10,
    });

    let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
    let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons");
    let shards = encoder.create_shards(&data, &manifest.encoding);

    assert_eq!(manifest.encoding, "rs-4-7");
    assert_eq!(shards.len(), 7);

    // Drop all 3 parity shards (indices 4, 5, 6)
    let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
    shard_opts[4] = None;
    shard_opts[5] = None;
    shard_opts[6] = None;

    let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
    assert_eq!(reconstructed, data, "Must reconstruct from data shards alone");
}
```

- [ ] **Step 2: Add test for RS reconstruction after dropping 3 mixed shards**

```rust
#[test]
fn test_rs_reconstruct_after_dropping_3_mixed_shards() {
    // Prove: RS 4+3 can survive loss of any 3 shards (mix of data and parity)
    let encoder = ShardEncoder::new(ShardConfig {
        shard_size: 25,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 50,
        single_shard_max: 10,
    });

    let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
    let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons");
    let shards = encoder.create_shards(&data, &manifest.encoding);

    // Drop 2 data shards and 1 parity shard
    let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
    shard_opts[0] = None; // data shard 0
    shard_opts[2] = None; // data shard 2
    shard_opts[5] = None; // parity shard 1

    let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
    assert_eq!(reconstructed, data, "Must reconstruct from 4 remaining shards");
}
```

- [ ] **Step 3: Add test proving 4 dropped shards fails**

```rust
#[test]
fn test_rs_fails_when_too_many_shards_lost() {
    // Prove: RS 4+3 CANNOT survive loss of 4 shards (only 3 remain, need 4)
    let encoder = ShardEncoder::new(ShardConfig {
        shard_size: 25,
        rs_data_shards: 4,
        rs_parity_shards: 3,
        rs_threshold: 50,
        single_shard_max: 10,
    });

    let data: Vec<u8> = (0..200).map(|i| (i % 256) as u8).collect();
    let manifest = encoder.create_manifest(&data, "application/octet-stream", "commons");
    let shards = encoder.create_shards(&data, &manifest.encoding);

    // Drop 4 shards — only 3 remain, but we need 4 data shards minimum
    let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
    shard_opts[0] = None;
    shard_opts[1] = None;
    shard_opts[2] = None;
    shard_opts[4] = None;

    let result = encoder.reconstruct(&manifest, &shard_opts);
    assert!(result.is_err(), "Must fail when fewer than data_shards remain");
}
```

- [ ] **Step 4: Add test for chunked encoding roundtrip**

```rust
#[test]
fn test_chunked_roundtrip() {
    let encoder = ShardEncoder::new(ShardConfig {
        shard_size: 10,
        single_shard_max: 5,
        rs_threshold: 500,
        ..Default::default()
    });

    let data: Vec<u8> = (0..73).map(|i| (i % 256) as u8).collect();
    let manifest = encoder.create_manifest(&data, "text/plain", "commons");
    let shards = encoder.create_shards(&data, &manifest.encoding);

    assert_eq!(manifest.encoding, "chunked");
    assert_eq!(shards.len(), 8); // 73 bytes / 10 = 8 chunks

    // All present — reconstruct
    let shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
    let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
    assert_eq!(reconstructed, data);
}
```

- [ ] **Step 5: Add test for chunked encoding fails on missing shard**

```rust
#[test]
fn test_chunked_fails_on_missing_shard() {
    let encoder = ShardEncoder::new(ShardConfig {
        shard_size: 10,
        single_shard_max: 5,
        rs_threshold: 500,
        ..Default::default()
    });

    let data: Vec<u8> = (0..73).map(|i| (i % 256) as u8).collect();
    let manifest = encoder.create_manifest(&data, "text/plain", "commons");
    let shards = encoder.create_shards(&data, &manifest.encoding);

    let mut shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
    shard_opts[3] = None; // Drop one chunk

    let result = encoder.reconstruct(&manifest, &shard_opts);
    assert!(result.is_err(), "Chunked encoding requires all shards");
}
```

- [ ] **Step 6: Add test for single shard roundtrip**

```rust
#[test]
fn test_single_shard_roundtrip() {
    let encoder = ShardEncoder::new(ShardConfig::default());

    let data = b"The fruit back on the tree.";
    let manifest = encoder.create_manifest(data, "text/plain", "commons");
    let shards = encoder.create_shards(data, &manifest.encoding);

    assert_eq!(manifest.encoding, "none");
    assert_eq!(shards.len(), 1);

    let shard_opts: Vec<Option<Vec<u8>>> = shards.iter().map(|s| Some(s.clone())).collect();
    let reconstructed = encoder.reconstruct(&manifest, &shard_opts).unwrap();
    assert_eq!(reconstructed, data);
}
```

- [ ] **Step 7: Run all sharding tests**

Run: `cd elohim/elohim-storage && RUSTFLAGS='--cfg getrandom_backend="custom"' cargo test sharding -- --nocapture`

Expected: All tests pass, including the new ones. The RS tests prove that encoding survives loss of up to 3 shards but fails on loss of 4.

- [ ] **Step 8: Commit**

```bash
git add elohim/elohim-storage/src/sharding.rs
git commit -m "test(storage): prove RS encode/drop/reconstruct pipeline

Six new integration tests covering:
- RS 4+3 survives loss of all 3 parity shards
- RS 4+3 survives loss of 3 mixed data+parity shards
- RS 4+3 correctly fails when 4+ shards lost
- Chunked encoding roundtrip and missing-shard failure
- Single shard roundtrip"
```

---

### Task 5: Update Omnibar Stewards from Allocation Data

The omnibar (protocol address bar in focused view) also reads from `node.stewardedBy`. Wire it to use allocation data too.

**Files:**
- Modify: `app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts:364-370`

- [ ] **Step 1: Find the omnibar steward population code**

At line 367, the code reads:
```typescript
this.omnibarStewards = (contentNode.stewardedBy || []).map(s => ({
  humanId: s.humanId,
  displayName: s.humanId,
  ratio: s.affinity ?? 0,
```

- [ ] **Step 2: Add a method that populates omnibar stewards from allocation data**

Add a new method in the component:

```typescript
private updateOmnibarStewards(): void {
  if (!this.stewardship?.allocations?.length) {
    this.omnibarStewards = [];
    return;
  }
  this.omnibarStewards = this.stewardship.allocations.map(a => ({
    humanId: a.allocation?.stewardPresenceId || '',
    displayName: a.presence?.displayName || a.allocation?.stewardPresenceId || 'Unknown',
    ratio: a.allocation?.allocationRatio ?? 0,
  }));
}
```

- [ ] **Step 3: Call updateOmnibarStewards after stewardship loads**

In `loadStewardship()` (line 467), update the subscribe handler:

```typescript
private loadStewardship(nodeId: string): void {
  this.stewardshipService
    .getContentStewardship(nodeId)
    .pipe(takeUntil(this.destroy$))
    .subscribe({
      next: stewardship => {
        this.stewardship = stewardship;
        this.updateOmnibarStewards();
      },
      error: () => {
        // Stewardship is supplemental — don't block on failure
      },
    });
}
```

- [ ] **Step 4: Remove the old stewardedBy-based omnibar population**

At line 367, replace:
```typescript
this.omnibarStewards = (contentNode.stewardedBy || []).map(s => ({
  humanId: s.humanId,
  displayName: s.humanId,
  ratio: s.affinity ?? 0,
```

With:
```typescript
// Omnibar stewards populated from allocation data in loadStewardship()
this.omnibarStewards = [];
```

- [ ] **Step 5: Verify in browser**

Open a content page in focused view (if available). The omnibar should show steward names from the allocation data, not empty.

- [ ] **Step 6: Commit**

```bash
git add app/elohim-app/src/app/lamad/components/content-viewer/content-viewer.component.ts
git commit -m "fix(lamad): wire omnibar stewards to allocation data

Omnibar steward display now reads from StewardshipAllocationService
instead of empty node.stewardedBy, matching the resilience tooltip fix."
```

---

### Summary: Sprint A Delivers

After all 5 tasks:
- Resilience tooltip shows real steward names, types, percentages from allocation DB
- Resilience icon reflects actual steward count (loading state while fetching)
- Context menu renders above all UI elements
- Signal summary shows empty state instead of hiding
- RS encoding proven correct: survives 3 shard loss, fails on 4
- Omnibar stewards populated from live data
