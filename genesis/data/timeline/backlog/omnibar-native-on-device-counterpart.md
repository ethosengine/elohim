---
id: "backlog-omnibar-native-on-device-counterpart"
kind: "backlog"
contentType: "backlog-item"
contentFormat: "markdown"
title: "Omnibar trust-wrapper — native on-device counterpart (offload flag)"
slug: "omnibar-native-on-device-counterpart"
written: "2026-06-26"
author: "operator-stated"
status: "envisioned"
priority: "low"
target_window: "open-ended"
domain: D8
tags: [omnibar, render, doorway, tauri, peer-runtime, offload]
---

# Omnibar trust-wrapper — native on-device counterpart (offload flag)

**Domain:** D8 (doorway projection / render). Long-term, operator-stated 2026-06-26.
**Relates:** genesis/docs/superpowers/specs/2026-06-26-native-rust-epr-shell-ssr-design.md ; [[project_hub_optional_floor]] ; [[project_rea_compute_commitment_primitive]]

The EPR omnibar / trust-wrapper is delivered (Phase 2) as a **runtime-served client element** — the "web2.0 wrapper" — served by the peer runtime (doorway AND the device's own elohim-storage sidecar) at a content-addressed `/chrome/` path, client-rendered in browser, `/deliver` (CSR), and Tauri identically.

**The peer-runtime framing:** Tauri is the native client running ON a peer-based runtime; the device IS a peer. Its runtime should serve the wrapper (SSR/chrome) to **itself (local Tauri webview) AND to other peers connecting to it**, gated by device capability — **enabled/disabled at the runtime layer** (the existing `RenderCapabilityProfile` is the seam).

**Long-term TODO:** a runtime-layer flag that **OFFLOADS the web2.0 wrapper to a fully on-device NATIVE counterpart** — a Tauri-native trust/EPR-navigation surface that replaces the web2.0 element on a capable device. Two implementations of the one trust-wrapper (web2.0 element ↔ native), chosen by a runtime flag. NOT Phase 2 scope; captured so it isn't lost.
