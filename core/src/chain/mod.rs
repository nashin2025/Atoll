//! Chain module for the Atoll Protocol.
//!
//! Blockchain state management, DAG structure with GHOSTDAG fork choice,
//! and beacon chain for shard coordination.

pub mod blockchain;
pub mod state;
pub mod dag;
pub mod beacon_chain;
