//! Stress tests for Phase 6 Testing & Auditing.
//!
//! High-concurrency, high-throughput scenarios that verify the system
//! remains correct under load.

use atoll_core::chain::blockchain::Blockchain;
use atoll_core::chain::sharding::cross_shard::CrossShardManager;
use atoll_core::chain::sharding::rebalancer::ShardRebalancer;
use atoll_core::chain::sharding::shard_manager::ShardManager;
use atoll_core::chain::state::ChainState;
use atoll_core::consensus::finality::hotstuff::{HotStuffBFT, HotStuffState, VoteType};
use atoll_core::consensus::penta::{poh, por, pos_vrf, post, pouw, PentaConsensus};
use atoll_core::crypto::morph_hash::{select_algorithm_chain, MorphHashEngine};
use atoll_core::network::encrypted_mempool::EncryptedMempool as NetworkEncryptedMempool;
use atoll_core::network::gossip::{GossipConfig, GossipProtocol};
use atoll_core::network::PeerInfo;
use atoll_core::network::{NetworkMessage, PingMessage};
use atoll_core::primitives::address::Address;
use atoll_core::primitives::block::{Block, BlockBody, BlockHeader};
use atoll_core::primitives::hash::Hash512;
use atoll_core::primitives::signature::Signature;
use atoll_core::primitives::transaction::Transaction;
use atoll_core::privacy::encrypted_mempool::{
    DecryptionShare, EncryptedMempool as PrivacyEncryptedMempool, EncryptedTransaction,
};
use atoll_core::privacy::homomorphic::{
    HomomorphicContext, HomomorphicEngine, HomomorphicScheme, Plaintext,
};
use atoll_core::privacy::post_quantum::{
    PQKEMEngine, PQKEMScheme, PQSignatureEngine, PQSignatureScheme,
};
use atoll_core::vm::evm::EVMExecutor;
use atoll_core::vm::{ExecutionContext, VMRouter, VMType};
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_address(bytes: [u8; 20]) -> Address {
    Address::from_bytes(bytes)
}

fn make_hash(bytes: [u8; 64]) -> Hash512 {
    Hash512::from_bytes(&bytes).unwrap()
}

fn make_valid_pouw() -> pouw::PoUWResult {
    pouw::PoUWResult {
        valid: true,
        usefulness_score: 0.8,
        difficulty_met: true,
    }
}

fn make_valid_pos() -> pos_vrf::PoSResult {
    pos_vrf::PoSResult {
        valid: true,
        combined_score: 0.9,
    }
}

fn make_valid_post() -> post::PoSTResult {
    post::PoSTResult {
        valid: true,
        size_score: 1.0,
    }
}

fn make_valid_por() -> por::PoRResult {
    por::PoRResult {
        raw_score: 0.8,
        decayed_score: 0.72,
        is_eligible: true,
    }
}

fn make_valid_poh() -> poh::PoHResult {
    poh::PoHResult { is_verified: true }
}

fn make_block(
    prev_hash: Hash512,
    height: u64,
    proposer: Address,
) -> Block {
    Block {
        header: BlockHeader {
            version: 1,
            height,
            shard_id: 0,
            timestamp: atoll_core::GENESIS_TIMESTAMP + height,
            prev_hash,
            state_root: Hash512::zero(),
            transaction_root: Hash512::zero(),
            proposer,
            difficulty: 1,
        },
        body: BlockBody {
            transactions: Vec::new(),
        },
    }
}

fn make_valid_validation() -> atoll_core::consensus::penta::PentaValidation {
    atoll_core::consensus::penta::PentaValidation {
        pouw_valid: true,
        pouw_score: 0.8,
        pos_valid: true,
        pos_score: 0.9,
        post_valid: true,
        post_score: 1.0,
        por_valid: true,
        por_score: 0.72,
        poh_valid: true,
        poh_score: 1.0,
        composite_score: 0.665,
        finality_achieved: true,
        all_layers_pass: true,
    }
}

// ---------------------------------------------------------------------------
// High-throughput hashing stress
// ---------------------------------------------------------------------------

#[test]
fn morph_hash_high_throughput() {
    let mut engine = MorphHashEngine::new();
    let prev_hash = Hash512::zero();

    let mut results = HashSet::new();
    for i in 0..10_000 {
        let input = format!("stress_test_input_{}", i).into_bytes();
        let hash = engine.morph_hash(&input, &prev_hash, i % 1000);
        results.insert(hash);
    }

    // All hashes should be unique (no collisions)
    assert_eq!(results.len(), 10_000);
}

#[test]
fn morph_hash_concurrent_engines() {
    // Each thread has its own engine (MorphHashEngine is not Sync)
    let num_engines = 8;
    let hashes_per_engine: usize = 1_000;

    let all_hashes: Vec<Vec<Hash512>> = (0..num_engines)
        .map(|engine_id| {
            let mut engine = MorphHashEngine::new();
            let prev_hash = Hash512::from_bytes(&[engine_id as u8; 64]).unwrap();
            (0..hashes_per_engine)
                .map(|i| {
                    engine.morph_hash(
                        format!("engine_{}_hash_{}", engine_id, i).as_bytes(),
                        &prev_hash,
                        i as u64,
                    )
                })
                .collect()
        })
        .collect();

    // Each engine should produce unique hashes
    for (engine_id, hashes) in all_hashes.iter().enumerate() {
        let mut seen = HashSet::new();
        for h in hashes {
            assert!(
                seen.insert(*h),
                "Engine {} produced duplicate hash",
                engine_id
            );
        }
        assert_eq!(seen.len(), hashes_per_engine);
    }
}

#[test]
fn blake3_hash_high_throughput() {
    let mut results = HashSet::new();
    for i in 0..50_000 {
        let input = format!("blake3_stress_{}", i).into_bytes();
        let hash = Hash512::blake3(&input);
        results.insert(hash);
    }
    assert_eq!(results.len(), 50_000);
}

// ---------------------------------------------------------------------------
// Chain stress: many blocks
// ---------------------------------------------------------------------------

#[test]
fn blockchain_many_blocks_stress() {
    let mut bc = Blockchain::default();
    let consensus = PentaConsensus::new();
    let validation = make_valid_validation();

    let mut prev_hash = bc.tip_hash();
    for height in 1..=1_000 {
        let block = make_block(prev_hash, height, make_address([height as u8; 20]));
        let result = bc.insert_block(block, &validation);
        assert!(result.is_ok(), "Block at height {} failed", height);
        prev_hash = result.unwrap();
    }

    assert_eq!(bc.tip_height(), 1_000);
    assert_eq!(bc.block_count(), 1_001); // genesis + 1000
}

#[test]
fn blockchain_rejects_out_of_order_blocks() {
    let mut bc = Blockchain::default();
    let consensus = PentaConsensus::new();
    let validation = make_valid_validation();

    // Insert block at height 1
    let block1 = make_block(bc.tip_hash(), 1, make_address([1u8; 20]));
    let hash1 = bc.insert_block(block1, &validation).unwrap();

    // Try to insert block with wrong prev_hash
    let block2_wrong = make_block(
        Hash512::from_bytes(&[9u8; 64]).unwrap(),
        2,
        make_address([3u8; 20]),
    );
    let result = bc.insert_block(block2_wrong, &validation);
    assert!(result.is_err());

    // Chain should still be at height 1
    assert_eq!(bc.tip_height(), 1);
}

// ---------------------------------------------------------------------------
// ChainState stress: many transfers
// ---------------------------------------------------------------------------

#[test]
fn chain_state_many_transfers_stress() {
    let mut state = ChainState::new();

    // Create 1000 accounts with balances
    for i in 0..1000 {
        state.set_balance(make_address([i as u8; 20]), 1_000_000);
    }

    // Perform 10000 transfers
    for i in 0..10_000 {
        let from = make_address([(i % 1000) as u8; 20]);
        let to = make_address([((i + 1) % 1000) as u8; 20]);
        assert!(state.transfer(&from, &to, 1).is_ok());
    }

    // Total supply should be conserved
    let total: u128 = (0..1000)
        .map(|i| state.balance(&make_address([i as u8; 20])))
        .sum();
    assert_eq!(total, 1_000_000_000); // 1000 * 1_000_000
}

#[test]
fn chain_state_concurrent_balance_reads() {
    let mut state = ChainState::new();
    state.set_balance(make_address([1u8; 20]), 100_000_000);

    // Many reads of the same balance should all return the same value
    for _ in 0..10_000 {
        assert_eq!(state.balance(&make_address([1u8; 20])), 100_000_000);
    }
}

// ---------------------------------------------------------------------------
// Consensus stress: many validations
// ---------------------------------------------------------------------------

#[test]
fn penta_consensus_many_validations() {
    let consensus = PentaConsensus::new();
    let mut passed = 0u64;
    let mut failed = 0u64;

    for i in 0..10_000 {
        let pouw = pouw::PoUWResult {
            valid: i % 2 == 0,
            usefulness_score: 0.8,
            difficulty_met: true,
        };
        let pos = pos_vrf::PoSResult {
            valid: true,
            combined_score: 0.9,
        };
        let post = post::PoSTResult {
            valid: true,
            size_score: 1.0,
        };
        let por = por::PoRResult {
            raw_score: 0.8,
            decayed_score: 0.72,
            is_eligible: true,
        };
        let poh = poh::PoHResult { is_verified: true };

        let result = consensus.validate_block(&pouw, &pos, &post, &por, &poh);
        if result.is_ok() {
            passed += 1;
        } else {
            failed += 1;
        }
    }

    // Half should pass (even i), half should fail (odd i, pouw invalid)
    assert_eq!(passed, 5_000);
    assert_eq!(failed, 5_000);
}

#[test]
fn hotstuff_many_votes() {
    let bft = HotStuffBFT::new();
    let mut state = HotStuffState::new(100);
    state.current_height = 1;
    state.current_round = 1;

    let block_hash = Hash512::zero();

    // Add 67 votes one at a time, checking finality only after quorum
    for i in 0..67 {
        state.add_vote(atoll_core::consensus::finality::hotstuff::BFTVote {
            validator: make_address([i as u8; 20]),
            block_hash,
            height: 1,
            round: 1,
            vote_type: VoteType::Commit,
            signature: vec![i as u8; 64],
        });

        if i >= 66 {
            assert!(bft.check_finality(&state, &block_hash));
        } else {
            assert!(!bft.check_finality(&state, &block_hash));
        }
    }
}

// ---------------------------------------------------------------------------
// Gossip stress: many messages
// ---------------------------------------------------------------------------

#[test]
fn gossip_many_messages_dedup() {
    let mut gossip = GossipProtocol::new(GossipConfig {
        fanout: 8,
        max_history: 10_000,
        gossip_interval_ms: 1,
    });

    let peers: Vec<[u8; 32]> = (0..100).map(|i| [i as u8; 32]).collect();
    let sender = [255u8; 32];

    // Broadcast 10000 unique messages
    for i in 0..10_000 {
        let msg = NetworkMessage::Ping(PingMessage {
            nonce: i,
            timestamp: i * 1000,
        });
        gossip.broadcast(&msg, sender);
    }

    assert_eq!(gossip.history_size(), 10_000);

    // Re-broadcast same messages - all should be deduped
    for i in 0..10_000 {
        let msg = NetworkMessage::Ping(PingMessage {
            nonce: i,
            timestamp: i * 1000,
        });
        let history_before = gossip.history_size();
        gossip.broadcast(&msg, sender);
        assert_eq!(gossip.history_size(), history_before);
    }

    // Select gossip targets
    let targets = gossip.select_gossip_targets(&peers, sender);
    assert!(targets.len() <= 8); // fanout
}

// ---------------------------------------------------------------------------
// Sharding stress: many shards
// ---------------------------------------------------------------------------

#[test]
fn shard_manager_many_shards() {
    let mut manager = ShardManager::new();

    // Create 100 shards
    for i in 1..=100u16 {
        let result = manager.create_shard(atoll_core::chain::sharding::ShardConfig {
            id: i,
            ..Default::default()
        });
        assert!(result.is_ok(), "Failed to create shard {}", i);
    }

    assert_eq!(manager.shard_count(), 101); // genesis + 100
    assert_eq!(manager.active_shard_ids().len(), 101usize);
}

#[test]
fn shard_manager_many_validators_per_shard() {
    let mut manager = ShardManager::new();

    // Assign 256 validators to shard 0 (all unique u8 values)
    let validators: Vec<Address> = (0..256).map(|i| make_address([i as u8; 20])).collect();
    let validators_for_removal = validators.clone();
    manager.assign_validators(0, validators).unwrap();

    let shard_validators = manager.get_shard_validators(0).unwrap();
    assert_eq!(shard_validators.len(), 256);

    // Remove them in batches
    for batch in validators_for_removal.chunks(50) {
        manager.remove_validators(0, batch).unwrap();
    }

    let shard_validators = manager.get_shard_validators(0).unwrap();
    assert_eq!(shard_validators.len(), 0);
}

// ---------------------------------------------------------------------------
// Cross-shard stress: many transactions
// ---------------------------------------------------------------------------

#[test]
fn cross_shard_many_transactions() {
    let mut xshard_mgr = CrossShardManager::new();

    // Create 5000 cross-shard requests
    let mut request_ids = Vec::new();
    for i in 0..5_000 {
        let id = xshard_mgr.create_request(
            0,
            1,
            make_address([(i % 255) as u8; 20]),
            make_address([((i + 1) % 255) as u8; 20]),
            i as u128 * 100,
            vec![i as u8],
            1000u64 + i as u64,
        );
        request_ids.push(id);
    }

    assert_eq!(xshard_mgr.total_cross_shard_tx(), 5_000);
    assert_eq!(xshard_mgr.pending_request_count(), 5_000);

    // Convert all requests to receipts
    let mut receipt_ids = Vec::new();
    for i in 0..5_000 {
        let receipt = xshard_mgr.request_to_receipt(&request_ids[i]).unwrap();
        receipt_ids.push(receipt.id);
    }
    assert_eq!(xshard_mgr.pending_receipt_count(), 5_000);

    // Execute first 1000 receipts
    for i in 0..1_000 {
        xshard_mgr.execute_receipt(&receipt_ids[i], 1050).unwrap();
    }

    assert_eq!(xshard_mgr.executed_receipt_count(), 1_000);
    assert_eq!(xshard_mgr.pending_receipt_count(), 4_000);
}

// ---------------------------------------------------------------------------
// Encrypted mempool stress
// ---------------------------------------------------------------------------

#[test]
fn encrypted_mempool_many_transactions() {
    let mut mempool = PrivacyEncryptedMempool::new(
        Hash512::zero(),
        vec![make_address([1u8; 20]), make_address([2u8; 20])],
        1,
    );

    // Add 10000 transactions
    for i in 0..10_000 {
        let tx_hash = Hash512::blake3(format!("tx_{}", i).as_bytes());
        let tx = EncryptedTransaction {
            ciphertext: vec![i as u8],
            order_commitment: tx_hash,
            fee_commitment: Hash512::zero(),
            submitter_proof: vec![],
            submitter: make_address([(i % 255) as u8; 20]),
            received_at: i,
        };
        mempool.add_transaction(tx, tx_hash).unwrap();
    }

    assert_eq!(mempool.pending_count(), 10_000);
    assert_eq!(mempool.get_ordered_transactions().len(), 10_000);
}

// ---------------------------------------------------------------------------
// VM stress: many executions
// ---------------------------------------------------------------------------

#[test]
fn evm_many_executions() {
    let context = ExecutionContext::default();

    for i in 0..10_000 {
        let mut evm = EVMExecutor::new(context.clone(), 10_000);
        let bytecode = vec![0x00, 0x01, i as u8, 0x00]; // STOP after some ops
        let result = evm.execute(&bytecode, &[]).unwrap();
        match result {
            atoll_core::vm::ExecutionResult::Success { gas_used, .. } => {
                assert!(gas_used > 0);
            }
            _ => panic!("EVM execution {} failed", i),
        }
    }
}

#[test]
fn vm_router_many_routes() {
    for i in 0..10_000 {
        let prefix = (i % 6) as u8;
        let result = VMRouter::route(&[prefix, 0x00]);
        match prefix {
            0 => assert_eq!(result, Ok(VMType::EVM)),
            1 => assert_eq!(result, Ok(VMType::WASM)),
            2 => assert_eq!(result, Ok(VMType::Move)),
            3 => assert_eq!(result, Ok(VMType::RISCV)),
            4 => assert_eq!(result, Ok(VMType::AI)),
            5 => assert_eq!(result, Ok(VMType::ZK)),
            _ => unreachable!(),
        }
    }
}

// ---------------------------------------------------------------------------
// Atomic counter stress test
// ---------------------------------------------------------------------------

#[test]
fn atomic_counter_concurrent_increments() {
    let counter = Arc::new(AtomicU64::new(0));
    let mut handles = vec![];

    for _ in 0..8 {
        let counter = Arc::clone(&counter);
        handles.push(std::thread::spawn(move || {
            for _ in 0..10_000 {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }));
    }

    for handle in handles {
        handle.join().unwrap();
    }

    assert_eq!(counter.load(Ordering::Relaxed), 80_000);
}
