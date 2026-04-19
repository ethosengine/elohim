//! Space-type definitions — the first-class architectural boundary between
//! exempt interiors (offline / private-drafting / play) and network-impacting
//! zones the gate must wrap.
//!
//! Note: `detect_from_event` logic lives in `gate-client::space` because it
//! depends on `RelationalImpactEvent` — a pure-data type from this crate —
//! but the detection logic is gate-client's responsibility, not a data concern.
//!
//! Per spec §1.5: space-type is primarily a **context signal** fed into wisdom,
//! not a gate-control mechanism. Only the explicitly-exempt interiors short-
//! circuit the gate; every other space-type flows through the universal band
//! with the space signal included in context.

use serde::{Deserialize, Serialize};

#[cfg(feature = "typescript")]
use ts_rs::TS;

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
