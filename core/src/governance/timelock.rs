//! Timelock implementation for the Atoll Protocol.
//!
//! Proposals that pass voting must wait through a timelock before execution.
//! Timelock duration depends on proposal type:
//!   - Standard:   30 days
//!   - Constitutional: 60 days
//!   - Emergency:   1 day

use crate::primitives::address::Address;
use serde::{Deserialize, Serialize};

/// Type of governance proposal, determines voting period and timelock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProposalType {
    /// Standard parameter change or treasury allocation
    Standard,
    /// Constitutional change (higher threshold, longer timelock)
    Constitutional,
    /// Emergency pause/circuit breaker (shortest timelock)
    Emergency,
}

impl ProposalType {
    /// Returns the voting period in blocks (1-second blocks).
    pub fn voting_period_blocks(self) -> u64 {
        match self {
            ProposalType::Standard => 7 * 24 * 60 * 60, // 7 days
            ProposalType::Constitutional => 30 * 24 * 60 * 60, // 30 days
            ProposalType::Emergency => 1 * 24 * 60 * 60, // 1 day
        }
    }

    /// Returns the timelock in blocks before execution.
    pub fn timelock_blocks(self) -> u64 {
        match self {
            ProposalType::Standard => 30 * 24 * 60 * 60, // 30 days
            ProposalType::Constitutional => 60 * 24 * 60 * 60, // 60 days
            ProposalType::Emergency => 1 * 24 * 60 * 60, // 1 day
        }
    }

    /// Returns the approval threshold in basis points (1/100 of 1%).
    pub fn approval_threshold_bps(self) -> u16 {
        match self {
            ProposalType::Standard => 5100,       // 51%
            ProposalType::Constitutional => 7500, // 75%
            ProposalType::Emergency => 5100,      // 51%
        }
    }
}

/// Timelock queue entry for a passed proposal awaiting execution.
#[derive(Debug, Clone)]
pub struct TimelockEntry {
    pub proposal_id: u64,
    pub execute_at_block: u64,
    pub executed: bool,
}

/// Timelock queue manages pending proposal executions.
#[derive(Debug, Default, Clone)]
pub struct TimelockQueue {
    entries: Vec<TimelockEntry>,
    next_id: u64,
}

impl TimelockQueue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a proposal to the timelock queue.
    pub fn add(&mut self, proposal_id: u64, execute_at_block: u64) -> u64 {
        let id = self.next_id;
        self.entries.push(TimelockEntry {
            proposal_id,
            execute_at_block,
            executed: false,
        });
        self.next_id += 1;
        id
    }

    /// Get entries ready for execution at the given block.
    pub fn ready_for_execution(&self, current_block: u64) -> Vec<&TimelockEntry> {
        self.entries
            .iter()
            .filter(|e| !e.executed && e.execute_at_block <= current_block)
            .collect()
    }

    /// Mark an entry as executed.
    pub fn mark_executed(&mut self, entry_id: u64) -> Result<(), String> {
        let entry = self
            .entries
            .iter_mut()
            .find(|e| e.proposal_id == entry_id && !e.executed)
            .ok_or_else(|| "Timelock entry not found or already executed".to_string())?;
        entry.executed = true;
        Ok(())
    }

    /// Get all pending (not yet executed) entries.
    pub fn pending(&self) -> Vec<&TimelockEntry> {
        self.entries.iter().filter(|e| !e.executed).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_type_voting_periods() {
        assert_eq!(
            ProposalType::Standard.voting_period_blocks(),
            7 * 24 * 60 * 60
        );
        assert_eq!(
            ProposalType::Constitutional.voting_period_blocks(),
            30 * 24 * 60 * 60
        );
        assert_eq!(
            ProposalType::Emergency.voting_period_blocks(),
            1 * 24 * 60 * 60
        );
    }

    #[test]
    fn proposal_type_timelocks() {
        assert_eq!(ProposalType::Standard.timelock_blocks(), 30 * 24 * 60 * 60);
        assert_eq!(
            ProposalType::Constitutional.timelock_blocks(),
            60 * 24 * 60 * 60
        );
        assert_eq!(ProposalType::Emergency.timelock_blocks(), 1 * 24 * 60 * 60);
    }

    #[test]
    fn proposal_type_thresholds() {
        assert_eq!(ProposalType::Standard.approval_threshold_bps(), 5100);
        assert_eq!(ProposalType::Constitutional.approval_threshold_bps(), 7500);
        assert_eq!(ProposalType::Emergency.approval_threshold_bps(), 5100);
    }

    #[test]
    fn timelock_queue_add_and_ready() {
        let mut queue = TimelockQueue::new();
        let id = queue.add(42, 100);
        assert_eq!(id, 0);

        // Not ready at block 99
        assert!(queue.ready_for_execution(99).is_empty());

        // Ready at block 100
        let ready = queue.ready_for_execution(100);
        assert_eq!(ready.len(), 1);
        assert_eq!(ready[0].proposal_id, 42);
    }

    #[test]
    fn timelock_queue_mark_executed() {
        let mut queue = TimelockQueue::new();
        queue.add(42, 100);

        queue.mark_executed(42).unwrap();
        assert!(queue.ready_for_execution(100).is_empty());

        // Already executed
        assert!(queue.mark_executed(42).is_err());
    }

    #[test]
    fn timelock_queue_pending() {
        let mut queue = TimelockQueue::new();
        queue.add(1, 100);
        queue.add(2, 200);

        assert_eq!(queue.pending().len(), 2);
        queue.mark_executed(1).unwrap();
        assert_eq!(queue.pending().len(), 1);
    }
}
