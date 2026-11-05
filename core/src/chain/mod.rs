//! Chain module for the Atoll Protocol.
//!
//! Blockchain state management, DAG structure with GHOSTDAG fork choice,
//! beacon chain for shard coordination, genesis finalization, and sharding for
//! horizontal scaling.

pub mod beacon_chain;
pub mod blockchain;
pub mod dag;
pub mod genesis;
pub mod sharding;
pub mod state;
