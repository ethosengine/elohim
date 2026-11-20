---
name: feedback_bridge_native_toolchains_not_lcd
title: "feedback_bridge_native_toolchains_not_lcd"
description: "Operator 2026-11-10: never level devices onto one image/toolchain so cache keys match; keep each device native and bridge toolchains in the artifact store (key true, admit by host requirement)."
metadata:
  type: feedback
---

When the artifact-store F2r miss traced to differing glibc-devel between the two devices' native
images, I moved every device onto the x86-64-v2 image (a916ddcfa). The operator rejected it:
"settling by being compatible to the lowest common denominator ... rather than natively building
support for bridging over native toolchains over the image level. I don't like it." Reverted in
b929983ca.

**Why:** levelling makes the stronger device pay for the weaker one and hides the real gap: an
identity key cannot say whether a foreign build runs here. Pair devs on different hardware
are the normal case, and the protocol should bridge that difference, not erase it.

**How to apply:** each device runs the flavor native to its hardware. Level the floor BELOW the
image instead: a declared target (pinned, CID-addressed sysroot + C flags) in the build method, so
two native images get identical link inputs, plus variant sets admitted on the host by requires/
provides/admit. Authority: spec/host-compatibility-foundation (genesis/docs/superpowers/specs/
2026-11-10-host-compatibility-foundation-design.md); steward ruled H1-H7 as recommended on
2026-11-10, so K1 = key honesty only and K1b (cross-key substitute) is WITHDRAWN. When a fix
"works" by making environments identical, ask whether it should be a floor or a bridge.
Related: [[project_earned_reach_governance_pr_ceremony_vision]], [[project_rakia_as_artifact_store_custody_registry]].
