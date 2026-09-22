---
name: reference-node-hardware-ethosengine-shem
title: Node hardware — ethosengine + shem
description: "ethosengine = X470D4U/3900X/64G, worn QLC mirror; shem = T7610 dual E5/135G/ZFS — before storage/reboot/placement."
metadata: 
  node_type: memory
  title: Node hardware — ethosengine + shem
  type: reference
  originSessionId: 19a11573-749b-4c03-98f6-ce6c246c5400
  modified: 2026-09-20T22:09:27.602Z
cites:
  - genesis/manifests/cluster-state.yaml
---

Measured 2026-09-18 from inside the workspace (/sys, /proc) and Prometheus node-exporter. Re-probe before relying on it after any purchase.

**ethosengine** (192.168.86.100, kernel 7.0.0-31): ASRock Rack X470D4U, Ryzen 9 3900X (24t), 64 GB, no add-in cards (BMC video only; CPU x16/x8 slots empty).
- Storage: 2x Crucial P1 1TB (CT1000P1SSD8, **QLC**, 200 TBW rating) in md RAID1 — md0 boot, **md1 = the 4 GB swap**, md2 = 971 GB holding /projects. Onboard M.2 links are Gen3 x2 and Gen2 x4 (~1.7 GB/s cap; X470 has no Gen4).
- Measured host write rate ~6 MB/s avg ≈ 195 TB/yr — the P1s burn their rating in about a year.
- `kernel.softlockup_panic=1`, `kernel.panic=10`, `hung_task_panic=0`, `watchdog_thresh=10`, nvme io_timeout 30 s: a 20 s kernel spin auto-reboots the node — the "softlock reboot during heavy dev" mechanism. Working theory: build writes + swap on the worn QLC mirror + hard NFS stall reclaim; unconfirmed until a panic backtrace is captured (pstore/kdump). See [[project_devspace_recovery]], [[project_ram_guard_oom_group_kill]].

**shem** (10.99.0.2 — other premises over the VPN, kernel 6.8): Dell Precision T7610 (2013, BIOS A06 2014), dual Xeon E5-26xx v2 6-core (24t, Ivy Bridge, PCIe Gen3, no M.2, no bifurcation), 135 GB DDR3.
- Storage: single Samsung 870 SATA boot (unmirrored, LVM); ZFS `tank` ≈3.86 TB = mirror of 2x WD4000FYYZ (WD RE 4TB, SATA drives on the onboard LSI SAS controller, 2012-era).
- CPU package temps peak 86 °C over 7 d under fixture load only ([[project_compute_envelope_three_homes_k8s_bridge]] records its PSU failure as thermal).

**Purchase reasoning reached 2026-09-18:** TLC NVMe (WD SN7100 2TB class) on single-drive passive PCIe adapters for both nodes; used enterprise SATA (PM883) only where a SATA bay is the constraint; no white-label DRAMless SATA (Fikwot); NAND prices are in an AI-driven spike so verify $/TB at checkout. CORRECTED same day: the "$156 SN7100 2TB" figure was stale — real street price was $243 on sale (June 2026), $300+ typical, and the operator's 4-drive cart came to $2,558. At those prices used enterprise with power-loss protection wins (PM883 3.84TB SATA ≈ $235; used U.2 1.92TB ≈ $200 + adapter). SECOND CORRECTION 2026-09-20: the "$235 PM883" was also a stale single listing — the operator's preferred store (serverpartdeals.com, 3-yr seller warranty, 30-day returns; its Shopify feed `/collections/<handle>/products.json` is readable with curl when the page itself renders empty) lists refurb enterprise at 3.84TB SAS $469 / SATA $539-589, 1.92TB SATA $279-319, 1.6TB U.2 NVMe $329 — about the same $/TB as new consumer NVMe. ethosengine has SATA ports only (no SAS HBA); shem's onboard LSI controller takes SAS. Never quote a single-source SSD price as a budget basis; do the $0 steps (swap off md1, pstore) before any spend.
