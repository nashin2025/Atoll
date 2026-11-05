//! Encrypted Mempool for the Atoll Protocol.
//!
//! MEV-resistant transaction propagation using threshold encryption.
//! Transactions are encrypted before broadcast and only decrypted
//! after block proposer selection by a k-of-n committee.
//! Phase 1: Interface and simulation.
//! Phase 2: Full threshold encryption implementation.

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};

/// Encrypted transaction in the mempool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedTransaction {
    pub ciphertext: Vec<u8>,
    pub order_commitment: Hash512,
    pub fee_commitment: Hash512,
    pub submitter_proof: Vec<u8>,
    pub submitter: Address,
    pub received_at: u64,
}

/// Threshold decryption share
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DecryptionShare {
    pub tx_hash: Hash512,
    pub share: Vec<u8>,
    pub validator: Address,
    pub share_index: u32,
}

/// Mempool entry with encrypted transaction
#[derive(Debug, Clone)]
pub struct MempoolEntry {
    pub encrypted_tx: EncryptedTransaction,
    pub received_at: u64,
    pub peer_id: Hash512,
}

/// Encrypted mempool with threshold decryption
pub struct EncryptedMempool {
    pub threshold_public_key: Hash512,
    pub committee: Vec<Address>,
    pub threshold: usize,
    pub pending: HashMap<Hash512, MempoolEntry>,
    pub order_queue: VecDeque<Hash512>,
    pub decryption_shares: HashMap<Hash512, Vec<DecryptionShare>>,
    pub max_mempool_size: usize,
}

impl EncryptedMempool {
    /// Creates a new encrypted mempool
    pub fn new(
        threshold_public_key: Hash512,
        committee: Vec<Address>,
        threshold: usize,
    ) -> Self {
        Self {
            threshold_public_key,
            committee,
            threshold,
            pending: HashMap::new(),
            order_queue: VecDeque::new(),
            decryption_shares: HashMap::new(),
            max_mempool_size: 100_000,
        }
    }

    /// Adds an encrypted transaction to the mempool
    pub fn add_transaction(
        &mut self,
        tx: EncryptedTransaction,
        peer_id: Hash512,
    ) -> Result<(), String> {
        if self.pending.len() >= self.max_mempool_size {
            return Err("Mempool full".to_string());
        }

        let tx_hash = tx.order_commitment;
        if self.pending.contains_key(&tx_hash) {
            return Err("Transaction already in mempool".to_string());
        }

        let entry = MempoolEntry {
            encrypted_tx: tx.clone(),
            received_at: tx.received_at,
            peer_id,
        };

        self.pending.insert(tx_hash, entry);
        self.order_queue.push_back(tx_hash);

        Ok(())
    }

    /// Removes a transaction from the mempool
    pub fn remove_transaction(
        &mut self,
        tx_hash: &Hash512,
    ) -> Option<MempoolEntry> {
        let entry = self.pending.remove(tx_hash);
        self.order_queue.retain(|h| h != tx_hash);
        self.decryption_shares.remove(tx_hash);
        entry
    }

    /// Adds a decryption share for a transaction
    pub fn add_decryption_share(
        &mut self,
        share: DecryptionShare,
    ) -> Result<(), String> {
        if !self.pending.contains_key(&share.tx_hash) {
            return Err("Transaction not in mempool".to_string());
        }

        self.decryption_shares
            .entry(share.tx_hash)
            .or_insert_with(Vec::new)
            .push(share);

        Ok(())
    }

    /// Returns transactions ready for decryption (in order)
    pub fn get_ordered_transactions(&self) -> Vec<&Hash512> {
        self.order_queue
            .iter()
            .filter(|h| self.pending.contains_key(h))
            .collect()
    }

    /// Returns transactions that have enough decryption shares
    pub fn get_decryptable_transactions(&self) -> Vec<&Hash512> {
        self.get_ordered_transactions()
            .into_iter()
            .filter(|h| {
                self.decryption_shares
                    .get(*h)
                    .map(|shares| shares.len() >= self.threshold)
                    .unwrap_or(false)
            })
            .collect()
    }

    /// Returns the number of pending transactions
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Returns the threshold
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Returns the committee size
    pub fn committee_size(&self) -> usize {
        self.committee.len()
    }

    /// Returns the threshold public key
    pub fn threshold_public_key(&self) -> &Hash512 {
        &self.threshold_public_key
    }

    /// Checks if we have enough shares to decrypt
    pub fn can_decrypt(
        &self,
        tx_hash: &Hash512,
    ) -> bool {
        self.decryption_shares
            .get(tx_hash)
            .map(|shares| shares.len() >= self.threshold)
            .unwrap_or(false)
    }

    /// Returns decryption shares for a transaction
    pub fn get_decryption_shares(
        &self,
        tx_hash: &Hash512,
    ) -> Option<&Vec<DecryptionShare>> {
        self.decryption_shares.get(tx_hash)
    }

    /// Returns mempool statistics
    pub fn stats(&self) -> MempoolStats {
        MempoolStats {
            pending_count: self.pending.len(),
            decryptable_count: self.get_decryptable_transactions().len(),
            committee_size: self.committee.len(),
            threshold: self.threshold,
            max_size: self.max_mempool_size,
            utilization: self.pending.len() as f64 / self.max_mempool_size as f64,
        }
    }

    /// Sets the maximum mempool size
    pub fn set_max_size(
        &mut self,
        size: usize,
    ) {
        self.max_mempool_size = size;
    }

    /// Cleans up old transactions (older than max_age blocks)
    pub fn cleanup_old(
        &mut self,
        current_block: u64,
        max_age: u64,
    ) {
        let to_remove: Vec<Hash512> = self
            .pending
            .iter()
            .filter(|(_, entry)| current_block.saturating_sub(entry.received_at) > max_age)
            .map(|(hash, _)| *hash)
            .collect();

        for hash in to_remove {
            self.remove_transaction(&hash);
        }
    }
}

/// Mempool statistics
#[derive(Debug, Clone)]
pub struct MempoolStats {
    pub pending_count: usize,
    pub decryptable_count: usize,
    pub committee_size: usize,
    pub threshold: usize,
    pub max_size: usize,
    pub utilization: f64,
}

impl Default for EncryptedMempool {
    fn default() -> Self {
        Self::new(Hash512::zero(), vec![], 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove_transaction() {
        let mut mempool =
            EncryptedMempool::new(Hash512::zero(), vec![Address::from_bytes([1u8; 20])], 1);

        let tx = EncryptedTransaction {
            ciphertext: vec![1, 2, 3],
            order_commitment: Hash512::zero(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
            submitter: Address::from_bytes([5u8; 20]),
            received_at: 1000,
        };

        mempool.add_transaction(tx, Hash512::zero()).unwrap();
        assert_eq!(mempool.pending_count(), 1);

        mempool.remove_transaction(&Hash512::zero());
        assert_eq!(mempool.pending_count(), 0);
    }

    #[test]
    fn duplicate_transaction_rejected() {
        let mut mempool = EncryptedMempool::new(Hash512::zero(), vec![], 1);

        let tx = EncryptedTransaction {
            ciphertext: vec![1],
            order_commitment: Hash512::zero(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
            submitter: Address::zero(),
            received_at: 1000,
        };

        mempool
            .add_transaction(tx.clone(), Hash512::zero())
            .unwrap();
        let result = mempool.add_transaction(tx, Hash512::zero());
        assert!(result.is_err());
    }

    #[test]
    fn decryption_shares() {
        let mut mempool = EncryptedMempool::new(
            Hash512::zero(),
            vec![
                Address::from_bytes([1u8; 20]),
                Address::from_bytes([2u8; 20]),
                Address::from_bytes([3u8; 20]),
            ],
            2,
        );

        let tx = EncryptedTransaction {
            ciphertext: vec![1],
            order_commitment: Hash512::zero(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
            submitter: Address::zero(),
            received_at: 1000,
        };
        mempool.add_transaction(tx, Hash512::zero()).unwrap();
        let tx_hash = Hash512::zero();

        // Add shares one by one
        mempool
            .add_decryption_share(DecryptionShare {
                tx_hash,
                share: vec![1],
                validator: Address::from_bytes([1u8; 20]),
                share_index: 0,
            })
            .unwrap();

        assert!(!mempool.can_decrypt(&tx_hash));

        mempool
            .add_decryption_share(DecryptionShare {
                tx_hash,
                share: vec![2],
                validator: Address::from_bytes([2u8; 20]),
                share_index: 1,
            })
            .unwrap();

        assert!(mempool.can_decrypt(&tx_hash));
        assert_eq!(mempool.get_decryption_shares(&tx_hash).unwrap().len(), 2);
    }

    #[test]
    fn ordered_transactions() {
        let mut mempool = EncryptedMempool::new(Hash512::zero(), vec![], 1);

        let tx1 = EncryptedTransaction {
            ciphertext: vec![1],
            order_commitment: Hash512::from_bytes(&[1u8; 64]).unwrap(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
            submitter: Address::zero(),
            received_at: 1000,
        };
        let tx2 = EncryptedTransaction {
            ciphertext: vec![2],
            order_commitment: Hash512::from_bytes(&[2u8; 64]).unwrap(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
            submitter: Address::zero(),
            received_at: 1000,
        };

        mempool.add_transaction(tx1, Hash512::zero()).unwrap();
        mempool.add_transaction(tx2, Hash512::zero()).unwrap();

        let ordered = mempool.get_ordered_transactions();
        assert_eq!(ordered.len(), 2);
    }

    #[test]
    fn threshold_check() {
        let mempool = EncryptedMempool::new(
            Hash512::zero(),
            vec![
                Address::from_bytes([1u8; 20]),
                Address::from_bytes([2u8; 20]),
                Address::from_bytes([3u8; 20]),
            ],
            2,
        );
        assert_eq!(mempool.threshold(), 2);
        assert_eq!(mempool.committee_size(), 3);
    }

    #[test]
    fn mempool_stats() {
        let mut mempool = EncryptedMempool::new(
            Hash512::zero(),
            vec![
                Address::from_bytes([1u8; 20]),
                Address::from_bytes([2u8; 20]),
            ],
            1,
        );

        let tx = EncryptedTransaction {
            ciphertext: vec![1],
            order_commitment: Hash512::zero(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
            submitter: Address::zero(),
            received_at: 1000,
        };
        mempool.add_transaction(tx, Hash512::zero()).unwrap();

        let stats = mempool.stats();
        assert_eq!(stats.pending_count, 1);
        assert_eq!(stats.committee_size, 2);
        assert_eq!(stats.threshold, 1);
        assert_eq!(stats.max_size, 100_000);
    }

    #[test]
    fn cleanup_old() {
        let mut mempool = EncryptedMempool::new(Hash512::zero(), vec![], 1);

        let tx = EncryptedTransaction {
            ciphertext: vec![1],
            order_commitment: Hash512::zero(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
            submitter: Address::zero(),
            received_at: 100,
        };
        mempool.add_transaction(tx, Hash512::zero()).unwrap();
        assert_eq!(mempool.pending_count(), 1);

        // Clean up with max_age of 50 blocks, current block 200
        mempool.cleanup_old(200, 50);
        assert_eq!(mempool.pending_count(), 0);
    }

    #[test]
    fn set_max_size() {
        let mut mempool = EncryptedMempool::new(Hash512::zero(), vec![], 1);
        mempool.set_max_size(50);
        assert_eq!(mempool.stats().max_size, 50);
    }
}
