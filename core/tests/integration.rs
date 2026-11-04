//! Integration tests for Phase 6 Testing & Auditing.
//!
//! These tests verify that multiple modules work together correctly,
//! simulating real-world scenarios that span primitives, crypto, consensus,
//! chain, network, privacy, VM, storage, mempool, and API layers.

use atoll_core::chain::blockchain::Blockchain;
use atoll_core::chain::sharding::cross_shard::CrossShardManager;
use atoll_core::chain::sharding::rebalancer::ShardRebalancer;
use atoll_core::chain::sharding::shard_manager::ShardManager;
use atoll_core::chain::state::ChainState;
use atoll_core::config::chain_params::ChainParams;
use atoll_core::config::genesis::GenesisConfig;
use atoll_core::config::network_params::NetworkParams;
use atoll_core::consensus::finality::hotstuff::{HotStuffBFT, HotStuffState, VoteType};
use atoll_core::consensus::fork_choice::ForkChoice;
use atoll_core::consensus::penta::{
    poh, por, pos_vrf, post, pouw, PentaConsensus, PentaValidation,
};
use atoll_core::crypto::morph_hash::{select_algorithm_chain, MorphHashEngine};
use atoll_core::network::discovery::DiscoveryProtocol;
use atoll_core::network::gossip::{GossipConfig, GossipProtocol};
use atoll_core::network::p2p::PeerConnection;
use atoll_core::network::sync::SyncManager;
use atoll_core::network::{NetworkMessage, PeerInfo, PingMessage};
use atoll_core::primitives::address::Address;
use atoll_core::primitives::block::{Block, BlockBody, BlockHeader};
use atoll_core::primitives::hash::Hash512;
use atoll_core::privacy::encrypted_mempool::{
    DecryptionShare, EncryptedMempool as PrivacyEncryptedMempool, EncryptedTransaction,
};
use atoll_core::privacy::homomorphic::{
    HomomorphicContext, HomomorphicEngine, HomomorphicScheme, Plaintext,
};
use atoll_core::privacy::post_quantum::{
    HybridKeyPair, HybridSignatureEngine, PQKEMEngine, PQKEMScheme, PQSignatureEngine,
    PQSignatureScheme,
};
use atoll_core::privacy::zk_starks::{StarkProof, StarkProver, StarkVerifier};
use atoll_core::vm::ai_vm::AIVMExecutor;
use atoll_core::vm::evm::EVMExecutor;
use atoll_core::vm::move_vm::MoveExecutor;
use atoll_core::vm::riscv::RISCVExecutor;
use atoll_core::vm::wasm::WASMExecutor;
use atoll_core::vm::zk_vm::ZKVMExecutor;
use atoll_core::vm::{ExecutionContext, ExecutionResult, GasMeter, VMRouter, VMType};

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

fn make_address(bytes: [u8; 20]) -> Address {
    Address::from_bytes(bytes)
}

fn make_hash(bytes: [u8; 64]) -> Hash512 {
    Hash512::from_bytes(&bytes)
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

fn make_block_with_prev(prev_hash: Hash512, height: u64, proposer: Address) -> Block {
    Block {
        header: BlockHeader {
            version: 1,
            height,
            shard_id: 0,
            timestamp: atoll_core::GENESIS_TIMESTAMP + height,
            prev_hash,
            state_root: atoll_core::primitives::hash::Hash512::zero(),
            transaction_root: atoll_core::primitives::hash::Hash512::zero(),
            proposer,
            difficulty: 1,
        },
        body: BlockBody {
            transactions: Vec::new(),
        },
    }
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
// Primitives + Crypto integration
// ---------------------------------------------------------------------------

#[test]
fn primitives_hash_and_block_integration() {
    let block = atoll_core::primitives::block::Block::default();
    let bytes = block.serialize_bytes();
    let hash1 = atoll_core::primitives::hash::Hash512::blake3(&bytes);
    let hash2 = atoll_core::primitives::hash::Hash512::blake3(&bytes);
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.to_array().len(), 64);

    let addr = make_address([42u8; 20]);
    let hex = addr.to_hex();
    let restored = Address::from_hex(&hex).unwrap();
    assert_eq!(addr, restored);
}

#[test]
fn morph_hash_with_block_header() {
    let mut engine = MorphHashEngine::new();
    let prev_hash = Hash512::zero();
    let block = make_block_with_prev(prev_hash, 1, make_address([1u8; 20]));
    let block_hash = engine.morph_hash(&block.serialize_bytes(), &prev_hash, 1);
    let block_hash2 = engine.morph_hash(&block.serialize_bytes(), &prev_hash, 1);
    assert_eq!(block_hash, block_hash2);
    let block_hash3 = engine.morph_hash(&block.serialize_bytes(), &prev_hash, 2);
    assert_ne!(block_hash, block_hash3);
}

// ---------------------------------------------------------------------------
// Consensus + Chain integration
// ---------------------------------------------------------------------------

#[test]
fn penta_consensus_validates_block_for_chain_insertion() {
    let consensus = PentaConsensus::new();
    let validation = consensus.validate_block(
        &make_valid_pouw(),
        &make_valid_pos(),
        &make_valid_post(),
        &make_valid_por(),
        &make_valid_poh(),
    );
    assert!(validation.is_ok());
    assert!(validation.unwrap().is_valid());
}

#[test]
fn blockchain_accepts_valid_block_with_penta_validation() {
    let mut bc = Blockchain::default();
    let consensus = PentaConsensus::new();
    let validation = consensus
        .validate_block(
            &make_valid_pouw(),
            &make_valid_pos(),
            &make_valid_post(),
            &make_valid_por(),
            &make_valid_poh(),
        )
        .unwrap();

    let block = make_block_with_prev(bc.tip_hash(), 1, make_address([1u8; 20]));
    let block_hash = bc.insert_block(block, &validation).unwrap();
    assert_eq!(bc.tip_height(), 1);
    assert_eq!(bc.tip_hash(), block_hash);
    assert_eq!(bc.block_count(), 2);
}

#[test]
fn blockchain_rejects_invalid_validation() {
    let mut bc = Blockchain::default();
    let consensus = PentaConsensus::new();
    let invalid_validation = PentaValidation {
        pouw_valid: false,
        pouw_score: 0.0,
        pos_valid: false,
        pos_score: 0.0,
        post_valid: false,
        post_score: 0.0,
        por_valid: false,
        por_score: 0.0,
        poh_valid: false,
        poh_score: 0.0,
        composite_score: 0.0,
        finality_achieved: false,
        all_layers_pass: false,
    };

    let block = make_block_with_prev(bc.tip_hash(), 1, make_address([1u8; 20]));
    let result = bc.insert_block(block, &invalid_validation);
    assert!(result.is_err());
    assert_eq!(bc.tip_height(), 0);
}

#[test]
fn hotstuff_finality_with_penta_consensus() {
    let consensus = PentaConsensus::new();
    let mut hotstuff_state = HotStuffState::new(100);
    hotstuff_state.current_height = 1;
    hotstuff_state.current_round = 1;

    let block_hash = Hash512::zero();
    let validator = make_address([1u8; 20]);
    for _ in 0..67 {
        hotstuff_state.add_vote(atoll_core::consensus::finality::hotstuff::BFTVote {
            validator: validator.clone(),
            block_hash,
            height: 1,
            round: 1,
            vote_type: VoteType::Commit,
            signature: vec![1; 64],
        });
    }

    let validation = consensus.validate_block_with_finality(
        &make_valid_pouw(),
        &make_valid_pos(),
        &make_valid_post(),
        &make_valid_por(),
        &make_valid_poh(),
        &hotstuff_state,
        &block_hash,
    );
    assert!(validation.is_ok());
    assert!(validation.unwrap().finality_achieved);
}

#[test]
fn fork_choice_selects_heaviest_chain() {
    let mut dag = ForkChoice::new();
    let genesis = make_hash([0u8; 64]);
    dag.insert_block(genesis, 0);

    let b1 = make_hash([1u8; 64]);
    dag.insert_block(b1, 1);
    let b2 = make_hash([2u8; 64]);
    dag.insert_block(b2, 2);
    let b3 = make_hash([3u8; 64]);
    dag.insert_block(b3, 3);

    let f1 = make_hash([4u8; 64]);
    dag.insert_block(f1, 1);
    let f2 = make_hash([5u8; 64]);
    dag.insert_block(f2, 2);

    assert_eq!(dag.tip(), Some(&b3));
    assert_eq!(dag.block_count(), 6);
}

// ---------------------------------------------------------------------------
// Chain + Sharding integration
// ---------------------------------------------------------------------------

#[test]
fn shard_manager_with_cross_shard_communication() {
    let mut shard_mgr = ShardManager::new();
    let mut xshard_mgr = CrossShardManager::new();

    shard_mgr
        .create_shard(atoll_core::chain::sharding::ShardConfig {
            id: 1,
            ..Default::default()
        })
        .unwrap();

    let request_id = xshard_mgr.create_request(
        0,
        1,
        make_address([1u8; 20]),
        make_address([2u8; 20]),
        100_000_000,
        vec![1, 2, 3],
        1000,
    );

    assert_eq!(xshard_mgr.total_cross_shard_tx(), 1);

    let receipt_id = xshard_mgr.request_to_receipt(&request_id).unwrap().id;
    xshard_mgr.execute_receipt(&receipt_id, 1050).unwrap();
    assert!(xshard_mgr.is_receipt_executed(&receipt_id));
}

#[test]
fn shard_rebalancer_split_and_merge() {
    let mut rebalancer = ShardRebalancer::new();
    rebalancer.set_rebalance_interval(100);

    let validators: Vec<atoll_core::primitives::address::Address> =
        (0..200).map(|i| make_address([i as u8; 20])).collect();
    rebalancer
        .shard_manager_mut()
        .assign_validators(0, validators)
        .unwrap();

    rebalancer
        .shard_manager_mut()
        .update_shard_state(0, 100, Hash512::zero(), 2.0, 1000, 100, 100)
        .unwrap();

    let result = rebalancer.update_shard_load(0, 200, 2.0, 1000, 100, 100, Hash512::zero());
    assert!(result.is_ok());
    assert_eq!(rebalancer.last_rebalance_block, 200);
    assert!(rebalancer.shard_manager().shard_count() >= 1);
}

// ---------------------------------------------------------------------------
// Network + Consensus integration
// ---------------------------------------------------------------------------

#[test]
fn gossip_propagates_consensus_votes() {
    let mut gossip = GossipProtocol::new(GossipConfig::default());
    let peers = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];

    let vote = atoll_core::network::BFTVote {
        validator: make_address([1u8; 20]),
        block_hash: make_hash([1u8; 64]),
        height: 1,
        round: 1,
        vote_type: 2,
        signature: vec![1; 64],
    };
    let message = NetworkMessage::BFTCommit(vote.clone());
    let sender = [9u8; 32];

    gossip.broadcast(&message, sender);
    assert_eq!(gossip.history_size(), 1);

    let targets = gossip.select_gossip_targets(&peers, sender);
    assert!(!targets.contains(&sender));
    assert!(targets.len() <= 4);
}

#[test]
fn sync_manager_tracks_peer_heights() {
    let mut sm = SyncManager::new(10);
    let node_id = [1u8; 32];

    sm.update_peer_height(node_id, 100);
    assert_eq!(sm.tracked_peer_count(), 1);

    let ahead = sm.peers_ahead_of_us(50);
    assert!(ahead.contains(&node_id));

    let behind = sm.peers_needing_blocks(200);
    assert!(behind.contains(&node_id));
}

#[test]
fn discovery_finds_closest_peers() {
    let mut discovery = DiscoveryProtocol::new([1u8; 32], 2);
    for i in 0..5 {
        let info = PeerInfo {
            node_id: [i as u8; 32],
            address: format!("127.0.0.1:{}", 30000 + i),
            port: 30000 + i as u16,
            last_seen: 0,
        };
        discovery.register_peer(info);
    }

    let neighbors = discovery.handle_find_node(&atoll_core::network::FindNodeMessage {
        target_node_id: [0u8; 32],
    });
    assert!(neighbors.peers.len() <= 2);
}

#[test]
fn encrypted_mempool_threshold_decryption() {
    let mut mempool = PrivacyEncryptedMempool::new(
        Hash512::zero(),
        vec![
            make_address([1u8; 20]),
            make_address([2u8; 20]),
            make_address([3u8; 20]),
        ],
        2,
    );

    let tx = EncryptedTransaction {
        ciphertext: vec![1, 2, 3],
        order_commitment: make_hash([1u8; 64]),
        fee_commitment: Hash512::zero(),
        submitter_proof: vec![],
        submitter: make_address([5u8; 20]),
        received_at: 1000,
    };
    mempool.add_transaction(tx, Hash512::zero()).unwrap();
    assert_eq!(mempool.pending_count(), 1);

    let tx_hash = make_hash([1u8; 64]);
    mempool
        .add_decryption_share(DecryptionShare {
            tx_hash,
            share: vec![1],
            validator: make_address([1u8; 20]),
            share_index: 0,
        })
        .unwrap();
    assert!(!mempool.can_decrypt(&tx_hash));

    mempool
        .add_decryption_share(DecryptionShare {
            tx_hash,
            share: vec![2],
            validator: make_address([2u8; 20]),
            share_index: 1,
        })
        .unwrap();
    assert!(mempool.can_decrypt(&tx_hash));
    assert_eq!(mempool.get_decryptable_transactions().len(), 1);
}

// ---------------------------------------------------------------------------
// Privacy + Crypto integration
// ---------------------------------------------------------------------------

#[test]
fn homomorphic_encrypt_decrypt_roundtrip() {
    let mut engine = HomomorphicEngine::new(HomomorphicContext::new(
        HomomorphicScheme::BFV,
        4096,
        1 << 30,
        1 << 20,
    ));
    engine.generate_key_pair().unwrap();

    let plaintext = Plaintext {
        data: vec![10, 20, 30],
    };
    let ciphertext = engine.encrypt(&plaintext).unwrap();
    let decrypted = engine.decrypt(&ciphertext).unwrap();
    assert_eq!(decrypted.data, plaintext.data);
}

#[test]
fn homomorphic_addition_preserves_values() {
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
    let pt2 = Plaintext {
        data: vec![4, 5, 6],
    };
    let ct1 = engine.encrypt(&pt1).unwrap();
    let ct2 = engine.encrypt(&pt2).unwrap();
    let ct_sum = engine.add(&ct1, &ct2).unwrap();
    let decrypted = engine.decrypt(&ct_sum).unwrap();
    assert_eq!(decrypted.data, vec![5, 7, 9]);
}

#[test]
fn stark_prove_and_verify_integration() {
    let prover = StarkProver::new(40, 256);
    let verifier = StarkVerifier::new();

    let computation_hash = Hash512::blake3(b"integration_computation");
    let inputs = vec![vec![1, 2, 3], vec![4, 5, 6]];
    let outputs = vec![vec![7, 8, 9]];

    let proof: StarkProof = prover.prove(computation_hash, &inputs, &outputs).unwrap();
    assert_eq!(proof.queries.len(), 40);

    let result = verifier.verify(&proof, computation_hash, &inputs).unwrap();
    assert!(result);
}

#[test]
fn pq_signature_kem_integration() {
    let sig_engine = PQSignatureEngine::new();
    let keypair = sig_engine
        .generate_keypair(PQSignatureScheme::Dilithium5)
        .unwrap();
    let message = b"integration test message";

    let signature = sig_engine.sign(message, &keypair).unwrap();
    assert_eq!(signature.scheme, PQSignatureScheme::Dilithium5);
    let valid = sig_engine
        .verify(message, &signature, &keypair.public_key)
        .unwrap();
    assert!(valid);

    let kem_engine = PQKEMEngine::new();
    let kem_keypair = kem_engine.generate_keypair(PQKEMScheme::Kyber768).unwrap();
    let encapsulated = kem_engine
        .encapsulate(&kem_keypair.public_key, PQKEMScheme::Kyber768)
        .unwrap();
    let decapsulated = kem_engine
        .decapsulate(&encapsulated.ciphertext, &kem_keypair.secret_key)
        .unwrap();
    assert_eq!(decapsulated, encapsulated.shared_secret);
}

#[test]
fn hybrid_signature_roundtrip() {
    let engine = HybridSignatureEngine::new();
    let keypair: HybridKeyPair = engine.generate_keypair().unwrap();
    let message = b"hybrid integration test";

    let signature = engine.sign(message, &keypair).unwrap();
    assert_eq!(signature.classical.len(), 64);
    assert!(!signature.pq_dilithium.is_empty());

    let valid = engine
        .verify(
            message,
            &signature,
            &keypair.classical,
            &keypair.pq_dilithium,
        )
        .unwrap();
    assert!(valid);
}

// ---------------------------------------------------------------------------
// VM integration
// ---------------------------------------------------------------------------

#[test]
fn vm_router_routes_all_types() {
    assert_eq!(VMRouter::route(&[0x00]), Ok(VMType::EVM));
    assert_eq!(VMRouter::route(&[0x01]), Ok(VMType::WASM));
    assert_eq!(VMRouter::route(&[0x02]), Ok(VMType::Move));
    assert_eq!(VMRouter::route(&[0x03]), Ok(VMType::RISCV));
    assert_eq!(VMRouter::route(&[0x04]), Ok(VMType::AI));
    assert_eq!(VMRouter::route(&[0x05]), Ok(VMType::ZK));
    assert_eq!(VMRouter::route(&[0xFF]), Ok(VMType::EVM));
    assert!(VMRouter::route(&[]).is_err());
}

#[test]
fn evm_execution_with_gas_metering() {
    let context = ExecutionContext::default();
    let mut evm = EVMExecutor::new(context, 10_000);
    let result = evm.execute(&[0x00], &[]).unwrap();
    match result {
        ExecutionResult::Success { gas_used, .. } => {
            assert!(gas_used > 0);
            assert!(gas_used <= 10_000);
        }
        _ => panic!("Expected Success"),
    }
}

#[test]
fn wasm_execution_with_gas_metering() {
    let context = ExecutionContext::default();
    let mut wasm = WASMExecutor::new(context, 10_000);
    let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let result = wasm.execute(&wasm_bytes, "main", &[]).unwrap();
    assert!(result.gas_used() > 0);
}

#[test]
fn move_execution_with_gas_metering() {
    let context = ExecutionContext::default();
    let mut mv = MoveExecutor::new(context, 10_000);
    let result = mv.execute(&[0x01, 0x02], "main", &[]).unwrap();
    assert!(result.gas_used() > 0);
}

#[test]
fn riscv_execution_with_gas_metering() {
    let context = ExecutionContext::default();
    let mut riscv = RISCVExecutor::new(context, 10_000);
    let binary = vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00];
    let result = riscv.execute(&binary, 0x1000, &[]).unwrap();
    assert!(result.gas_used() > 0);
}

#[test]
fn ai_vm_deploy_and_infer() {
    let context = ExecutionContext::default();
    let mut ai_vm = AIVMExecutor::new(context, 50_000_000);
    let model_bytes = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
    ];
    let deploy_result = ai_vm.deploy_model(&model_bytes).unwrap();
    match deploy_result {
        ExecutionResult::Success { return_data, .. } => {
            assert_eq!(return_data.len(), 64);
        }
        _ => panic!("Expected Success"),
    }

    let model_hash = Hash512::zero();
    let input = vec![1, 2, 3, 4, 5];
    let infer_result = ai_vm.execute_inference(&model_hash, &input).unwrap();
    assert!(infer_result.gas_used() > 0);
}

#[test]
fn zk_vm_execution_with_proof() {
    let context = ExecutionContext::default();
    let mut zk_vm = ZKVMExecutor::new(context, 100_000_000);
    let result = zk_vm.execute(&[1], &[vec![2]], &[3]).unwrap();
    assert!(result.gas_used() > 0);

    let verify_result = zk_vm.verify_proof(&[1], &[], &[1]).unwrap();
    assert!(verify_result);
}

// ---------------------------------------------------------------------------
// Gas metering integration
// ---------------------------------------------------------------------------

#[test]
fn gas_meter_prevents_overrun() {
    let mut meter = GasMeter::new(100, 1);
    assert!(meter.charge(50).is_ok());
    assert!(meter.charge(60).is_err());
    assert_eq!(meter.gas_used, 50);
    assert_eq!(meter.total_cost(), 50);
}

// ---------------------------------------------------------------------------
// Config integration
// ---------------------------------------------------------------------------

#[test]
fn config_values_are_consistent() {
    let genesis = GenesisConfig::default();
    assert_eq!(genesis.chain_id, atoll_core::CHAIN_ID);
    assert_eq!(genesis.timestamp, atoll_core::GENESIS_TIMESTAMP);
    assert_eq!(genesis.difficulty, atoll_core::INITIAL_DIFFICULTY);

    let params = ChainParams::default();
    assert_eq!(params.halving_interval, 210_000);
    assert_eq!(params.initial_block_reward, 50_000_000);

    let net = NetworkParams::default();
    assert_eq!(net.target_block_time, atoll_core::TARGET_BLOCK_TIME);
    assert_eq!(net.max_block_size, 2 * 1024 * 1024);
    assert_eq!(net.max_tx_per_block, 10_000);
}

// ---------------------------------------------------------------------------
// Full pipeline integration
// ---------------------------------------------------------------------------

#[test]
fn full_block_proposal_and_finality_pipeline() {
    let consensus = PentaConsensus::new();

    let validation = consensus.validate_block(
        &make_valid_pouw(),
        &make_valid_pos(),
        &make_valid_post(),
        &make_valid_por(),
        &make_valid_poh(),
    );
    assert!(validation.is_ok());
    let validation = validation.unwrap();
    assert!(validation.is_valid());

    let mut bc = Blockchain::default();
    let block = make_block_with_prev(bc.tip_hash(), 1, make_address([1u8; 20]));
    let block_hash = bc.insert_block(block, &validation).unwrap();
    assert_eq!(bc.tip_height(), 1);
    assert_eq!(bc.tip_hash(), block_hash);

    let mut hotstuff_state = HotStuffState::new(100);
    hotstuff_state.current_height = 1;
    hotstuff_state.current_round = 1;
    let validator = make_address([1u8; 20]);
    for _ in 0..67 {
        hotstuff_state.add_vote(atoll_core::consensus::finality::hotstuff::BFTVote {
            validator: validator.clone(),
            block_hash,
            height: 1,
            round: 1,
            vote_type: VoteType::Commit,
            signature: vec![1; 64],
        });
    }

    let final_validation = consensus.validate_block_with_finality(
        &make_valid_pouw(),
        &make_valid_pos(),
        &make_valid_post(),
        &make_valid_por(),
        &make_valid_poh(),
        &hotstuff_state,
        &block_hash,
    );
    assert!(final_validation.is_ok());
    assert!(final_validation.unwrap().finality_achieved);

    bc.finalize(1);
    assert_eq!(bc.finalized_height(), 1);
}

trait GasUsedHelper {
    fn gas_used(&self) -> u64;
}

impl GasUsedHelper for ExecutionResult {
    fn gas_used(&self) -> u64 {
        match self {
            ExecutionResult::Success { gas_used, .. } => *gas_used,
            ExecutionResult::Revert { gas_used, .. } => *gas_used,
            ExecutionResult::Error { gas_used, .. } => *gas_used,
        }
    }
}
