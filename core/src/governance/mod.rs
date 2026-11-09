//! Governance module for the Atoll Protocol.
//!
//! Implements on-chain governance with:
//! - Proposal creation with ATOLL deposit (100 ATOLL)
//! - Quadratic voting weighted by PROOF balance (sqrt)
//! - Voting periods: 7 days (standard), 30 days (constitutional)
//! - Approval thresholds: 51% (standard), 75% (constitutional)
//! - Quorum: 10% of PROOF holders must participate
//! - Timelock execution: 30 days (standard), 60 days (constitutional), 1 day (emergency)

pub mod ai_advisory;
pub mod futarchy;
pub mod proposals;
pub mod quadratic_voting;
pub mod timelock;

pub use proposals::{GovernanceEngine, Proposal, ProposalState, PROPOSAL_DEPOSIT, QUORUM_PERCENT};
pub use quadratic_voting::{quadratic_weight, voting_weight};
pub use timelock::{ProposalType, TimelockEntry, TimelockQueue};
