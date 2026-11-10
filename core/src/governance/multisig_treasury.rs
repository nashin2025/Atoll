//! Multi-signature treasury module for the Atoll Protocol.
//!
//! Implements a Gnosis Safe-style multi-signature wallet where multiple
//! owners must confirm transactions before they can be executed.
//!
//! Key features:
//! - Threshold signatures (configurable M-of-N)
//! - Propose-execute pattern
//! - Owner management (add/remove owners, change threshold)
//! - Transaction tracking (pending, confirmed, executed, rejected)
//! - Treasury balance tracking

use crate::chain::state::ChainState;
use crate::primitives::address::Address;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Transaction status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TxStatus {
    /// Transaction proposed, awaiting confirmations
    Proposed,
    /// Transaction has enough confirmations, ready to execute
    Confirmed,
    /// Transaction executed
    Executed,
    /// Transaction rejected (cancelled)
    Rejected,
}

/// A multi-signature transaction
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MultiSigTransaction {
    /// Unique transaction ID
    pub id: u64,
    /// Recipient address
    pub to: Address,
    /// Amount to transfer (in smallest ATOLL unit)
    pub value: u128,
    /// Data payload (for contract calls)
    pub data: Vec<u8>,
    /// Transaction status
    pub status: TxStatus,
    /// Address that proposed the transaction
    pub proposer: Address,
    /// Addresses that have confirmed the transaction
    pub confirmations: HashSet<Address>,
    /// Block height when transaction was proposed
    pub created_at: u64,
    /// Block height when transaction was executed
    pub executed_at: Option<u64>,
}

/// Multi-signature treasury
#[derive(Debug, Clone)]
pub struct MultiSigTreasury {
    /// Treasury address (the multi-sig contract address)
    pub treasury_address: Address,
    /// Owners who can propose and confirm transactions
    pub owners: HashSet<Address>,
    /// Number of confirmations required to execute
    pub threshold: u64,
    /// All transactions
    pub transactions: HashMap<u64, MultiSigTransaction>,
    /// Next transaction ID
    pub next_tx_id: u64,
}

impl MultiSigTreasury {
    /// Create a new multi-signature treasury
    pub fn new(treasury_address: Address, owners: Vec<Address>, threshold: u64) -> Result<Self, String> {
        if owners.is_empty() {
            return Err("Treasury must have at least one owner".to_string());
        }
        if threshold == 0 || threshold > owners.len() as u64 {
            return Err("Threshold must be between 1 and number of owners".to_string());
        }
        if !owners.iter().all(|o| *o != Address::zero()) {
            return Err("Owner address cannot be zero".to_string());
        }

        Ok(Self {
            treasury_address,
            owners: owners.into_iter().collect(),
            threshold,
            transactions: HashMap::new(),
            next_tx_id: 1,
        })
    }

    /// Propose a new transaction
    pub fn propose_transaction(
        &mut self,
        proposer: Address,
        to: Address,
        value: u128,
        data: Vec<u8>,
        current_block: u64,
    ) -> Result<u64, String> {
        // Only owners can propose
        if !self.owners.contains(&proposer) {
            return Err("Only owners can propose transactions".to_string());
        }

        let tx_id = self.next_tx_id;
        self.next_tx_id += 1;

        let tx = MultiSigTransaction {
            id: tx_id,
            to,
            value,
            data,
            status: TxStatus::Proposed,
            proposer,
            confirmations: HashSet::new(),
            created_at: current_block,
            executed_at: None,
        };

        self.transactions.insert(tx_id, tx);
        Ok(tx_id)
    }

    /// Confirm a proposed transaction
    pub fn confirm_transaction(
        &mut self,
        owner: Address,
        tx_id: u64,
    ) -> Result<(), String> {
        // Only owners can confirm
        if !self.owners.contains(&owner) {
            return Err("Only owners can confirm transactions".to_string());
        }

        let tx = self.transactions.get_mut(&tx_id).ok_or_else(|| {
            format!("Transaction {} not found", tx_id)
        })?;

        // Cannot confirm executed or rejected transactions
        if tx.status != TxStatus::Proposed {
            return Err("Transaction is not in proposed state".to_string());
        }

        // Owner cannot confirm their own proposal (prevents self-dealing)
        if tx.proposer == owner {
            return Err("Proposer cannot confirm their own transaction".to_string());
        }

        // Add confirmation
        tx.confirmations.insert(owner);

        // Check if threshold reached
        if tx.confirmations.len() as u64 >= self.threshold {
            tx.status = TxStatus::Confirmed;
        }

        Ok(())
    }

    /// Execute a confirmed transaction
    pub fn execute_transaction(
        &mut self,
        executor: Address,
        tx_id: u64,
        current_block: u64,
        state: &mut ChainState,
    ) -> Result<(), String> {
        // Only owners can execute
        if !self.owners.contains(&executor) {
            return Err("Only owners can execute transactions".to_string());
        }

        let tx = self.transactions.get_mut(&tx_id).ok_or_else(|| {
            format!("Transaction {} not found", tx_id)
        })?;

        // Must be confirmed before execution
        if tx.status != TxStatus::Confirmed {
            return Err("Transaction is not confirmed".to_string());
        }

        // Check treasury has sufficient balance
        let treasury_balance = state.balance(&self.treasury_address);
        if treasury_balance < tx.value {
            return Err("Insufficient treasury balance".to_string());
        }

        // Execute the transfer atomically: burn from treasury, then mint to recipient.
        // If mint fails after burn, compensate by re-minting to treasury.
        state.burn_atoll(&self.treasury_address, tx.value)?;
        if let Err(e) = state.mint_atoll(tx.to, tx.value) {
            // Compensating transaction: refund the burn if mint fails
            let _ = state.mint_atoll(self.treasury_address, tx.value);
            return Err(e);
        }

        // Mark as executed
        tx.status = TxStatus::Executed;
        tx.executed_at = Some(current_block);

        Ok(())
    }

    /// Reject a proposed transaction
    pub fn reject_transaction(
        &mut self,
        owner: Address,
        tx_id: u64,
    ) -> Result<(), String> {
        // Only owners can reject
        if !self.owners.contains(&owner) {
            return Err("Only owners can reject transactions".to_string());
        }

        // Get transaction and check conditions before mutable borrow
        let tx = self.transactions.get(&tx_id).ok_or_else(|| {
            format!("Transaction {} not found", tx_id)
        })?;

        // Only proposer or owner can reject
        if tx.proposer != owner && !self.is_owner(&owner) {
            return Err("Only proposer or owner can reject".to_string());
        }

        // Cannot reject executed transactions
        if tx.status == TxStatus::Executed {
            return Err("Cannot reject executed transaction".to_string());
        }

        // Now get mutable borrow and reject
        let tx = self.transactions.get_mut(&tx_id).ok_or_else(|| {
            format!("Transaction {} not found", tx_id)
        })?;
        tx.status = TxStatus::Rejected;
        Ok(())
    }

    /// Add a new owner
    pub fn add_owner(&mut self, new_owner: Address) -> Result<(), String> {
        if new_owner == Address::zero() {
            return Err("Owner cannot be zero address".to_string());
        }
        if self.owners.contains(&new_owner) {
            return Err("Owner already exists".to_string());
        }
        self.owners.insert(new_owner);
        Ok(())
    }

    /// Remove an owner
    pub fn remove_owner(&mut self, owner: Address) -> Result<(), String> {
        if !self.owners.contains(&owner) {
            return Err("Owner not found".to_string());
        }
        if self.owners.len() <= 1 {
            return Err("Cannot remove last owner".to_string());
        }
        self.owners.remove(&owner);

        // Adjust threshold if needed
        if self.threshold > self.owners.len() as u64 {
            self.threshold = self.owners.len() as u64;
        }

        Ok(())
    }

    /// Change the threshold
    pub fn change_threshold(&mut self, new_threshold: u64) -> Result<(), String> {
        if new_threshold == 0 || new_threshold > self.owners.len() as u64 {
            return Err("Threshold must be between 1 and number of owners".to_string());
        }
        self.threshold = new_threshold;
        Ok(())
    }

    /// Get transaction by ID
    pub fn get_transaction(&self, tx_id: u64) -> Option<&MultiSigTransaction> {
        self.transactions.get(&tx_id)
    }

    /// Get all transactions
    pub fn get_all_transactions(&self) -> Vec<&MultiSigTransaction> {
        self.transactions.values().collect()
    }

    /// Get pending transactions
    pub fn get_pending_transactions(&self) -> Vec<&MultiSigTransaction> {
        self.transactions
            .values()
            .filter(|t| t.status == TxStatus::Proposed)
            .collect()
    }

    /// Get confirmed transactions ready for execution
    pub fn get_confirmed_transactions(&self) -> Vec<&MultiSigTransaction> {
        self.transactions
            .values()
            .filter(|t| t.status == TxStatus::Confirmed)
            .collect()
    }

    /// Check if address is an owner
    pub fn is_owner(&self, address: &Address) -> bool {
        self.owners.contains(address)
    }

    /// Get treasury balance
    pub fn get_treasury_balance(&self, state: &ChainState) -> u128 {
        state.balance(&self.treasury_address)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::chain::state::ChainState;
    use crate::primitives::address::Address;

    fn create_test_treasury() -> MultiSigTreasury {
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let treasury = Address::from_bytes([4u8; 20]);

        MultiSigTreasury::new(treasury, vec![owner1, owner2, owner3], 2).unwrap()
    }

    #[test]
    fn test_create_treasury() {
        let treasury = create_test_treasury();
        assert_eq!(treasury.owners.len(), 3);
        assert_eq!(treasury.threshold, 2);
        assert_eq!(treasury.next_tx_id, 1);
    }

    #[test]
    fn test_create_treasury_invalid_threshold() {
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let treasury = Address::from_bytes([4u8; 20]);

        // Threshold 0 should fail
        assert!(MultiSigTreasury::new(treasury, vec![owner1, owner2], 0).is_err());
        // Threshold > owners should fail
        assert!(MultiSigTreasury::new(treasury, vec![owner1, owner2], 3).is_err());
    }

    #[test]
    fn test_propose_and_confirm_transaction() {
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        // Owner 1 proposes transaction
        let tx_id = treasury.propose_transaction(
            owner1,
            recipient,
            1000,
            vec![],
            0,
        ).unwrap();
        assert_eq!(tx_id, 1);

        let tx = treasury.get_transaction(tx_id).unwrap();
        assert_eq!(tx.status, TxStatus::Proposed);
        assert_eq!(tx.confirmations.len(), 0);

        // Owner 2 confirms (first confirmation - threshold is 2)
        treasury.confirm_transaction(owner2, tx_id).unwrap();
        let tx = treasury.get_transaction(tx_id).unwrap();
        assert_eq!(tx.status, TxStatus::Proposed); // Still proposed, need 2 confirmations
        assert!(tx.confirmations.contains(&owner2));

        // Owner 3 confirms (second confirmation - meets threshold)
        treasury.confirm_transaction(owner3, tx_id).unwrap();
        let tx = treasury.get_transaction(tx_id).unwrap();
        assert_eq!(tx.status, TxStatus::Confirmed); // Now confirmed with 2 confirmations
        assert!(tx.confirmations.contains(&owner2));
        assert!(tx.confirmations.contains(&owner3));
    }

    #[test]
    fn test_proposer_cannot_confirm_own_transaction() {
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        let tx_id = treasury.propose_transaction(
            owner1,
            recipient,
            1000,
            vec![],
            0,
        ).unwrap();

        // Owner 1 cannot confirm their own transaction
        let result = treasury.confirm_transaction(owner1, tx_id);
        assert!(result.is_err());

        // Owner 2 can confirm (but threshold is 2, so still proposed)
        treasury.confirm_transaction(owner2, tx_id).unwrap();
        let tx = treasury.get_transaction(tx_id).unwrap();
        assert_eq!(tx.status, TxStatus::Proposed); // Need 2 confirmations
        assert_eq!(tx.confirmations.len(), 1);
    }

    #[test]
    fn test_execute_transaction() {
        let mut state = ChainState::new();
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        // Fund treasury
        state.mint_atoll(treasury.treasury_address, 10000).unwrap();

        // Propose transaction
        let tx_id = treasury.propose_transaction(
            owner1,
            recipient,
            1000,
            vec![],
            0,
        ).unwrap();

        // Confirm with two owners (threshold = 2)
        treasury.confirm_transaction(owner2, tx_id).unwrap();
        treasury.confirm_transaction(owner3, tx_id).unwrap();

        // Execute
        treasury.execute_transaction(owner1, tx_id, 0, &mut state).unwrap();

        // Verify balances
        assert_eq!(state.balance(&recipient), 1000);
        assert_eq!(state.balance(&treasury.treasury_address), 9000);

        let tx = treasury.get_transaction(tx_id).unwrap();
        assert_eq!(tx.status, TxStatus::Executed);
        assert!(tx.executed_at.is_some());
    }

    #[test]
    fn test_execute_insufficient_balance() {
        let mut state = ChainState::new();
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        let tx_id = treasury.propose_transaction(
            owner1,
            recipient,
            1000000, // More than treasury has
            vec![],
            0,
        ).unwrap();
        treasury.confirm_transaction(owner2, tx_id).unwrap();
        treasury.confirm_transaction(owner3, tx_id).unwrap();

        // Should fail due to insufficient balance
        let result = treasury.execute_transaction(owner1, tx_id, 0, &mut state);
        assert!(result.is_err());
    }

    #[test]
    fn test_reject_transaction() {
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        // Owner 1 proposes a transaction
        let tx_id = treasury.propose_transaction(
            owner1,
            recipient,
            1000,
            vec![],
            0,
        ).unwrap();

        // Owner 3 (non-proposer, non-owner is owner3? owner3 IS an owner) can reject
        // Actually owner3 is an owner, so can reject
        treasury.reject_transaction(owner3, tx_id).unwrap();
        let tx = treasury.get_transaction(tx_id).unwrap();
        assert_eq!(tx.status, TxStatus::Rejected);
    }

    #[test]
    fn test_reject_transaction_non_owner_cannot_reject() {
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let non_owner = Address::from_bytes([9u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        let tx_id = treasury.propose_transaction(
            owner1,
            recipient,
            1000,
            vec![],
            0,
        ).unwrap();

        // Non-owner cannot reject
        let result = treasury.reject_transaction(non_owner, tx_id);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Only owners can reject transactions".to_string());
    }

    #[test]
    fn test_reject_transaction_proposer_can_reject() {
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        let tx_id = treasury.propose_transaction(
            owner1,
            recipient,
            1000,
            vec![],
            0,
        ).unwrap();

        // Proposer (owner1) can reject their own transaction
        treasury.reject_transaction(owner1, tx_id).unwrap();
        let tx = treasury.get_transaction(tx_id).unwrap();
        assert_eq!(tx.status, TxStatus::Rejected);
    }

    #[test]
    fn test_reject_executed_transaction_fails() {
        let mut state = ChainState::new();
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        state.mint_atoll(treasury.treasury_address, 10000).unwrap();

        let tx_id = treasury.propose_transaction(
            owner1,
            recipient,
            1000,
            vec![],
            0,
        ).unwrap();
        treasury.confirm_transaction(owner2, tx_id).unwrap();
        treasury.confirm_transaction(owner3, tx_id).unwrap();
        treasury.execute_transaction(owner1, tx_id, 0, &mut state).unwrap();

        // Cannot reject executed transaction
        let result = treasury.reject_transaction(owner2, tx_id);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Cannot reject executed transaction".to_string());
    }

    #[test]
    fn test_add_and_remove_owner() {
        let mut treasury = create_test_treasury();
        let owner4 = Address::from_bytes([4u8; 20]);

        // Add new owner
        treasury.add_owner(owner4).unwrap();
        assert!(treasury.is_owner(&owner4));
        assert_eq!(treasury.owners.len(), 4);

        // Remove owner
        treasury.remove_owner(owner4).unwrap();
        assert!(!treasury.is_owner(&owner4));
        assert_eq!(treasury.owners.len(), 3);
    }

    #[test]
    fn test_change_threshold() {
        let mut treasury = create_test_treasury();

        // Change threshold from 2 to 3
        treasury.change_threshold(3).unwrap();
        assert_eq!(treasury.threshold, 3);

        // Cannot set threshold > owners
        assert!(treasury.change_threshold(4).is_err());
        // Cannot set threshold to 0
        assert!(treasury.change_threshold(0).is_err());
    }

    #[test]
    fn test_get_pending_transactions() {
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        // Propose two transactions
        let tx1 = treasury.propose_transaction(owner1, recipient, 1000, vec![], 0).unwrap();
        let tx2 = treasury.propose_transaction(owner1, recipient, 2000, vec![], 0).unwrap();

        // Both should be pending
        let pending = treasury.get_pending_transactions();
        assert_eq!(pending.len(), 2);

        // Confirm tx1 with two owners (threshold = 2)
        treasury.confirm_transaction(owner2, tx1).unwrap();
        treasury.confirm_transaction(owner3, tx1).unwrap();

        // Only tx2 should be pending
        let pending = treasury.get_pending_transactions();
        assert_eq!(pending.len(), 1);
        assert_eq!(pending[0].id, tx2);
    }

    #[test]
    fn test_get_confirmed_transactions() {
        let mut treasury = create_test_treasury();
        let owner1 = Address::from_bytes([1u8; 20]);
        let owner2 = Address::from_bytes([2u8; 20]);
        let owner3 = Address::from_bytes([3u8; 20]);
        let recipient = Address::from_bytes([5u8; 20]);

        let tx1 = treasury.propose_transaction(owner1, recipient, 1000, vec![], 0).unwrap();
        let tx2 = treasury.propose_transaction(owner1, recipient, 2000, vec![], 0).unwrap();

        // Confirm tx1 with two owners
        treasury.confirm_transaction(owner2, tx1).unwrap();
        treasury.confirm_transaction(owner3, tx1).unwrap();

        let confirmed = treasury.get_confirmed_transactions();
        assert_eq!(confirmed.len(), 1);
        assert_eq!(confirmed[0].id, tx1);
    }
}
