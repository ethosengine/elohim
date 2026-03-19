//! Resource Nature — Economic nature of resources in the REA ontology
//!
//! Classifies HOW a resource behaves economically, complementing
//! `ResourceCategory` (WHAT it is) and `ResourceClassification` (its domain).
//!
//! ## Dimensions
//!
//! - **Rivalry**: Does one agent's consumption prevent another's?
//! - **Excludability**: Can access be restricted?
//! - **Depletability**: Does stock diminish with use or replenish?
//! - **Fungibility**: Are units interchangeable?
//! - **CapacityModel**: Stock, flow, or both?
//! - **Circularity**: What happens at end-of-life? (linear → obligation, circular → loop closed)
//!
//! ## Classic Economics 2x2 (derived)
//!
//! ```text
//!                    Excludable         Non-excludable
//! Rival        │  Private good    │  Common-pool resource  │
//! Non-rival    │  Club good       │  Public good           │
//! ```
//!
//! ## Circularity Deficit Accumulator
//!
//! Consuming a `Linear` resource auto-generates circularity obligation tokens.
//! These accumulate in a governance-scoped pool, creating economic pressure to
//! build recycling/substitution capacity. When capacity comes online, the resource
//! spec upgrades and obligations stop generating. The system self-heals.
//!
//! ## Source of Truth
//!
//! This is a value object (embedded struct), not a standalone entity.
//! Source of truth is inherited from the parent entity:
//! - On `StewardedResource` → DHT (via parent's dht_anchor_hash)
//! - In protocol constants → protocol schema
//!
//! Protocol schema: `elohim/sdk/schemas/v1/objects/resource-nature.schema.json`

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// Whether consumption by one agent prevents consumption by another.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum Rivalry {
    /// One agent's use prevents another's (strawberry, kWh, seat)
    Rival,
    /// Use does not diminish availability (knowledge, digital content)
    NonRival,
    /// Congestible — degrades under heavy use (bandwidth, shared compute)
    PartiallyRival,
}

/// Whether access can be restricted.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum Excludability {
    /// Access can be restricted (fenced land, DRM content)
    Excludable,
    /// Cannot practically prevent access (clean air, public knowledge)
    NonExcludable,
    /// Excludable within governance structure (commons with Ostrom rules)
    ConditionallyExcludable,
}

/// Whether stock diminishes with use or replenishes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum Depletability {
    /// Finite stock, use reduces permanently (fossil fuel, mineral)
    Depletable,
    /// Replenishes on a cycle (solar flow, seasonal harvest)
    Renewable,
    /// Use can INCREASE future stock if managed correctly (permaculture, living soil)
    Regenerative,
    /// No stock concept (knowledge, digital content, reputation)
    NonDepletable,
}

/// Whether units are interchangeable.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum Fungibility {
    /// Any unit interchangeable with any other (kWh, bushel of grade-A wheat)
    Fungible,
    /// Each instance unique (painting, land parcel, specific attestation)
    NonFungible,
    /// Interchangeable within a grade/class but not across (Grade A vs Grade B eggs)
    ClassFungible,
}

/// Whether resource has a finite stock, continuous flow, or both.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum CapacityModel {
    /// Finite stock that depletes (mineral deposit)
    Stock,
    /// Continuous flow that doesn't accumulate (solar radiation)
    Flow,
    /// Both standing stock and inflow (reservoir, battery bank)
    StockAndFlow,
}

/// Lifecycle design intent — how value is expected to flow back after primary use.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub enum Circularity {
    /// Single-use, no designed recovery path. Generates circularity obligation tokens.
    Linear,
    /// Same function, multiple cycles (glass jar, tool, building)
    Reusable,
    /// Degrades through use tiers (EV battery → grid storage → mineral recycling)
    Cascading,
    /// Designed to return to feedstock (food → compost → soil → food)
    Circular,
}

/// Economic nature of a resource — how it behaves under use, transfer,
/// governance, and full lifecycle.
///
/// This is a value object embedded on `ResourceSpecification`, not a
/// standalone entity. No DHT entry type, no table.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export, export_to = "../../sdk/storage-client-ts/src/generated/")]
pub struct ResourceNature {
    pub rivalry: Rivalry,
    pub excludability: Excludability,
    pub depletability: Depletability,
    pub fungibility: Fungibility,
    /// Stock, flow, or both
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_model: Option<CapacityModel>,
    /// Lifecycle design intent — how value flows back after primary use
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circularity: Option<Circularity>,
    /// ResourceSpecification ID this becomes at end-of-life.
    /// Declared at creation, not disposal — the protocol makes end-of-life
    /// visible from the beginning. Absence on a rival+depletable resource
    /// triggers circularity obligation accumulation.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_of_life_resource_spec: Option<String>,
    /// Whether this resource can serve as input to a process producing a
    /// different resource type (energy → computation → governance decision)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transformable: Option<bool>,
}

/// Classic economics goods classification derived from rivalry × excludability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GoodsClassification {
    /// Rival + excludable (strawberry, private land)
    Private,
    /// Rival + non-excludable (fishery, aquifer) — needs Ostrom-style governance
    CommonPool,
    /// Non-rival + excludable (streaming service, gated content)
    Club,
    /// Non-rival + non-excludable (knowledge, clean air) — needs funding mechanism
    Public,
}

impl ResourceNature {
    /// Derive the classic economics goods classification.
    pub fn classify_goods(&self) -> GoodsClassification {
        let rival = self.rivalry != Rivalry::NonRival;
        let excludable = self.excludability != Excludability::NonExcludable;
        match (rival, excludable) {
            (true, true) => GoodsClassification::Private,
            (true, false) => GoodsClassification::CommonPool,
            (false, true) => GoodsClassification::Club,
            (false, false) => GoodsClassification::Public,
        }
    }

    /// Whether consuming this resource should generate circularity obligation tokens.
    pub fn requires_circularity_obligation(&self) -> bool {
        let is_linear = self.circularity.as_ref() == Some(&Circularity::Linear);
        let no_end_of_life = self.end_of_life_resource_spec.is_none();
        let is_rival = self.rivalry != Rivalry::NonRival;
        // Obligation triggers when: rival resource consumed linearly OR
        // rival+depletable resource consumed with no declared end-of-life path
        is_rival
            && (is_linear || (self.depletability == Depletability::Depletable && no_end_of_life))
    }

    /// Whether this resource requires observer/oracle attestation for usage events.
    pub fn requires_observer_attestation(&self) -> bool {
        self.rivalry != Rivalry::NonRival && self.depletability == Depletability::Depletable
    }

    /// Whether this resource requires consent-based allocation (Ostrom governance).
    pub fn requires_consent_allocation(&self) -> bool {
        self.classify_goods() == GoodsClassification::CommonPool
    }

    // ========================================================================
    // Temporal coupling — Schedule requirements derived from nature
    // ========================================================================

    /// Whether this resource nature requires a temporal replenishment schedule.
    /// Renewable flow resources (solar, seasonal harvest) need an RRULE defining
    /// when the flow replenishes so conservation constraints can enforce rate limits.
    pub fn requires_replenishment_schedule(&self) -> bool {
        self.depletability == Depletability::Renewable
            && self.capacity_model == Some(CapacityModel::Flow)
    }

    /// Whether this resource nature requires periodic degradation checks.
    /// Cascading resources (EV battery → grid storage) need scheduled evaluation
    /// to determine when to trigger the cascade to the end-of-life resource.
    pub fn requires_cascade_check(&self) -> bool {
        self.circularity == Some(Circularity::Cascading)
    }

    /// Whether this resource benefits from a stewardship maintenance schedule.
    /// Regenerative resources improve under active care — scheduled maintenance
    /// drives stewardship incentives.
    pub fn suggests_maintenance_schedule(&self) -> bool {
        self.depletability == Depletability::Regenerative
    }

    // ========================================================================
    // Spatial coupling — SpatialContext requirements derived from nature
    // ========================================================================

    /// Whether this resource requires spatial context for governance.
    /// Physical resources (energy, water, food, land) need spatial context
    /// to enforce bioregional carrying capacity limits and constitutional
    /// governance scoping.
    pub fn requires_spatial_context(&self) -> bool {
        self.rivalry != Rivalry::NonRival && self.depletability != Depletability::NonDepletable
    }
}
