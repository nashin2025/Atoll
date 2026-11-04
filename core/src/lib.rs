//! Atoll Protocol Core Library
//!
//! This crate contains the core protocol implementation for Atoll.
//!
//! Development workflow: Research → Implement → Test → PR

#![allow(missing_docs)]
#![warn(clippy::all)]
#![warn(clippy::pedantic)]

pub mod api;
pub mod chain;
pub mod config;
pub mod consensus;
pub mod crypto;
pub mod governance;
pub mod mempool;
pub mod network;
pub mod primitives;
pub mod privacy;
pub mod storage;
pub mod telemetry;
pub mod tokens;
pub mod vm;

// Protocol version
pub const CHIMERA_VERSION: u32 = 1;
// TODO(Atoll): Rename constant to ATOLL_VERSION after codebase rename sweep
pub const ATOLL_VERSION: u32 = 1;

/// Protocol name
pub const PROTOCOL_NAME: &str = "Atoll";

/// Chain ID for the Atoll network
pub const CHAIN_ID: u64 = 777777;

/// Genesis timestamp: 2027-01-01T00:00:00Z
pub const GENESIS_TIMESTAMP: u64 = 1798761600;

/// Target block time in seconds
pub const TARGET_BLOCK_TIME: u64 = 1;

/// Initial difficulty
pub const INITIAL_DIFFICULTY: u128 = 1 << 20;

#[cfg(test)]
mod tests {
    #[test]
    fn protocol_constants_are_valid() {
        assert_eq!(crate::PROTOCOL_NAME, "Atoll");
        assert_eq!(crate::CHAIN_ID, 777777);
        assert_eq!(crate::TARGET_BLOCK_TIME, 1);
    }
}
