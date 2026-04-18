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
/// See spec §P1.5 (Privacy, Drafting, Play, and Roleplay as Architectural
/// Primitives).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "typescript", derive(TS))]
#[cfg_attr(feature = "typescript", ts(export))]
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
#[cfg_attr(feature = "typescript", ts(export))]
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
        assert_eq!(detect_from_event(&event).space_type, SpaceType::AdviceSeeking);
    }
}
