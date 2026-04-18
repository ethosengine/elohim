//! Space-type detection — the first-class architectural boundary between
//! exempt interiors (offline / private-drafting / play) and network-impacting
//! zones the gate must wrap.
//!
//! Per spec §1.5: space-type is primarily a **context signal** fed into wisdom,
//! not a gate-control mechanism. Only the explicitly-exempt interiors short-
//! circuit the gate; every other space-type flows through the universal band
//! with the space signal included in context.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

use crate::events::RelationalImpactEvent;

/// The space in which an event is occurring.
///
/// Space-types split into three groups:
///
/// - **Exempt interiors** — the gate does **not** fire. These are architectural
///   primitives, not wisdom judgments: a gate that watched every keystroke
///   inside a private journal would be a panopticon. Exempt variants:
///   [`Offline`](SpaceType::Offline),
///   [`PrivateDraftingInterior`](SpaceType::PrivateDraftingInterior),
///   [`PlayInterior`](SpaceType::PlayInterior),
///   [`RoleplayInterior`](SpaceType::RoleplayInterior).
/// - **Boundary-crossing** — the gate fires, and a summarization primitive
///   applies to any private context being surfaced. Variants:
///   [`PrivateDraftingCrossing`](SpaceType::PrivateDraftingCrossing),
///   [`PlayExiting`](SpaceType::PlayExiting),
///   [`RoleplayExiting`](SpaceType::RoleplayExiting),
///   [`SyncAfterOffline`](SpaceType::SyncAfterOffline),
///   [`AdviceSeeking`](SpaceType::AdviceSeeking).
/// - **Normal public activity** — the gate fires with full wisdom. Variant:
///   [`Public`](SpaceType::Public).
///
/// Space-type is primarily a **context signal fed into wisdom**, not a
/// gate-control mechanism. Wisdom reads space-type the way humans read a
/// conversation's setting. Only the explicit exempt interiors short-circuit
/// the gate.
///
/// See spec §P1.5 (Privacy, Drafting, Play, and Roleplay as Architectural
/// Primitives) and §1.5 (Space-type detection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(
        export,
        export_to = "../../elohim-agent-sdk/src/gate-client/generated/"
    )
)]
#[serde(rename_all = "kebab-case")]
pub enum SpaceType {
    /// Normal network activity. Universal band fires with full wisdom.
    Public,

    /// A private draft being published to the network — boundary-crossing.
    /// Gate fires; summarization primitive applies.
    PrivateDraftingCrossing,

    /// Content leaving a play-space for the public — boundary-crossing.
    PlayExiting,

    /// Roleplay content crossing into non-roleplay context — boundary-crossing.
    RoleplayExiting,

    /// Sync event after an offline period — boundary-crossing.
    SyncAfterOffline,

    /// User seeking advice from an elohim — boundary-crossing (because the
    /// elohim witnesses the framing).
    AdviceSeeking,

    // --- Exempt interiors (gate does NOT fire) ---
    /// Node is offline; no peer impact possible.
    Offline,

    /// Writes to local source chain only, not gossipped.
    PrivateDraftingInterior,

    /// Interior of a play-space, bounded audience.
    PlayInterior,

    /// Interior of a roleplay-space, explicitly fictional.
    RoleplayInterior,
}

impl SpaceType {
    /// Whether this space-type is an exempt interior (gate does not fire).
    pub fn is_exempt(&self) -> bool {
        matches!(
            self,
            SpaceType::Offline
                | SpaceType::PrivateDraftingInterior
                | SpaceType::PlayInterior
                | SpaceType::RoleplayInterior
        )
    }

    /// Whether this space-type is boundary-crossing (gate fires, summarization
    /// primitive applies to private context being surfaced).
    pub fn is_boundary_crossing(&self) -> bool {
        matches!(
            self,
            SpaceType::PrivateDraftingCrossing
                | SpaceType::PlayExiting
                | SpaceType::RoleplayExiting
                | SpaceType::SyncAfterOffline
                | SpaceType::AdviceSeeking
        )
    }
}

/// Context enriching a space-type with caller-declared mode flags.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(
    feature = "typescript",
    ts(
        export,
        export_to = "../../elohim-agent-sdk/src/gate-client/generated/"
    )
)]
#[serde(rename_all = "camelCase")]
pub struct SpaceContext {
    pub space_type: SpaceType,
    /// Caller-declared play-mode override (user in playful creation mode).
    pub play_mode: bool,
    /// Caller-declared roleplay-mode override (user in fictional identity).
    pub roleplay_mode: bool,
}

impl SpaceContext {
    pub fn is_exempt(&self) -> bool {
        self.space_type.is_exempt()
    }
}

/// Detect the space-type for an event, using the three signals per spec §1.5:
/// call-site marker, user-declared mode flags, target-based inference.
///
/// Phase 0: target-based inference only. Call-site markers and mode flags wire
/// in during Phase 1 when integration points declare their context.
pub fn detect_from_event(event: &RelationalImpactEvent) -> SpaceContext {
    // Target-based inference: most RelationalImpactEvent variants imply
    // boundary-crossing or public context by construction. PrivateToPublicCrossing
    // is explicitly named. AdviceSought routes through the advice-seeking space.
    let space_type = match event {
        RelationalImpactEvent::AdviceSought { .. } => SpaceType::AdviceSeeking,
        RelationalImpactEvent::SyncToPeers { .. } => SpaceType::SyncAfterOffline,
        RelationalImpactEvent::PrivateToPublicCrossing { .. } => SpaceType::PrivateDraftingCrossing,
        _ => SpaceType::Public,
    };

    SpaceContext {
        space_type,
        play_mode: false,
        roleplay_mode: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Legacy consolidated tests (kept for regression) ─────────────────────

    #[test]
    fn exempt_interiors_are_exempt() {
        assert!(SpaceType::Offline.is_exempt());
        assert!(SpaceType::PrivateDraftingInterior.is_exempt());
        assert!(SpaceType::PlayInterior.is_exempt());
        assert!(SpaceType::RoleplayInterior.is_exempt());
    }

    #[test]
    fn boundary_crossing_is_not_exempt() {
        assert!(!SpaceType::PrivateDraftingCrossing.is_exempt());
        assert!(!SpaceType::PlayExiting.is_exempt());
        assert!(!SpaceType::Public.is_exempt());
    }

    #[test]
    fn advice_sought_infers_advice_seeking() {
        let event = RelationalImpactEvent::AdviceSought {
            requester: "agent-a".to_string(),
            summary_cid: "bafkrei...".to_string(),
            topic: "reflection".to_string(),
        };
        assert_eq!(
            detect_from_event(&event).space_type,
            SpaceType::AdviceSeeking
        );
    }

    // ─── Exempt interior variants — explicit per-variant ─────────────────────

    #[test]
    fn offline_is_exempt_not_boundary_crossing() {
        assert!(SpaceType::Offline.is_exempt());
        assert!(!SpaceType::Offline.is_boundary_crossing());
    }

    #[test]
    fn private_drafting_interior_is_exempt_not_boundary_crossing() {
        assert!(SpaceType::PrivateDraftingInterior.is_exempt());
        assert!(!SpaceType::PrivateDraftingInterior.is_boundary_crossing());
    }

    #[test]
    fn play_interior_is_exempt_not_boundary_crossing() {
        assert!(SpaceType::PlayInterior.is_exempt());
        assert!(!SpaceType::PlayInterior.is_boundary_crossing());
    }

    #[test]
    fn roleplay_interior_is_exempt_not_boundary_crossing() {
        assert!(SpaceType::RoleplayInterior.is_exempt());
        assert!(!SpaceType::RoleplayInterior.is_boundary_crossing());
    }

    // ─── Boundary-crossing variants — explicit per-variant ───────────────────

    #[test]
    fn private_drafting_crossing_is_boundary_crossing_not_exempt() {
        assert!(SpaceType::PrivateDraftingCrossing.is_boundary_crossing());
        assert!(!SpaceType::PrivateDraftingCrossing.is_exempt());
    }

    #[test]
    fn play_exiting_is_boundary_crossing_not_exempt() {
        assert!(SpaceType::PlayExiting.is_boundary_crossing());
        assert!(!SpaceType::PlayExiting.is_exempt());
    }

    #[test]
    fn roleplay_exiting_is_boundary_crossing_not_exempt() {
        assert!(SpaceType::RoleplayExiting.is_boundary_crossing());
        assert!(!SpaceType::RoleplayExiting.is_exempt());
    }

    #[test]
    fn sync_after_offline_is_boundary_crossing_not_exempt() {
        assert!(SpaceType::SyncAfterOffline.is_boundary_crossing());
        assert!(!SpaceType::SyncAfterOffline.is_exempt());
    }

    #[test]
    fn advice_seeking_is_boundary_crossing_not_exempt() {
        assert!(SpaceType::AdviceSeeking.is_boundary_crossing());
        assert!(!SpaceType::AdviceSeeking.is_exempt());
    }

    // ─── Public: neither exempt nor boundary-crossing ─────────────────────────
    //
    // `Public` is the "normal network activity" default. The gate fires (not
    // exempt), but it is not a *boundary-crossing* space per spec §1.5 —
    // boundary-crossing specifically refers to events that cross a privacy
    // boundary (offline→sync, draft→published, play→public).

    #[test]
    fn public_is_neither_exempt_nor_boundary_crossing() {
        assert!(!SpaceType::Public.is_exempt());
        assert!(!SpaceType::Public.is_boundary_crossing());
    }

    // ─── SpaceContext.is_exempt() delegates to SpaceType.is_exempt() ──────────

    #[test]
    fn space_context_is_exempt_delegates_to_space_type() {
        let exempt_ctx = SpaceContext {
            space_type: SpaceType::Offline,
            play_mode: false,
            roleplay_mode: false,
        };
        assert!(exempt_ctx.is_exempt());

        let non_exempt_ctx = SpaceContext {
            space_type: SpaceType::Public,
            play_mode: false,
            roleplay_mode: false,
        };
        assert!(!non_exempt_ctx.is_exempt());
    }

    // ─── detect_from_event: play_mode and roleplay_mode default to false ──────

    #[test]
    fn detect_from_event_always_returns_false_for_mode_flags() {
        // Phase 0: mode flags are not yet wired in. Confirm they are always false.
        let event = RelationalImpactEvent::ContentPublish {
            content_cid: "cid".to_string(),
            declared_reach: "public".to_string(),
            author: "agent".to_string(),
        };
        let ctx = detect_from_event(&event);
        assert!(!ctx.play_mode, "Phase 0: play_mode must be false");
        assert!(!ctx.roleplay_mode, "Phase 0: roleplay_mode must be false");
    }

    // ─── Serde round-trip for SpaceType ───────────────────────────────────────

    #[test]
    fn space_type_serde_round_trip() {
        let variants = [
            SpaceType::Public,
            SpaceType::PrivateDraftingCrossing,
            SpaceType::PlayExiting,
            SpaceType::RoleplayExiting,
            SpaceType::SyncAfterOffline,
            SpaceType::AdviceSeeking,
            SpaceType::Offline,
            SpaceType::PrivateDraftingInterior,
            SpaceType::PlayInterior,
            SpaceType::RoleplayInterior,
        ];
        for variant in &variants {
            let json = serde_json::to_string(variant).expect("serialize");
            let rt: SpaceType = serde_json::from_str(&json).expect("deserialize");
            assert_eq!(rt, *variant, "round-trip failed for {variant:?}");
        }
    }
}
