//! Bitswap block exchange protocol for Elohim nodes.
//!
//! Forked from rust-ipfs `libp2p-bitswap-next` and ported to:
//! - `libp2p` 0.54 (from 0.53)
//! - `cid` 0.11 (from `libipld` 0.16)
//! - Simplified `BitswapStore` trait (no `StoreParams` generic)

mod behaviour;
mod protocol;
mod query;
mod stats;

pub use crate::behaviour::{Bitswap, BitswapConfig, BitswapEvent, BitswapStore, Channel};
pub use crate::query::QueryId;
