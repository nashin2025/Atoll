//! Performance benchmarks for Phase 6 Testing & Auditing.
//!
//! Criterion benchmarks for hash functions, consensus, chain operations,
//! and VM execution.

use atoll_core::chain::blockchain::Blockchain;
use atoll_core::chain::sharding::cross_shard::CrossShardManager;
use atoll_core::chain::state::ChainState;
use atoll_core::consensus::penta::{poh, por, pos_vrf, post, pouw, PentaConsensus};
use atoll_core::crypto::morph_hash::MorphHashEngine;
use atoll_core::primitives::address::Address;
use atoll_core::primitives::block::{Block, BlockBody, BlockHeader};
use atoll_core::primitives::hash::Hash512;
use atoll_core::primitives::signature::Signature;
use atoll_core::primitives::transaction::Transaction;
use atoll_core::privacy::zk_starks::{StarkProver, StarkVerifier};
use atoll_core::vm::evm::EVMExecutor;
use atoll_core::vm::{ExecutionContext, GasMeter};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Hash benchmarks
// ---------------------------------------------------------------------------

fn bench_blake3_hash(c: &mut Criterion) {
    let input = b"Atoll Protocol benchmark input";
    c.bench_function("blake3_hash_64_bytes", |b| {
        b.iter(|| Hash512::blake3(black_box(input)))
    });
}

fn bench_blake3_hash_large_input(c: &mut Criterion) {
    let input = vec![0x42u8; 10_000];
    c.bench_function("blake3_hash_10kb", |b| {
        b.iter(|| Hash512::blake3(black_box(&input)))
    });
}

fn bench_hash512_from_hex(c: &mut Criterion) {
    let hex = "f".repeat(128);
    c.bench_function("hash512_from_hex", |b| {
        b.iter(|| Hash512::from_hex(black_box(&hex)).unwrap())
    });
}

fn bench_hash512_to_hex(c: &mut Criterion) {
    let hash = Hash512::blake3(b"test");
    c.bench_function("hash512_to_hex", |b| b.iter(|| black_box(hash.to_hex())));
}

// ---------------------------------------------------------------------------
// Morph-Hash benchmarks
// ---------------------------------------------------------------------------

fn bench_morph_hash(c: &mut Criterion) {
    let mut engine = MorphHashEngine::new();
    let input = b"Atoll Protocol morph hash benchmark";
    let prev_hash = Hash512::zero();
    c.bench_function("morph_hash", |b| {
        b.iter(|| engine.morph_hash(black_box(input), black_box(&prev_hash), black_box(1)))
    });
}

fn bench_morph_hash_production(c: &mut Criterion) {
    let mut engine = MorphHashEngine::with_memory_size(4096);
    let input = b"Atoll Protocol morph hash production benchmark";
    let prev_hash = Hash512::zero();
    c.bench_function("morph_hash_production", |b| {
        b.iter(|| engine.morph_hash(black_box(input), black_box(&prev_hash), black_box(1)))
    });
}

fn bench_select_algorithm_chain(c: &mut Criterion) {
    let prev_hash = Hash512::zero();
    c.bench_function("select_algorithm_chain", |b| {
        b.iter(|| {
            atoll_core::crypto::morph_hash::select_algorithm_chain(
                black_box(&prev_hash),
                black_box(100),
            )
        })
    });
}

// ---------------------------------------------------------------------------
// Address benchmarks
// ---------------------------------------------------------------------------

fn bench_address_to_hex(c: &mut Criterion) {
    let addr = Address::from_bytes([42u8; 20]);
    c.bench_function("address_to_hex", |b| b.iter(|| black_box(addr.to_hex())));
}

fn bench_address_from_hex(c: &mut Criterion) {
    let hex = "0x0102030405060708090a0b0c0d0e0f1011121314";
    c.bench_function("address_from_hex", |b| {
        b.iter(|| Address::from_hex(black_box(hex)).unwrap())
    });
}

// ---------------------------------------------------------------------------
// Transaction benchmarks
// ---------------------------------------------------------------------------

fn bench_transaction_serialize(c: &mut Criterion) {
    let tx = Transaction {
        version: 1,
        from: Address::from_bytes([1u8; 20]),
        to: Some(Address::from_bytes([2u8; 20])),
        value: 1_000_000,
        data: vec![1, 2, 3, 4, 5],
        signature: Signature::new([0u8; 64], vec![0u8; 2000]),
    };
    c.bench_function("transaction_serialize", |b| {
        b.iter(|| black_box(tx.serialize_bytes()))
    });
}

// ---------------------------------------------------------------------------
// Block benchmarks
// ---------------------------------------------------------------------------

fn bench_block_serialize(c: &mut Criterion) {
    let block = Block {
        header: BlockHeader {
            version: 1,
            height: 100,
            shard_id: 0,
            timestamp: 1798761600,
            prev_hash: Hash512::zero(),
            state_root: Hash512::zero(),
            transaction_root: Hash512::zero(),
            proposer: Address::from_bytes([1u8; 20]),
            difficulty: 1 << 20,
        },
        body: BlockBody {
            transactions: (0..10)
                .map(|_| Transaction {
                    version: 1,
                    from: Address::from_bytes([1u8; 20]),
                    to: Some(Address::from_bytes([2u8; 20])),
                    value: 100,
                    data: vec![1, 2, 3],
                    signature: Signature::default(),
                })
                .collect(),
        },
    };
    c.bench_function("block_serialize", |b| {
        b.iter(|| black_box(block.serialize_bytes()))
    });
}

// ---------------------------------------------------------------------------
// ChainState benchmarks
// ---------------------------------------------------------------------------

fn bench_chain_state_transfer(c: &mut Criterion) {
    let mut state = ChainState::new();
    let from = Address::from_bytes([1u8; 20]);
    let to = Address::from_bytes([2u8; 20]);
    state.set_balance(from, 1_000_000_000);
    c.bench_function("chain_state_transfer", |b| {
        b.iter(|| state.transfer(black_box(&from), black_box(&to), black_box(100)))
    });
}

fn bench_chain_state_balance(c: &mut Criterion) {
    let mut state = ChainState::new();
    let addr = Address::from_bytes([1u8; 20]);
    state.set_balance(addr.clone(), 1_000_000_000);
    c.bench_function("chain_state_balance", |b| {
        b.iter(|| black_box(state.balance(black_box(&addr))))
    });
}

// ---------------------------------------------------------------------------
// Consensus benchmarks
// ---------------------------------------------------------------------------

fn bench_penta_validate_block(c: &mut Criterion) {
    let consensus = PentaConsensus::new();
    let pouw = pouw::PoUWResult {
        valid: true,
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
    c.bench_function("penta_validate_block", |b| {
        b.iter(|| {
            consensus.validate_block(
                black_box(&pouw),
                black_box(&pos),
                black_box(&post),
                black_box(&por),
                black_box(&poh),
            )
        })
    });
}

// ---------------------------------------------------------------------------
// VM benchmarks
// ---------------------------------------------------------------------------

fn bench_evm_execute(c: &mut Criterion) {
    let context = ExecutionContext::default();
    let mut evm = EVMExecutor::new(context, 10_000_000);
    let bytecode = vec![0x01, 0x01, 0x02, 0x30, 0x00, 0xF3];
    c.bench_function("evm_execute", |b| {
        b.iter(|| evm.execute(black_box(&bytecode), black_box(&[])))
    });
}

fn bench_gas_meter_charge(c: &mut Criterion) {
    let mut meter = GasMeter::new(1_000_000, 1);
    c.bench_function("gas_meter_charge", |b| {
        b.iter(|| meter.charge(black_box(1000)))
    });
}

// ---------------------------------------------------------------------------
// Block validation benchmarks
// ---------------------------------------------------------------------------

fn bench_block_validation_insert(c: &mut Criterion) {
    let consensus = PentaConsensus::new();
    let pouw = pouw::PoUWResult {
        valid: true,
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
    let validation = consensus
        .validate_block(&pouw, &pos, &post, &por, &poh)
        .unwrap();
    c.bench_function("block_validation_insert", |b| {
        b.iter(|| {
            let mut bc = Blockchain::default();
            let block = Block {
                header: BlockHeader {
                    version: 1,
                    height: 1,
                    shard_id: 0,
                    timestamp: 0,
                    prev_hash: bc.tip_hash(),
                    state_root: Hash512::zero(),
                    transaction_root: Hash512::zero(),
                    proposer: Address::zero(),
                    difficulty: 1,
                },
                body: BlockBody {
                    transactions: Vec::new(),
                },
            };
            let _ = bc.insert_block(block, &validation);
        })
    });
}

fn bench_block_validation_with_tx(c: &mut Criterion) {
    let consensus = PentaConsensus::new();
    let pouw = pouw::PoUWResult {
        valid: true,
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
    let validation = consensus
        .validate_block(&pouw, &pos, &post, &por, &poh)
        .unwrap();
    c.bench_function("block_validation_with_10_tx", |b| {
        b.iter(|| {
            let mut bc = Blockchain::default();
            let mut txs = Vec::new();
            for _ in 0..10 {
                txs.push(Transaction {
                    version: 1,
                    from: Address::from_bytes([1u8; 20]),
                    to: Some(Address::from_bytes([2u8; 20])),
                    value: 100,
                    data: vec![1, 2, 3],
                    signature: Signature::default(),
                });
            }
            let block = Block {
                header: BlockHeader {
                    version: 1,
                    height: 1,
                    shard_id: 0,
                    timestamp: 0,
                    prev_hash: bc.tip_hash(),
                    state_root: Hash512::zero(),
                    transaction_root: Hash512::zero(),
                    proposer: Address::zero(),
                    difficulty: 1,
                },
                body: BlockBody { transactions: txs },
            };
            let _ = bc.insert_block(block, &validation);
        })
    });
}

// ---------------------------------------------------------------------------
// Transaction verification benchmarks
// ---------------------------------------------------------------------------

fn bench_transaction_hash(c: &mut Criterion) {
    let tx = Transaction {
        version: 1,
        from: Address::from_bytes([1u8; 20]),
        to: Some(Address::from_bytes([2u8; 20])),
        value: 1_000_000,
        data: vec![1, 2, 3, 4, 5],
        signature: Signature::new([0u8; 64], vec![0u8; 2000]),
    };
    let tx_bytes = tx.serialize_bytes();
    c.bench_function("transaction_hash_blake3", |b| {
        b.iter(|| Hash512::blake3(black_box(&tx_bytes)))
    });
}

// ---------------------------------------------------------------------------
// Cross-shard message processing benchmarks
// ---------------------------------------------------------------------------

fn bench_cross_shard_create_request(c: &mut Criterion) {
    let mut manager = CrossShardManager::new();
    c.bench_function("cross_shard_create_request", |b| {
        b.iter(|| {
            let id = manager.create_request(
                black_box(0u16),
                black_box(1u16),
                black_box(Address::from_bytes([1u8; 20])),
                black_box(Address::from_bytes([2u8; 20])),
                black_box(100_000_000u128),
                black_box(vec![1u8; 32]),
                black_box(1000u64),
            );
            black_box(id);
        })
    });
}

fn bench_cross_shard_full_lifecycle(c: &mut Criterion) {
    c.bench_function("cross_shard_full_lifecycle", |b| {
        b.iter(|| {
            let mut manager = CrossShardManager::new();
            let request_id = manager.create_request(
                0,
                1,
                Address::from_bytes([1u8; 20]),
                Address::from_bytes([2u8; 20]),
                50_000_000,
                vec![],
                1000,
            );
            let receipt_id = manager.request_to_receipt(&request_id).unwrap().id;
            let _ = manager.execute_receipt(&receipt_id, 1050);
        })
    });
}

fn bench_cross_shard_batch_100(c: &mut Criterion) {
    c.bench_function("cross_shard_batch_100", |b| {
        b.iter(|| {
            let mut manager = CrossShardManager::new();
            for i in 0..100u64 {
                let id = manager.create_request(
                    0,
                    1,
                    Address::from_bytes([1u8; 20]),
                    Address::from_bytes([2u8; 20]),
                    i as u128,
                    vec![],
                    1000,
                );
                let receipt_id = manager.request_to_receipt(&id).unwrap().id;
                let _ = manager.execute_receipt(&receipt_id, 1050);
            }
        })
    });
}

// ---------------------------------------------------------------------------
// zk-STARKs proof generation/verification benchmarks
// ---------------------------------------------------------------------------

fn bench_zk_starks_prove(c: &mut Criterion) {
    let prover = StarkProver::new(40, 256);
    let computation_hash = Hash512::blake3(b"benchmark_computation");
    let inputs = vec![vec![1u8; 32], vec![2u8; 32], vec![3u8; 32]];
    let outputs = vec![vec![4u8; 32]];
    c.bench_function("zk_starks_prove_40_queries", |b| {
        b.iter(|| {
            let _ = prover
                .prove(
                    black_box(computation_hash),
                    black_box(&inputs),
                    black_box(&outputs),
                )
                .unwrap();
        })
    });
}

fn bench_zk_starks_verify(c: &mut Criterion) {
    let prover = StarkProver::new(40, 256);
    let verifier = StarkVerifier::new();
    let computation_hash = Hash512::blake3(b"benchmark_computation");
    let inputs = vec![vec![1u8; 32], vec![2u8; 32], vec![3u8; 32]];
    let outputs = vec![vec![4u8; 32]];
    let proof = prover.prove(computation_hash, &inputs, &outputs).unwrap();
    c.bench_function("zk_starks_verify_40_queries", |b| {
        b.iter(|| {
            let _ = verifier
                .verify(
                    black_box(&proof),
                    black_box(computation_hash),
                    black_box(&inputs),
                )
                .unwrap();
        })
    });
}

fn bench_zk_starks_roundtrip(c: &mut Criterion) {
    c.bench_function("zk_starks_prove_verify_roundtrip", |b| {
        b.iter(|| {
            let prover = StarkProver::new(40, 256);
            let verifier = StarkVerifier::new();
            let computation_hash = Hash512::blake3(b"roundtrip_computation");
            let inputs = vec![vec![1u8; 32], vec![2u8; 32]];
            let outputs = vec![vec![3u8; 32]];
            let proof = prover
                .prove(
                    black_box(computation_hash),
                    black_box(&inputs),
                    black_box(&outputs),
                )
                .unwrap();
            let _ = verifier
                .verify(
                    black_box(&proof),
                    black_box(computation_hash),
                    black_box(&inputs),
                )
                .unwrap();
        })
    });
}

// ---------------------------------------------------------------------------
// Homomorphic encryption benchmarks (placeholder modules)
// ---------------------------------------------------------------------------

fn bench_homomorphic_placeholder(c: &mut Criterion) {
    // BFV and CKKS modules are placeholder files (Phase 0 research).
    // No public API exists yet; benchmark a no-op to establish baseline.
    c.bench_function("homomorphic_bfv_placeholder", |b| b.iter(|| black_box(())));
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .warm_up_time(Duration::from_millis(200))
        .measurement_time(Duration::from_millis(200));
    targets =
    bench_blake3_hash,
    bench_blake3_hash_large_input,
    bench_hash512_from_hex,
    bench_hash512_to_hex,
    bench_morph_hash,
    bench_morph_hash_production,
    bench_select_algorithm_chain,
    bench_address_to_hex,
    bench_address_from_hex,
    bench_transaction_serialize,
    bench_transaction_hash,
    bench_block_serialize,
    bench_block_validation_insert,
    bench_block_validation_with_tx,
    bench_chain_state_transfer,
    bench_chain_state_balance,
    bench_penta_validate_block,
    bench_evm_execute,
    bench_gas_meter_charge,
    bench_cross_shard_create_request,
    bench_cross_shard_full_lifecycle,
    bench_cross_shard_batch_100,
    bench_zk_starks_prove,
    bench_zk_starks_verify,
    bench_zk_starks_roundtrip,
    bench_homomorphic_placeholder,
}

criterion_main!(benches);
