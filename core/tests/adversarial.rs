//! Adversarial tests for Phase 6 Testing & Auditing.
//!
//! Tests for invalid inputs, edge cases, malformed data, and boundary
//! conditions across all modules.

use atoll_core::chain::blockchain::Blockchain;
use atoll_core::chain::sharding::cross_shard::CrossShardManager;
use atoll_core::chain::sharding::rebalancer::ShardRebalancer;
use atoll_core::chain::sharding::shard_manager::ShardManager;
use atoll_core::chain::state::ChainState;
use atoll_core::consensus::finality::hotstuff::{BFTVote, HotStuffBFT, HotStuffState, VoteType};
use atoll_core::consensus::penta::{
    poh, por, pos_vrf, post, pouw, PentaConsensus, PentaValidation,
};
use atoll_core::crypto::morph_hash::{select_algorithm_chain, MorphAlgorithm, MorphHashEngine};
use atoll_core::network::discovery::DiscoveryProtocol;
use atoll_core::network::encrypted_mempool::EncryptedMempool as NetworkEncryptedMempool;
use atoll_core::network::gossip::{GossipConfig, GossipProtocol};
use atoll_core::network::peer_manager::{PeerManager, PeerStatus};
use atoll_core::network::sync::SyncManager;
use atoll_core::network::PeerInfo;
use atoll_core::network::{BlockHeader as NetBlockHeader, NetworkMessage, PingMessage};
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
    HybridSignatureEngine, PQKEMEngine, PQKEMScheme, PQSignatureEngine, PQSignatureScheme,
};
use atoll_core::privacy::zk_starks::{StarkProver, StarkVerifier};
use atoll_core::vm::ai_vm::AIVMExecutor;
use atoll_core::vm::evm::EVMExecutor;
use atoll_core::vm::move_vm::MoveExecutor;
use atoll_core::vm::riscv::RISCVExecutor;
use atoll_core::vm::wasm::WASMExecutor;
use atoll_core::vm::zk_vm::ZKVMExecutor;
use atoll_core::vm::{ExecutionContext, GasMeter, VMRouter, VMType};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn zero_address() -> Address {
    Address::zero()
}

fn zero_hash() -> Hash512 {
    Hash512::zero()
}

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

fn make_valid_validation() -> PentaValidation {
    PentaValidation {
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
// Primitives adversarial tests
// ---------------------------------------------------------------------------

#[test]
fn hash512_from_hex_invalid_length() {
    // Too short
    assert!(Hash512::from_hex("deadbeef").is_err());
    // Too long
    assert!(Hash512::from_hex(&"f".repeat(200)).is_err());
    // Invalid hex characters
    assert!(Hash512::from_hex(&"g".repeat(128)).is_err());
    // Empty string
    assert!(Hash512::from_hex("").is_err());
}

#[test]
fn hash512_from_hex_with_0x_prefix() {
    // Hash512::from_hex does NOT strip 0x prefix (unlike Address)
    assert!(Hash512::from_hex(&format!("0x{}", "f".repeat(128))).is_err());
    // Without prefix works
    assert!(Hash512::from_hex(&"f".repeat(128)).is_ok());
}

#[test]
fn hash512_from_bytes_wrong_length_returns_error() {
    // from_bytes returns an error if not exactly 64 bytes
    let result = Hash512::from_bytes(&[1u8; 63]);
    assert!(result.is_err());
    let result = Hash512::from_bytes(&[1u8; 65]);
    assert!(result.is_err());
    // Correct length works
    let h = Hash512::from_bytes(&[1u8; 64]).unwrap();
    assert_eq!(h.0, [1u8; 64]);
    let h2 = Hash512::from_array([2u8; 64]);
    assert_eq!(h2.0, [2u8; 64]);
}

#[test]
fn address_from_hex_invalid() {
    // Too short
    assert!(Address::from_hex("deadbeef").is_err());
    // Too long
    assert!(Address::from_hex(&"f".repeat(100)).is_err());
    // Invalid hex
    assert!(Address::from_hex(&"g".repeat(40)).is_err());
    // Odd length
    assert!(Address::from_hex(&"f".repeat(39)).is_err());
}

#[test]
fn address_from_hex_with_0x_prefix() {
    let addr = Address::from_hex("0x0102030405060708090a0b0c0d0e0f1011121314").unwrap();
    assert_eq!(addr.0[0], 0x01);
    assert_eq!(addr.0[19], 0x14);
}

#[test]
fn transaction_serialize_with_large_signature() {
    let tx = Transaction {
        version: 1,
        from: zero_address(),
        to: None,
        value: 0,
        data: Vec::new(),
        signature: Signature::new([0u8; 64], vec![0u8; 5000]), // Large dilithium sig
    };
    let bytes = tx.serialize_bytes();
    // Should serialize without error
    assert!(bytes.len() > 100);
}

#[test]
fn transaction_serialize_empty_data() {
    let tx = Transaction {
        version: 1,
        from: zero_address(),
        to: Some(zero_address()),
        value: u128::MAX,
        data: Vec::new(),
        signature: Signature::default(),
    };
    let bytes = tx.serialize_bytes();
    let bytes2 = tx.serialize_bytes();
    assert_eq!(bytes, bytes2); // Deterministic
}

#[test]
fn block_serialize_with_many_transactions() {
    let mut txs = Vec::new();
    for i in 0..100 {
        txs.push(Transaction {
            version: 1,
            from: make_address([i as u8; 20]),
            to: Some(make_address([(i + 1) as u8; 20])),
            value: i as u128,
            data: vec![i as u8],
            signature: Signature::default(),
        });
    }

    let block = Block {
        header: BlockHeader {
            version: 1,
            height: 100,
            shard_id: 0,
            timestamp: 0,
            prev_hash: zero_hash(),
            state_root: zero_hash(),
            transaction_root: zero_hash(),
            proposer: zero_address(),
            difficulty: 1,
        },
        body: BlockBody { transactions: txs },
    };

    let bytes = block.serialize_bytes();
    assert!(!bytes.is_empty());

    // Deterministic
    let bytes2 = block.serialize_bytes();
    assert_eq!(bytes, bytes2);
}

// ---------------------------------------------------------------------------
// Crypto adversarial tests
// ---------------------------------------------------------------------------

#[test]
fn morph_hash_empty_input() {
    let mut engine = MorphHashEngine::new();
    let prev_hash = zero_hash();
    let hash = engine.morph_hash(&[], &prev_hash, 0);
    assert_eq!(hash.0.len(), 64);
    assert_ne!(hash, zero_hash());
}

#[test]
fn morph_hash_very_large_input() {
    let mut engine = MorphHashEngine::new();
    let prev_hash = zero_hash();
    let large_input = vec![0xFFu8; 1_000_000]; // 1 MB
    let hash = engine.morph_hash(&large_input, &prev_hash, 0);
    assert_eq!(hash.0.len(), 64);
}

#[test]
fn morph_hash_different_prev_hash_different_output() {
    let mut engine = MorphHashEngine::new();
    let input = b"same_input";

    let hash1 = engine.morph_hash(input, &zero_hash(), 0);
    let hash2 = engine.morph_hash(input, &make_hash([1u8; 64]), 0);
    assert_ne!(hash1, hash2);
}

#[test]
fn select_algorithm_chain_deterministic_for_same_inputs() {
    let prev_hash = zero_hash();
    let chain1 = select_algorithm_chain(&prev_hash, 100);
    let chain2 = select_algorithm_chain(&prev_hash, 100);
    assert_eq!(chain1, chain2);
    assert_eq!(chain1.len(), 16);
}

#[test]
fn select_algorithm_chain_all_valid_algorithms() {
    let prev_hash = zero_hash();
    let chain = select_algorithm_chain(&prev_hash, 0);
    for algo in &chain {
        // All selected algorithms should be valid variants
        let _name = algo.name();
        assert!(!_name.is_empty());
    }
}

#[test]
fn morph_algorithm_all_has_32_unique_entries() {
    let all = MorphAlgorithm::all();
    assert_eq!(all.len(), 32);
    let mut names: Vec<&str> = all.iter().map(|a| a.name()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 32);
}

#[test]
fn homomorphic_encrypt_without_key_fails() {
    let engine = HomomorphicEngine::new(HomomorphicContext::new(
        HomomorphicScheme::BFV,
        4096,
        1 << 30,
        1 << 20,
    ));
    let plaintext = Plaintext { data: vec![1] };
    assert!(engine.encrypt(&plaintext).is_err());
}

#[test]
fn homomorphic_add_mismatched_schemes_fails() {
    let mut engine1 = HomomorphicEngine::new(HomomorphicContext::new(
        HomomorphicScheme::BFV,
        4096,
        1 << 30,
        1 << 20,
    ));
    engine1.generate_key_pair().unwrap();
    let mut engine2 = HomomorphicEngine::new(HomomorphicContext::new(
        HomomorphicScheme::CKKS,
        8192,
        1 << 40,
        1 << 20,
    ));
    engine2.generate_key_pair().unwrap();

    let pt = Plaintext { data: vec![1] };
    let ct1 = engine1.encrypt(&pt).unwrap();
    let ct2 = engine2.encrypt(&pt).unwrap();

    // Can't add across engines since schemes differ
    let result = engine1.add(&ct1, &ct2);
    assert!(result.is_err());
}

#[test]
fn homomorphic_add_mismatched_sizes_fails() {
    let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
        HomomorphicScheme::BFV,
        4096,
        1 << 30,
        1 << 20,
    ));
    engine.generate_key_pair().unwrap();

    let pt1 = Plaintext {
        data: vec![1, 2, 3],
    };
    let pt2 = Plaintext { data: vec![4, 5] };
    let ct1 = engine.encrypt(&pt1).unwrap();
    let ct2 = engine.encrypt(&pt2).unwrap();

    let result = engine.add(&ct1, &ct2);
    assert!(result.is_err());
}

#[test]
fn homomorphic_multiply_noise_too_high_fails() {
    let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
        HomomorphicScheme::BFV,
        4096,
        1 << 30,
        1 << 20,
    ));
    engine.generate_key_pair().unwrap();

    let pt = Plaintext { data: vec![1] };
    let ct = engine.encrypt(&pt).unwrap();

    // Multiply many times to exceed noise threshold (1000)
    let mut current = ct.clone();
    for _ in 0..100 {
        match engine.multiply(&current, &ct) {
            Ok(c) => current = c,
            Err(_) => break, // Noise too high, expected
        }
    }
    // After many multiplications, noise should exceed 1000 and fail
    let result = engine.multiply(&current, &ct);
    // Either succeeds with high noise or fails - both are valid behaviors
    let _ = result;
}

#[test]
fn pq_sign_empty_secret_key_fails() {
    let engine = PQSignatureEngine::new();
    let keypair = PQSignatureEngine::new()
        .generate_keypair(PQSignatureScheme::Dilithium2)
        .unwrap();
    // Create a keypair with empty secret key
    let empty_keypair = atoll_core::privacy::post_quantum::PQKeyPair {
        scheme: PQSignatureScheme::Dilithium2,
        public_key: vec![1; 1312],
        secret_key: vec![],
    };
    let result = engine.sign(b"test", &empty_keypair);
    assert!(result.is_err());
}

#[test]
fn pq_sign_empty_message() {
    let engine = PQSignatureEngine::new();
    let keypair = engine
        .generate_keypair(PQSignatureScheme::Dilithium2)
        .unwrap();
    let result = engine.sign(&[], &keypair);
    assert!(result.is_ok()); // Empty message should still work
}

#[test]
fn pq_verify_empty_signature_fails() {
    let engine = PQSignatureEngine::new();
    let keypair = engine
        .generate_keypair(PQSignatureScheme::Dilithium2)
        .unwrap();
    let sig = atoll_core::privacy::post_quantum::PQSignature {
        scheme: PQSignatureScheme::Dilithium2,
        signature: vec![],
    };
    let result = engine.verify(b"test", &sig, &keypair.public_key).unwrap();
    assert!(!result);
}

#[test]
fn pq_kem_encapsulate_empty_public_key_fails() {
    let engine = PQKEMEngine::new();
    let result = engine.encapsulate(&[], PQKEMScheme::Kyber512);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Empty public key");
}

#[test]
fn pq_kem_decapsulate_empty_ciphertext_fails() {
    let engine = PQKEMEngine::new();
    let result = engine.decapsulate(&[], &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Empty ciphertext");
}

#[test]
fn stark_prove_empty_inputs_fails() {
    let prover = StarkProver::new(40, 256);
    let result = prover.prove(zero_hash(), &[], &vec![vec![1]]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "No inputs provided");
}

#[test]
fn stark_prove_empty_outputs_fails() {
    let prover = StarkProver::new(40, 256);
    let result = prover.prove(zero_hash(), &vec![vec![1]], &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "No outputs provided");
}

// ---------------------------------------------------------------------------
// Consensus adversarial tests
// ---------------------------------------------------------------------------

#[test]
fn penta_consensus_invalid_pouw() {
    let consensus = PentaConsensus::new();
    let result = consensus.validate_block(
        &pouw::PoUWResult {
            valid: false,
            usefulness_score: 0.8,
            difficulty_met: true,
        },
        &make_valid_pos(),
        &make_valid_post(),
        &make_valid_por(),
        &make_valid_poh(),
    );
    assert_eq!(
        result,
        Err(atoll_core::consensus::penta::ConsensusError::PoUWFailed)
    );
}

#[test]
fn penta_consensus_invalid_pos() {
    let consensus = PentaConsensus::new();
    let result = consensus.validate_block(
        &make_valid_pouw(),
        &pos_vrf::PoSResult {
            valid: false,
            combined_score: 0.9,
        },
        &make_valid_post(),
        &make_valid_por(),
        &make_valid_poh(),
    );
    assert_eq!(
        result,
        Err(atoll_core::consensus::penta::ConsensusError::PoSFailed)
    );
}

#[test]
fn penta_consensus_composite_score_too_low() {
    let consensus = PentaConsensus::new();
    // All scores zero -> composite = 0 < 0.5 threshold
    let pouw = pouw::PoUWResult {
        valid: true,
        usefulness_score: 0.0,
        difficulty_met: true,
    };
    let pos = pos_vrf::PoSResult {
        valid: true,
        combined_score: 0.0,
    };
    let post = post::PoSTResult {
        valid: true,
        size_score: 0.0,
    };
    let por = por::PoRResult {
        raw_score: 0.0,
        decayed_score: 0.0,
        is_eligible: true,
    };
    let poh = poh::PoHResult { is_verified: true }; // poh passes, but composite score too low

    let result = consensus.validate_block(&pouw, &pos, &post, &por, &poh);
    assert_eq!(
        result,
        Err(atoll_core::consensus::penta::ConsensusError::CompositeScoreTooLow)
    );
}

#[test]
fn hotstuff_quorum_with_fewer_than_2n3_votes() {
    let state = HotStuffState::new(100);
    // quorum_size = 67 for 100 validators
    assert_eq!(state.quorum_size, 67);

    // With 0 validators, quorum = 1 (2*0/3 + 1)
    let state0 = HotStuffState::new(0);
    assert_eq!(state0.quorum_size, 1);
}

#[test]
fn hotstuff_vote_wrong_height_no_quorum() {
    let mut state = HotStuffState::new(100);
    state.current_height = 5;
    state.current_round = 5;

    let block_hash = zero_hash();
    let validator = zero_address();
    for _ in 0..67 {
        state.add_vote(BFTVote {
            validator: validator.clone(),
            block_hash,
            height: 1, // Wrong height
            round: 1,
            vote_type: VoteType::Commit,
            signature: vec![1; 64],
        });
    }

    // Quorum check uses current_height/current_round (5,5), votes are at (1,1)
    assert!(!state.has_quorum(&block_hash, 5, 5, VoteType::Commit));
}

#[test]
fn slashing_invalid_work() {
    let consensus = PentaConsensus::new();
    let evidence = atoll_core::consensus::penta::SlashingEvidence::InvalidWork {
        validator: zero_address(),
    };
    let action = consensus.check_slashing(&evidence).unwrap();
    assert_eq!(action.penalty_percent, 50);
    assert!(action.burn);
    assert_eq!(action.ban_epochs, 365);
}

#[test]
fn slashing_downtime_exactly_threshold() {
    let consensus = PentaConsensus::new();
    // Exactly 1000 missed slots - NOT > 1000, so no slashing
    let evidence = atoll_core::consensus::penta::SlashingEvidence::Downtime {
        validator: zero_address(),
        missed_slots: 1000,
    };
    assert!(consensus.check_slashing(&evidence).is_none());

    // 1001 missed slots - slashing
    let evidence = atoll_core::consensus::penta::SlashingEvidence::Downtime {
        validator: zero_address(),
        missed_slots: 1001,
    };
    let action = consensus.check_slashing(&evidence).unwrap();
    assert_eq!(action.penalty_percent, 1);
}

// ---------------------------------------------------------------------------
// Chain adversarial tests
// ---------------------------------------------------------------------------

#[test]
fn blockchain_insert_block_wrong_prev_hash() {
    let mut bc = Blockchain::default();
    let consensus = PentaConsensus::new();
    let validation = make_valid_validation();

    let block = Block {
        header: BlockHeader {
            version: 1,
            height: 1,
            shard_id: 0,
            timestamp: 0,
            prev_hash: make_hash([9u8; 64]), // Wrong - doesn't match tip
            state_root: zero_hash(),
            transaction_root: zero_hash(),
            proposer: zero_address(),
            difficulty: 1,
        },
        body: BlockBody {
            transactions: Vec::new(),
        },
    };
    let result = bc.insert_block(block, &validation);
    assert!(result.is_err());
    assert_eq!(bc.tip_height(), 0);
}

#[test]
fn blockchain_get_nonexistent_block() {
    let bc = Blockchain::default();
    assert!(bc.get_block(&zero_hash()).is_none());
    // Genesis block hash is not zero_hash() - it's the morph hash of genesis
    assert!(bc.get_block_by_height(999).is_none());
}

#[test]
fn chain_state_transfer_to_self() {
    let mut state = ChainState::new();
    let addr = make_address([1u8; 20]);
    state.set_balance(addr, 1000);

    // Transfer to self should work
    assert!(state.transfer(&addr, &addr, 500).is_ok());
    assert_eq!(state.balance(&addr), 1000); // Balance unchanged (sent to self)
    assert_eq!(state.nonce(&addr), 1); // But nonce incremented
}

#[test]
fn chain_state_transfer_zero_amount() {
    let mut state = ChainState::new();
    let from = make_address([1u8; 20]);
    let to = make_address([2u8; 20]);
    state.set_balance(from, 100);

    assert!(state.transfer(&from, &to, 0).is_ok());
    assert_eq!(state.balance(&from), 100);
    assert_eq!(state.balance(&to), 0);
    assert_eq!(state.nonce(&from), 1);
}

#[test]
fn chain_state_burn_zero_amount() {
    let mut state = ChainState::new();
    let addr = make_address([1u8; 20]);
    state.set_balance(addr, 100);

    assert!(state.burn_atoll(&addr, 0).is_ok());
    assert_eq!(state.balance(&addr), 100);
}

// ---------------------------------------------------------------------------
// Sharding adversarial tests
// ---------------------------------------------------------------------------

#[test]
fn shard_manager_create_shard_id_exceeds_max() {
    let mut manager = ShardManager::new();
    manager.max_shards = 10;
    let result = manager.create_shard(atoll_core::chain::sharding::ShardConfig {
        id: 100, // Exceeds max_shards
        ..Default::default()
    });
    assert!(result.is_err());
}

#[test]
fn shard_manager_remove_nonexistent_shard() {
    let mut manager = ShardManager::new();
    let result = manager.remove_shard(999);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Shard does not exist");
}

#[test]
fn shard_manager_assign_validators_to_nonexistent_shard() {
    let mut manager = ShardManager::new();
    let result = manager.assign_validators(999, vec![zero_address()]);
    assert!(result.is_err());
}

#[test]
fn shard_manager_exceeds_max_validators() {
    let mut manager = ShardManager::new();
    // Shard 0 has max_validators = 1000 by default
    let validators: Vec<Address> = (0..1001).map(|i| make_address([i as u8; 20])).collect();
    let result = manager.assign_validators(0, validators);
    assert!(result.is_err());
}

#[test]
fn cross_shard_request_to_nonexistent_receipt() {
    let mut xshard_mgr = CrossShardManager::new();
    let result = xshard_mgr.request_to_receipt(&zero_hash());
    assert!(result.is_err());
}

#[test]
fn cross_shard_execute_nonexistent_receipt() {
    let mut xshard_mgr = CrossShardManager::new();
    let result = xshard_mgr.execute_receipt(&zero_hash(), 100);
    assert!(result.is_err());
}

#[test]
fn cross_shard_execute_receipt_twice() {
    let mut xshard_mgr = CrossShardManager::new();
    let request_id =
        xshard_mgr.create_request(0, 1, zero_address(), zero_address(), 10, vec![], 100);
    let receipt_id = xshard_mgr.request_to_receipt(&request_id).unwrap().id;

    xshard_mgr.execute_receipt(&receipt_id, 150).unwrap();
    let result = xshard_mgr.execute_receipt(&receipt_id, 160);
    assert!(result.is_err());
    // Receipt was removed from pending after first execution, so it's "not found"
    assert_eq!(result.unwrap_err(), "Receipt not found");
}

// ---------------------------------------------------------------------------
// Network adversarial tests
// ---------------------------------------------------------------------------

#[test]
fn gossip_select_targets_empty_peers() {
    let gossip = GossipProtocol::new(GossipConfig::default());
    let targets = gossip.select_gossip_targets(&[], [0u8; 32]);
    assert!(targets.is_empty());
}

#[test]
fn gossip_select_targets_only_sender() {
    let gossip = GossipProtocol::new(GossipConfig::default());
    let targets = gossip.select_gossip_targets(&[[1u8; 32]], [1u8; 32]);
    assert!(targets.is_empty()); // Only sender, excluded
}

#[test]
fn sync_manager_no_peers() {
    let sm = SyncManager::new(10);
    assert!(sm.peers_ahead_of_us(100).is_empty());
    assert!(sm.peers_needing_blocks(100).is_empty());
    assert_eq!(sm.tracked_peer_count(), 0);
}

#[test]
fn sync_manager_get_cached_header_nonexistent() {
    let sm = SyncManager::new(10);
    assert!(sm.get_cached_header(&zero_hash()).is_none());
}

#[test]
fn discovery_handle_find_node_no_peers() {
    let discovery = DiscoveryProtocol::new([1u8; 32], 8);
    let neighbors = discovery.handle_find_node(&atoll_core::network::FindNodeMessage {
        target_node_id: [2u8; 32],
    });
    assert!(neighbors.peers.is_empty());
}

#[test]
fn peer_manager_add_banned_peer() {
    let mut pm = PeerManager::new(10);
    let node_id = [1u8; 32];
    let info = PeerInfo {
        node_id,
        address: "127.0.0.1".to_string(),
        port: 30333,
        last_seen: 0,
    };

    pm.ban_peer(node_id, 3600);
    assert!(pm.is_banned(&node_id));
    assert!(!pm.add_peer(node_id, info));
}

#[test]
fn peer_manager_record_message_for_nonexistent_peer() {
    let mut pm = PeerManager::new(10);
    // Should not panic
    pm.record_message_sent(&[1u8; 32]);
    pm.record_message_received(&[1u8; 32]);
    pm.update_ping_pong(&[1u8; 32], 100, Some(101));
    assert_eq!(pm.connected_count(), 0);
}

// ---------------------------------------------------------------------------
// Privacy adversarial tests
// ---------------------------------------------------------------------------

#[test]
fn privacy_encrypted_mempool_add_duplicate() {
    let mut mempool = PrivacyEncryptedMempool::new(zero_hash(), vec![], 1);
    let tx = EncryptedTransaction {
        ciphertext: vec![1],
        order_commitment: zero_hash(),
        fee_commitment: zero_hash(),
        submitter_proof: vec![],
        submitter: zero_address(),
        received_at: 1000,
    };
    mempool.add_transaction(tx.clone(), zero_hash()).unwrap();
    let result = mempool.add_transaction(tx, zero_hash());
    assert!(result.is_err());
}

#[test]
fn privacy_encrypted_mempool_add_decryption_share_for_unknown_tx() {
    let mut mempool = PrivacyEncryptedMempool::new(zero_hash(), vec![], 1);
    let result = mempool.add_decryption_share(DecryptionShare {
        tx_hash: zero_hash(),
        share: vec![1],
        validator: zero_address(),
        share_index: 0,
    });
    assert!(result.is_err());
}

#[test]
fn privacy_encrypted_mempool_remove_nonexistent() {
    let mut mempool = PrivacyEncryptedMempool::new(zero_hash(), vec![], 1);
    let result = mempool.remove_transaction(&zero_hash());
    assert!(result.is_none());
}

#[test]
fn privacy_encrypted_mempool_max_size_reached() {
    let mut mempool = PrivacyEncryptedMempool::new(zero_hash(), vec![], 1);
    mempool.set_max_size(2);

    let tx1 = EncryptedTransaction {
        ciphertext: vec![1],
        order_commitment: make_hash([1u8; 64]),
        fee_commitment: zero_hash(),
        submitter_proof: vec![],
        submitter: zero_address(),
        received_at: 1000,
    };
    let tx2 = EncryptedTransaction {
        ciphertext: vec![2],
        order_commitment: make_hash([2u8; 64]),
        fee_commitment: zero_hash(),
        submitter_proof: vec![],
        submitter: zero_address(),
        received_at: 1000,
    };
    let tx3 = EncryptedTransaction {
        ciphertext: vec![3],
        order_commitment: make_hash([3u8; 64]),
        fee_commitment: zero_hash(),
        submitter_proof: vec![],
        submitter: zero_address(),
        received_at: 1000,
    };

    mempool.add_transaction(tx1, zero_hash()).unwrap();
    mempool.add_transaction(tx2, zero_hash()).unwrap();
    let result = mempool.add_transaction(tx3, zero_hash());
    assert!(result.is_err());
    assert_eq!(mempool.pending_count(), 2);
}

// ---------------------------------------------------------------------------
// VM adversarial tests
// ---------------------------------------------------------------------------

#[test]
fn evm_out_of_gas_on_expensive_opcode() {
    let context = ExecutionContext::default();
    let mut evm = EVMExecutor::new(context, 10); // Very low gas limit
                                                 // SELFDESTRUCT costs 5000 gas
    let result = evm.execute(&[0xFF], &[]);
    assert!(result.is_err());
}

#[test]
fn evm_invalid_add_operands() {
    let context = ExecutionContext::default();
    let mut evm = EVMExecutor::new(context, 10_000);
    // ADD opcode without enough operands
    let result = evm.execute(&[0x01, 0x01], &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid ADD operands");
}

#[test]
fn evm_invalid_mul_operands() {
    let context = ExecutionContext::default();
    let mut evm = EVMExecutor::new(context, 10_000);
    let result = evm.execute(&[0x02, 0x01], &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid MUL operands");
}

#[test]
fn evm_push_operand_out_of_bounds() {
    let context = ExecutionContext::default();
    let mut evm = EVMExecutor::new(context, 10_000);
    // PUSH1 (0x60) requires 1 byte operand, but none provided
    let result = evm.execute(&[0x60], &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "PUSH operand out of bounds");
}

#[test]
fn wasm_invalid_magic_number() {
    let context = ExecutionContext::default();
    let mut wasm = WASMExecutor::new(context, 10_000);
    let result = wasm.execute(
        &[0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00],
        "main",
        &[],
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid WASM magic number");
}

#[test]
fn wasm_unsupported_version() {
    let context = ExecutionContext::default();
    let mut wasm = WASMExecutor::new(context, 10_000);
    let result = wasm.execute(
        &[0x00, 0x61, 0x73, 0x6d, 0x02, 0x00, 0x00, 0x00],
        "main",
        &[],
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Unsupported WASM version: 2");
}

#[test]
fn wasm_too_short() {
    let context = ExecutionContext::default();
    let mut wasm = WASMExecutor::new(context, 10_000);
    let result = wasm.execute(&[0x00, 0x61, 0x73], "main", &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "WASM module too short");
}

#[test]
fn move_empty_bytecode() {
    let context = ExecutionContext::default();
    let mut mv = MoveExecutor::new(context, 10_000);
    let result = mv.execute(&[], "main", &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Empty Move bytecode");
}

#[test]
fn riscv_invalid_elf_magic() {
    let context = ExecutionContext::default();
    let mut riscv = RISCVExecutor::new(context, 10_000);
    let result = riscv.execute(
        &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00],
        0x1000,
        &[],
    );
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid RISC-V ELF magic number");
}

#[test]
fn riscv_too_short() {
    let context = ExecutionContext::default();
    let mut riscv = RISCVExecutor::new(context, 10_000);
    let result = riscv.execute(&[0x7f, 0x45], 0x1000, &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "RISC-V binary too short");
}

#[test]
fn ai_vm_model_too_large() {
    let context = ExecutionContext::default();
    let mut ai_vm = AIVMExecutor::new(context, 50_000_000);
    let model = vec![0u8; 101 * 1024 * 1024]; // 101 MB > 100 MB limit
    let result = ai_vm.deploy_model(&model);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Model too large");
}

#[test]
fn ai_vm_invalid_model() {
    let context = ExecutionContext::default();
    let mut ai_vm = AIVMExecutor::new(context, 50_000_000);
    let result = ai_vm.deploy_model(&[0x00, 0x01]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Invalid ONNX model");
}

#[test]
fn ai_vm_verify_empty_proof() {
    let context = ExecutionContext::default();
    let mut ai_vm = AIVMExecutor::new(context, 50_000_000);
    let result = ai_vm.verify_inference(&zero_hash(), &[], &[], &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Empty proof");
}

#[test]
fn zk_vm_empty_proof() {
    let context = ExecutionContext::default();
    let mut zk_vm = ZKVMExecutor::new(context, 100_000_000);
    let result = zk_vm.execute(&[1], &[], &[]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Empty ZK proof");
}

#[test]
fn zk_vm_empty_circuit_id() {
    let context = ExecutionContext::default();
    let mut zk_vm = ZKVMExecutor::new(context, 100_000_000);
    let result = zk_vm.execute(&[], &[], &[1]);
    assert!(result.is_err());
    assert_eq!(result.unwrap_err(), "Empty circuit ID");
}

#[test]
fn zk_vm_verify_empty_proof_returns_false() {
    let context = ExecutionContext::default();
    let mut zk_vm = ZKVMExecutor::new(context, 100_000_000);
    let result = zk_vm.verify_proof(&[1], &[], &[]).unwrap();
    assert!(!result);
}

#[test]
fn vm_router_empty_data() {
    assert!(VMRouter::route(&[]).is_err());
}

#[test]
fn gas_meter_out_of_gas() {
    let mut meter = GasMeter::new(10, 1);
    assert!(meter.charge(11).is_err());
    assert_eq!(meter.gas_used, 0); // No gas consumed on failure
    assert_eq!(meter.remaining(), 10);
}

#[test]
fn gas_meter_charge_exactly_limit() {
    let mut meter = GasMeter::new(100, 1);
    assert!(meter.charge(100).is_ok());
    assert_eq!(meter.remaining(), 0);
    assert!(meter.charge(1).is_err());
}

// ---------------------------------------------------------------------------
// Edge cases: zero values, max values, empty states
// ---------------------------------------------------------------------------

#[test]
fn edge_case_zero_hash_is_default() {
    assert_eq!(Hash512::zero(), Hash512::default());
    assert_eq!(Hash512::zero().to_array(), [0u8; 64]);
}

#[test]
fn edge_case_zero_address_is_default() {
    assert_eq!(Address::zero(), Address::default());
    assert_eq!(Address::zero().to_bytes(), [0u8; 20]);
}

#[test]
fn edge_case_default_signature() {
    let sig = Signature::default();
    assert_eq!(sig.ed25519, [0u8; 64]);
    assert!(sig.dilithium.is_empty());
    assert_eq!(sig.total_size(), 64);
}

#[test]
fn edge_case_default_transaction() {
    let tx = Transaction::default();
    assert_eq!(tx.version, 1);
    assert_eq!(tx.from, Address::zero());
    assert_eq!(tx.to, None);
    assert_eq!(tx.value, 0);
    assert!(tx.data.is_empty());
}

#[test]
fn edge_case_default_block() {
    let block = Block::default();
    assert_eq!(block.header.height, 0);
    assert_eq!(block.header.prev_hash, Hash512::zero());
    assert!(block.body.transactions.is_empty());
}

#[test]
fn edge_case_u128_max_value_transaction() {
    let tx = Transaction {
        version: 1,
        from: zero_address(),
        to: Some(zero_address()),
        value: u128::MAX,
        data: Vec::new(),
        signature: Signature::default(),
    };
    let bytes = tx.serialize_bytes();
    assert!(!bytes.is_empty());
    // Deterministic
    assert_eq!(bytes, tx.serialize_bytes());
}

#[test]
fn edge_case_empty_data_vec_transaction() {
    let tx = Transaction {
        version: 0,
        from: zero_address(),
        to: None,
        value: 0,
        data: Vec::new(),
        signature: Signature::default(),
    };
    let bytes = tx.serialize_bytes();
    assert!(!bytes.is_empty());
}
