//! Gateway-Assisted Signing for Humans
//!
//! Enables humans without their own Holochain nodes to interact with the
//! network through the gateway. The gateway manages custodial agents that
//! sign on behalf of authenticated humans.
//!
//! # Architecture
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────┐
//! │                     Human (no node)                                  │
//! │                          │                                           │
//! │                     JWT Auth                                         │
//! │                          ▼                                           │
//! │  ┌─────────────────────────────────────────────────────────────┐    │
//! │  │                    Doorway Gateway                           │    │
//! │  │  ┌─────────────────┐    ┌─────────────────────────────┐     │    │
//! │  │  │ SigningService  │───▶│ CustodialAgentPool          │     │    │
//! │  │  │ (validates req) │    │ (manages signing agents)    │     │    │
//! │  │  └─────────────────┘    └──────────┬──────────────────┘     │    │
//! │  │                                    │                         │    │
//! │  │                         ┌──────────▼──────────────────┐     │    │
//! │  │                         │   Holochain Conductor       │     │    │
//! │  │                         │   (receives signed entries) │     │    │
//! │  │                         └─────────────────────────────┘     │    │
//! │  └─────────────────────────────────────────────────────────────┘    │
//! └─────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # Security Model
//!
//! - Humans authenticate via JWT (or other auth mechanism)
//! - Gateway verifies human identity before signing
//! - Each human gets a consistent agent pubkey (derived from identity)
//! - Gateway maintains audit log of all signed entries
//! - Rate limiting prevents abuse
//!
//! # Usage Modes
//!
//! 1. **Onboarding**: New humans use gateway signing until they have their own node
//! 2. **Fallback**: Humans with nodes can use gateway if their node is offline
//! 3. **Lightweight**: Mobile/web clients that can't run full nodes

pub mod service;
pub mod session;

pub use service::{SigningService, SigningConfig, SignRequest, SignResponse};
pub use session::{HumanSession, SessionStore};
