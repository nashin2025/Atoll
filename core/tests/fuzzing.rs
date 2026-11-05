//! Fuzzing tests for Phase 6 Testing & Auditing.
//!
//! Property-based tests using iterative fuzzing to verify invariants
//! hold across a wide range of inputs.

use atoll_core::chain::state::ChainState;
use atoll_core::consensus::penta::{poh, por, pos_vrf, post, pouw, PentaConsensus};
use atoll_core::crypto::morph_hash::{select_algorithm_chain, MorphHashEngine};
use atoll_core::primitives::address::Address;
use atoll_core::primitives::block::{Block, BlockBody, BlockHeader};
use atoll_core::primitives::hash::Hash512;
use atoll_core::primitives::signature::Signature;
use atoll_core::primitives::transaction::Transaction;

// ---------------------------------------------------------------------------
// Hash512 fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_hash512_hex_roundtrip() {
    for i in 0..10_000u32 {
        let mut bytes = [0u8; 64];
        for (j, b) in bytes.iter_mut().enumerate() {
            *b = ((i * 31 + (j as u32) * 17) % 256) as u8;
        }
        let hash = Hash512::from_array(bytes);
        let hex = hash.to_hex();
        let restored = Hash512::from_hex(&hex).unwrap();
        assert_eq!(hash, restored);
    }
}

#[test]
fn fuzz_hash512_blake3_deterministic() {
    for i in 0..10_000u32 {
        let input: Vec<u8> = (0..(i % 100)).map(|j| (i * 7 + j * 3) as u8).collect();
        let h1 = Hash512::blake3(&input);
        let h2 = Hash512::blake3(&input);
        assert_eq!(h1, h2);
    }
}

#[test]
fn fuzz_hash512_output_is_64_bytes() {
    for i in 0..10_000u32 {
        let input: Vec<u8> = (0..(i % 1000))
            .map(|j| (i * 5 + (j as u32)) as u8)
            .collect();
        let hash = Hash512::blake3(&input);
        assert_eq!(hash.to_array().len(), 64);
    }
}

#[test]
fn fuzz_hash512_zero_and_default() {
    assert_eq!(Hash512::zero().0, [0u8; 64]);
    assert_eq!(Hash512::default(), Hash512::zero());
}

// ---------------------------------------------------------------------------
// Address fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_address_hex_roundtrip() {
    for i in 0..10_000u32 {
        let mut bytes = [0u8; 20];
        for (j, b) in bytes.iter_mut().enumerate() {
            *b = ((i * 13 + (j as u32) * 7) % 256) as u8;
        }
        let addr = Address::from_bytes(bytes);
        let hex = addr.to_hex();
        let restored = Address::from_hex(&hex).unwrap();
        assert_eq!(addr, restored);
    }
}

#[test]
fn fuzz_address_zero_and_default() {
    assert_eq!(Address::zero().0, [0u8; 20]);
    assert_eq!(Address::default(), Address::zero());
}

// ---------------------------------------------------------------------------
// Transaction fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_transaction_serialize_deterministic() {
    for i in 0..1_000u32 {
        let tx = Transaction {
            version: (i % 255) as u8,
            from: Address::from_bytes([i as u8; 20]),
            to: if i % 2 == 0 {
                Some(Address::from_bytes([(i + 1) as u8; 20]))
            } else {
                None
            },
            value: i as u128 * 1000,
            data: (0..(i % 50)).map(|j| j as u8).collect(),
            signature: Signature::default(),
        };
        let bytes1 = tx.serialize_bytes();
        let bytes2 = tx.serialize_bytes();
        assert_eq!(bytes1, bytes2);
    }
}

#[test]
fn fuzz_transaction_default_values() {
    let tx = Transaction::default();
    assert_eq!(tx.version, 1);
    assert_eq!(tx.from, Address::zero());
    assert_eq!(tx.to, None);
    assert_eq!(tx.value, 0);
    assert!(tx.data.is_empty());
}

#[test]
fn fuzz_transaction_u128_values() {
    for val in [0u128, 1, 100, u128::MAX / 2, u128::MAX] {
        let tx = Transaction {
            version: 1,
            from: Address::zero(),
            to: None,
            value: val,
            data: Vec::new(),
            signature: Signature::default(),
        };
        assert_eq!(tx.value, val);
    }
}

// ---------------------------------------------------------------------------
// Signature fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_signature_default_values() {
    let sig = Signature::default();
    assert_eq!(sig.ed25519, [0u8; 64]);
    assert!(sig.dilithium.is_empty());
    assert_eq!(sig.total_size(), 64);
}

#[test]
fn fuzz_signature_total_size() {
    for dilithium_len in [0usize, 1, 100, 1000, 5000, 10000] {
        let sig = Signature::new([0u8; 64], vec![0u8; dilithium_len]);
        assert_eq!(sig.total_size(), 64 + dilithium_len);
        assert_eq!(sig.dilithium_len(), dilithium_len);
    }
}

#[test]
fn fuzz_signature_dilithium_empty() {
    let sig = Signature::new([0u8; 64], vec![]);
    assert_eq!(sig.total_size(), 64);
    assert_eq!(sig.dilithium_len(), 0);
}

// ---------------------------------------------------------------------------
// Block fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_block_serialize_deterministic() {
    for i in 0..1_000u32 {
        let block = Block {
            header: BlockHeader {
                version: 1,
                height: i as u64,
                shard_id: (i % 64) as u16,
                timestamp: (i * 1000) as u64,
                prev_hash: Hash512::from_bytes(&[i as u8; 64]).unwrap(),
                state_root: Hash512::from_bytes(&[(i + 1) as u8; 64]).unwrap(),
                transaction_root: Hash512::from_bytes(&[(i + 2) as u8; 64]).unwrap(),
                proposer: Address::from_bytes([i as u8; 20]),
                difficulty: i as u128 * 1000,
            },
            body: BlockBody {
                transactions: Vec::new(),
            },
        };
        let bytes1 = block.serialize_bytes();
        let bytes2 = block.serialize_bytes();
        assert_eq!(bytes1, bytes2);
    }
}

#[test]
fn fuzz_block_default_has_genesis_values() {
    let block = Block::default();
    assert_eq!(block.header.height, 0);
    assert_eq!(block.header.prev_hash, Hash512::zero());
    assert!(block.body.transactions.is_empty());
}

// ---------------------------------------------------------------------------
// ChainState fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_chain_state_balance_default_is_zero() {
    for i in 0..1_000u32 {
        let addr = Address::from_bytes([i as u8; 20]);
        let state = ChainState::new();
        assert_eq!(state.balance(&addr), 0);
        assert_eq!(state.nonce(&addr), 0);
    }
}

#[test]
fn fuzz_chain_state_transfer_insufficient_fails() {
    for i in 0..1_000u32 {
        let from = Address::from_bytes([i as u8; 20]);
        let to = Address::from_bytes([(i + 1) as u8; 20]);
        let balance = i as u128 * 100;
        let amount = balance + 1;

        let mut state = ChainState::new();
        state.set_balance(from.clone(), balance);

        let result = state.transfer(&from, &to, amount);
        assert!(result.is_err());
        assert_eq!(state.balance(&from), balance);
        assert_eq!(state.balance(&to), 0);
    }
}

#[test]
fn fuzz_chain_state_transfer_preserves_supply() {
    for i in 0..1_000u32 {
        let from = Address::from_bytes([i as u8; 20]);
        let to = Address::from_bytes([(i + 1) as u8; 20]);
        let balance = i as u128 * 1000 + 100;
        let amount = i as u128 * 100;

        let mut state = ChainState::new();
        state.set_balance(from.clone(), balance);
        let initial = state.balance(&from) + state.balance(&to);

        let result = state.transfer(&from, &to, amount);
        assert!(result.is_ok());
        let final_total = state.balance(&from) + state.balance(&to);
        assert_eq!(initial, final_total);
        assert_eq!(state.nonce(&from), 1);
    }
}

// ---------------------------------------------------------------------------
// Consensus fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_penta_weights_sum_to_one() {
    let w = PentaConsensus::new().weights();
    let total = w.useful_work + w.stake_vrf + w.spacetime + w.reputation + w.humanness;
    assert!((total - 1.0).abs() < 0.001);
}

#[test]
fn fuzz_penta_composite_score_in_range() {
    for i in 0..1_000u32 {
        let pouw_score = (i % 100) as f64 / 100.0;
        let pos_score = ((i * 7) % 100) as f64 / 100.0;
        let post_score = ((i * 13) % 100) as f64 / 100.0;
        let por_score = ((i * 17) % 100) as f64 / 100.0;
        let poh_score = ((i * 19) % 100) as f64 / 100.0;

        let consensus = PentaConsensus::new();
        let pouw = pouw::PoUWResult {
            valid: true,
            usefulness_score: pouw_score,
            difficulty_met: true,
        };
        let pos = pos_vrf::PoSResult {
            valid: true,
            combined_score: pos_score,
        };
        let post = post::PoSTResult {
            valid: true,
            size_score: post_score,
        };
        let por = por::PoRResult {
            raw_score: por_score,
            decayed_score: por_score,
            is_eligible: true,
        };
        let poh = poh::PoHResult {
            is_verified: poh_score > 0.5,
        };

        if let Ok(validation) = consensus.validate_block(&pouw, &pos, &post, &por, &poh) {
            assert!(validation.composite_score >= 0.0);
            assert!(validation.composite_score <= 1.0);
        }
    }
}

// ---------------------------------------------------------------------------
// GasMeter fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_gas_meter_never_exceeds_limit() {
    for gas_limit in [1u64, 10, 100, 1000, 10000] {
        let mut meter = atoll_core::vm::GasMeter::new(gas_limit, 1);
        for charge in [1u64, 5, 10, 50, 100] {
            if meter.remaining() >= charge {
                assert!(meter.charge(charge).is_ok());
                assert!(meter.gas_used <= gas_limit);
            } else {
                assert!(meter.charge(charge).is_err());
            }
        }
        assert!(meter.remaining() <= gas_limit);
        assert!(meter.gas_used <= gas_limit);
    }
}

#[test]
fn fuzz_gas_meter_remaining_is_limit_minus_used() {
    for gas_limit in [1u64, 100, 1000] {
        for charge in [1u64, 10, 100] {
            let mut meter = atoll_core::vm::GasMeter::new(gas_limit, 1);
            if charge <= gas_limit {
                meter.charge(charge).unwrap();
                assert_eq!(meter.remaining(), gas_limit - charge);
                assert_eq!(meter.gas_used, charge);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// MorphHash fuzzing
// ---------------------------------------------------------------------------

#[test]
fn fuzz_morph_hash_deterministic() {
    for i in 0..1_000u32 {
        let input: Vec<u8> = (0..(i % 50)).map(|j| (i * 3 + (j as u32)) as u8).collect();
        let height = (i % 100) as u64;
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();
        let h1 = engine.morph_hash(&input, &prev_hash, height);
        let h2 = engine.morph_hash(&input, &prev_hash, height);
        assert_eq!(h1, h2);
    }
}

#[test]
fn fuzz_morph_hash_output_is_64_bytes() {
    for i in 0..1_000u32 {
        let input: Vec<u8> = (0..(i % 50)).map(|j| (i * 3 + j) as u8).collect();
        let mut engine = MorphHashEngine::new();
        let prev_hash = Hash512::zero();
        let hash = engine.morph_hash(&input, &prev_hash, 0);
        assert_eq!(hash.to_array().len(), 64);
    }
}

#[test]
fn fuzz_select_algorithm_chain_always_16() {
    for i in 0..1_000u32 {
        let mut bytes = [0u8; 64];
        for (j, b) in bytes.iter_mut().enumerate() {
            *b = ((i * 7 + (j as u32) * 11) % 256) as u8;
        }
        let prev_hash = Hash512::from_array(bytes);
        let height = (i % 1000) as u64;
        let chain = select_algorithm_chain(&prev_hash, height);
        assert_eq!(chain.len(), 16);
    }
}
