//! Encrypted mempool for MEV-resistant transaction propagation.
//!
//! Transactions are encrypted with a threshold public key before broadcast.
//! Only a k-of-n committee can collectively decrypt them after block proposal.

use crate::network::p2p::NodeId;
use crate::primitives::hash::Hash512;
use std::collections::{HashMap, VecDeque};
use std::time::SystemTime;

/// Encrypted transaction in the mempool
#[derive(Debug, Clone)]
pub struct MempoolEntry {
    pub encrypted_tx: crate::network::EncryptedTx,
    pub received_at: u64,
    pub peer_id: NodeId,
}

/// Encrypted mempool with threshold decryption
pub struct EncryptedMempool {
    pub threshold_public_key: [u8; 32],
    pub committee: Vec<NodeId>,
    pub threshold: usize,
    pub pending: HashMap<Hash512, MempoolEntry>,
    pub order_queue: VecDeque<Hash512>,
}

impl EncryptedMempool {
    /// Creates a new encrypted mempool
    pub fn new(
        threshold_public_key: [u8; 32],
        committee: Vec<NodeId>,
        threshold: usize,
    ) -> Self {
        Self {
            threshold_public_key,
            committee,
            threshold,
            pending: HashMap::new(),
            order_queue: VecDeque::new(),
        }
    }

    /// Adds an encrypted transaction to the mempool
    pub fn add_transaction(
        &mut self,
        tx: crate::network::EncryptedTx,
        peer_id: NodeId,
    ) {
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        let entry = MempoolEntry {
            encrypted_tx: tx.clone(),
            received_at: now,
            peer_id,
        };

        let tx_hash = tx.order_commitment;
        self.pending.insert(tx_hash, entry);
        self.order_queue.push_back(tx_hash);
    }

    /// Returns the number of pending transactions
    pub fn pending_count(&self) -> usize {
        self.pending.len()
    }

    /// Removes a transaction from the mempool (after decryption/inclusion)
    pub fn remove_transaction(
        &mut self,
        tx_hash: &Hash512,
    ) {
        self.pending.remove(tx_hash);
        // Remove from order queue (inefficient but simple for Phase 1)
        self.order_queue.retain(|h| h != tx_hash);
    }

    /// Returns transactions ready for decryption (in order)
    pub fn get_ordered_transactions(&self) -> Vec<&Hash512> {
        self.order_queue
            .iter()
            .filter(|h| self.pending.contains_key(h))
            .collect()
    }

    /// Returns the threshold public key
    pub fn threshold_public_key(&self) -> &[u8; 32] {
        &self.threshold_public_key
    }

    /// Returns the committee members
    pub fn committee(&self) -> &[NodeId] {
        &self.committee
    }

    /// Returns the decryption threshold (k-of-n)
    pub fn threshold(&self) -> usize {
        self.threshold
    }

    /// Checks if we have enough shares to decrypt
    pub fn can_decrypt(
        &self,
        shares: &[NodeId],
    ) -> bool {
        shares.len() >= self.threshold
    }
}

impl Default for EncryptedMempool {
    fn default() -> Self {
        Self::new([0u8; 32], vec![], 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_and_remove_transaction() {
        let mut mempool =
            EncryptedMempool::new([1u8; 32], vec![[2u8; 32], [3u8; 32], [4u8; 32]], 2);

        let tx = crate::network::EncryptedTx {
            ciphertext: vec![1, 2, 3],
            order_commitment: Hash512::zero(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
        };

        mempool.add_transaction(tx, [5u8; 32]);
        assert_eq!(mempool.pending_count(), 1);

        mempool.remove_transaction(&Hash512::zero());
        assert_eq!(mempool.pending_count(), 0);
    }

    #[test]
    fn threshold_check() {
        let mempool = EncryptedMempool::new([1u8; 32], vec![[1u8; 32], [2u8; 32], [3u8; 32]], 2);

        assert!(mempool.can_decrypt(&vec![[1u8; 32], [2u8; 32]]));
        assert!(!mempool.can_decrypt(&vec![[1u8; 32]]));
    }

    #[test]
    fn ordered_transactions() {
        let mut mempool = EncryptedMempool::new([1u8; 32], vec![], 1);

        let tx1 = crate::network::EncryptedTx {
            ciphertext: vec![1],
            order_commitment: Hash512::from_bytes(&[1u8; 64]).unwrap(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
        };
        let tx2 = crate::network::EncryptedTx {
            ciphertext: vec![2],
            order_commitment: Hash512::from_bytes(&[2u8; 64]).unwrap(),
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
        };

        mempool.add_transaction(tx1, [1u8; 32]);
        mempool.add_transaction(tx2, [2u8; 32]);

        let ordered = mempool.get_ordered_transactions();
        assert_eq!(ordered.len(), 2);
    }
}
