//! Chain module for the Atoll Protocol.
//!
//! Blockchain state management, DAG structure with GHOSTDAG fork choice,
//! beacon chain for shard coordination, and sharding for horizontal scaling.

pub mod beacon_chain;
pub mod blockchain;
pub mod dag;
pub mod sharding;
pub mod state;
