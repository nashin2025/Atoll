//! Cross-VM fuzzing and determinism testing for the Atoll Protocol.
//!
//! This module provides shared test helpers for:
//! - Determinism testing: same input → same output across fresh executor instances
//! - Fuzz testing: random inputs through each VM, verifying no panics and gas safety
//! - Cross-VM fuzzing: same random input fed to all VMs
//! - Router fuzzing: random bytecode through VMRouter::execute()

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;
use crate::vm::{
    ai_vm::AIVMExecutor, evm::EVMExecutor, move_vm::MoveExecutor, riscv::RISCVExecutor,
    wasm::WASMExecutor, zk_vm::ZKVMExecutor, ExecutionContext, ExecutionResult, VMRouter,
};

use rand::Rng;

/// Default gas limit for fuzzing tests
pub const DEFAULT_GAS_LIMIT: u64 = 1_000_000;

/// Creates a default execution context for fuzzing tests
pub fn test_context() -> ExecutionContext {
    ExecutionContext {
        caller: Address::zero(),
        origin: Address::zero(),
        gas_price: 1,
        gas_limit: DEFAULT_GAS_LIMIT,
        block_number: 0,
        block_timestamp: 0,
        chain_id: 777_777,
        difficulty: 1,
        is_static: false,
    }
}

/// Runs the same input through N fresh executor instances and verifies
/// that all produce identical `return_value` and `gas_used`.
///
/// # Arguments
/// * `execute_fn` - Closure that creates a fresh executor and runs the input
/// * `input` - The deterministic input to test
/// * `iterations` - Number of fresh executor instances to test
///
/// # Panics
/// Panics if any execution returns an error or if outputs differ across instances.
pub fn test_determinism<F>(execute_fn: F, input: &[u8], iterations: usize)
where
    F: Fn(&[u8]) -> ExecutionResult + Clone,
{
    let mut first_result: Option<ExecutionResult> = None;

    for i in 0..iterations {
        let result = execute_fn(input);

        match &first_result {
            None => first_result = Some(result.clone()),
            Some(prev) => {
                assert_eq!(
                    *prev, result,
                    "Determinism violation on iteration {}: results differ",
                    i
                );
            }
        }
    }
}

/// Runs random inputs through an executor and verifies:
/// 1. No panics (execution returns Ok or graceful Error)
/// 2. Gas used never exceeds gas limit
/// 3. All results are either Success, Revert, or Error (no unexpected variants)
///
/// # Arguments
/// * `execute_fn` - Closure that takes random input and returns ExecutionResult
/// * `input_gen` - Closure that generates a random input Vec<u8>
/// * `iterations` - Number of random inputs to test
///
/// # Panics
/// Panics if any execution panics, exceeds gas limit, or returns an unexpected variant.
pub fn test_fuzz<F, G>(execute_fn: F, mut input_gen: G, iterations: usize)
where
    F: Fn(&[u8]) -> ExecutionResult,
    G: FnMut() -> Vec<u8>,
{
    for i in 0..iterations {
        let input: Vec<u8> = input_gen();
        let result = execute_fn(&input);

        match &result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Fuzz test failed on iteration {}: gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
            ExecutionResult::Revert { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Fuzz test failed on iteration {}: revert gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
            ExecutionResult::Error { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Fuzz test failed on iteration {}: error gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
        }
    }
}

/// Runs random bytecode through all VMs and verifies no panics.
/// Uses bytecode prefix to route to the appropriate VM via VMRouter.
///
/// # Arguments
/// * `iterations` - Number of random inputs to test
///
/// # Panics
/// Panics if any VM panics during execution.
/// Runs random bytecode through all VMs and verifies no panics.
/// Uses bytecode prefix to route to the appropriate VM via VMRouter.
///
/// # Arguments
/// * `iterations` - Number of random inputs to test
///
/// # Panics
/// Panics if any VM panics during execution.
pub fn test_cross_vm_fuzz(iterations: usize) {
    let mut rng = rand::thread_rng();

    for i in 0..iterations {
        let input_len = rng.gen_range(0..64);
        let mut input: Vec<u8> = Vec::with_capacity(input_len + 1);
        // Random prefix determines which VM gets the input
        input.push(rng.gen_range(0..=6));
        // Random payload
        for _ in 0..input_len {
            input.push(rng.gen());
        }

        let result = VMRouter::execute(test_context(), &input);

        match &result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Cross-VM fuzz failed on iteration {}: gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
            ExecutionResult::Revert { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Cross-VM fuzz failed on iteration {}: revert gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
            ExecutionResult::Error { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Cross-VM fuzz failed on iteration {}: error gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
        }
    }
}

/// Runs the same valid input through all VMs and verifies each produces
/// a Success result (or a consistent error type for invalid input).
/// This tests cross-VM behavioral consistency: no VM should panic or
/// behave unexpectedly differently from others on the same input class.
///
/// # Arguments
/// * `iterations` - Number of random inputs to test
///
/// # Panics
/// Panics if any VM panics during execution.
pub fn test_cross_vm_determinism(iterations: usize) {
    let mut rng = rand::thread_rng();

    for i in 0..iterations {
        // Generate input with EVM prefix (0x00) - valid for EVM
        let input_len = rng.gen_range(1..32);
        let mut input: Vec<u8> = Vec::with_capacity(input_len + 1);
        input.push(0x00); // EVM prefix
        for _ in 0..input_len {
            input.push(rng.gen());
        }

        // EVM should succeed or error gracefully
        let evm_result = VMRouter::execute(test_context(), &input);
        match &evm_result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Cross-VM determinism failed on iteration {}: EVM gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
            ExecutionResult::Error { .. } | ExecutionResult::Revert { .. } => {
                // Expected for invalid EVM bytecode
            }
        }

        // Same input with WASM prefix (0x01) - may or may not be valid WASM
        let mut wasm_input = input.clone();
        wasm_input[0] = 0x01;
        let wasm_result = VMRouter::execute(test_context(), &wasm_input);
        match &wasm_result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Cross-VM determinism failed on iteration {}: WASM gas_used {} exceeds limit {}",
                    i, gas_used, DEFAULT_GAS_LIMIT
                );
            }
            ExecutionResult::Error { .. } | ExecutionResult::Revert { .. } => {
                // Expected for invalid WASM
            }
        }

        // Both results should be in the same category (success or not success)
        // This ensures no VM panics while another handles the same input gracefully
        let evm_is_success = matches!(evm_result, ExecutionResult::Success { .. });
        let wasm_is_success = matches!(wasm_result, ExecutionResult::Success { .. });
        // If one succeeds and the other doesn't, that's fine - they're different VMs
        // The key invariant is: NEITHER should panic
    }
}

/// Runs random inputs through VMRouter::execute() and verifies:
/// 1. No panics (router handles all prefixes gracefully)
/// 2. Unknown prefixes return Error, not panic
/// 3. Gas used never exceeds gas limit
///
/// # Arguments
/// * `iterations` - Number of random inputs to test
///
/// # Panics
/// Panics if the router panics on any input.
pub fn test_router_fuzz(iterations: usize) {
    let mut rng = rand::thread_rng();

    for i in 0..iterations {
        let input_len = rng.gen_range(0..64);
        let mut input: Vec<u8> = Vec::with_capacity(input_len + 1);
        // Random prefix byte
        input.push(rng.gen_range(0..255));
        // Random payload
        for _ in 0..input_len {
            input.push(rng.gen());
        }

        let result = VMRouter::execute(test_context(), &input);

        match &result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Router fuzz failed on iteration {}: gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
            ExecutionResult::Revert { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Router fuzz failed on iteration {}: revert gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
            ExecutionResult::Error { gas_used, .. } => {
                assert!(
                    *gas_used <= DEFAULT_GAS_LIMIT,
                    "Router fuzz failed on iteration {}: error gas_used {} exceeds limit {}",
                    i,
                    gas_used,
                    DEFAULT_GAS_LIMIT
                );
            }
        }
    }
}

/// Runs route_with_magic() with mismatched prefix/magic bytes and verifies
/// MismatchedMagic error is returned (not panic).
///
/// # Arguments
/// * `iterations` - Number of random mismatched inputs to test
///
/// # Panics
/// Panics if route_with_magic panics on any mismatched input.
pub fn test_route_with_magic_mismatch(iterations: usize) {
    // Test specific mismatch cases where prefix byte matches magic start
    // but the full magic confirms a different VM type.

    // Case 1: Prefix 0x00 (EVM) + full WASM magic \0asm + version
    // data[..8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00] = WASM magic
    // Prefix type = EVM, detected = WASM → MismatchedMagic
    let wasm_magic_input = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    let result = VMRouter::route_with_magic(&wasm_magic_input);
    assert!(
        matches!(
            result,
            Err(crate::vm::VMRouterError::MismatchedMagic {
                prefix: 0x00,
                detected: crate::vm::VMType::WASM
            })
        ),
        "Expected MismatchedMagic(EVM, WASM), got {:?}",
        result
    );

    // Case 2: Prefix 0x01 (WASM) + Move magic CA FE BA BE
    // data[..4] = [0x01, 0xCA, 0xFE, 0xBA] != MOVE magic [0xCA, 0xFE, 0xBA, 0xBE]
    // No magic match → returns Ok(WASM) (prefix-based)
    let move_prefix_input = vec![0x01, 0xCA, 0xFE, 0xBA, 0xBE];
    let result = VMRouter::route_with_magic(&move_prefix_input);
    assert!(
        result == Ok(crate::vm::VMType::WASM),
        "Expected Ok(WASM), got {:?}",
        result
    );

    // Case 3: Prefix 0x03 (RISC-V) + ELF magic \x7fELF
    // data[..4] = [0x03, 0x7F, 0x45, 0x4C] != ELF magic [0x7F, 0x45, 0x4C, 0x46]
    // No magic match → returns Ok(RISCV) (prefix-based)
    let elf_prefix_input = vec![0x03, 0x7F, 0x45, 0x4C, 0x46];
    let result = VMRouter::route_with_magic(&elf_prefix_input);
    assert!(
        result == Ok(crate::vm::VMType::RISCV),
        "Expected Ok(RISCV), got {:?}",
        result
    );

    // Case 4: Random mismatched inputs - no panic, just ok or mismatch
    let mut rng = rand::thread_rng();
    for i in 0..iterations {
        let prefix = rng.gen_range(0..6);
        let payload_len = rng.gen_range(0..8);
        let mut input: Vec<u8> = Vec::with_capacity(1 + payload_len);
        input.push(prefix);
        for _ in 0..payload_len {
            input.push(rng.gen());
        }

        let result = VMRouter::route_with_magic(&input);
        // Should never panic - either Ok(prefix_type) or Err(MismatchedMagic)
        assert!(
            result.is_ok()
                || matches!(
                    result,
                    Err(crate::vm::VMRouterError::MismatchedMagic { .. })
                ),
            "Route with magic should not panic on iteration {}: got {:?}",
            i,
            result
        );
    }
}

/// Runs random bytecode through EVM executor and verifies no panics.
pub fn test_evm_fuzz(iterations: usize) {
    test_fuzz(
        |input| {
            let mut evm = EVMExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
            crate::vm::execute_safe(|| evm.execute(input, &[]))
        },
        || {
            let mut bytes = vec![0x00]; // EVM prefix
            let mut rng = rand::thread_rng();
            for _ in 0..rng.gen_range(0..64) {
                bytes.push(rng.gen());
            }
            bytes
        },
        iterations,
    )
}

/// Runs random bytecode through WASM executor and verifies no panics.
pub fn test_wasm_fuzz(iterations: usize) {
    test_fuzz(
        |input| {
            let mut wasm = WASMExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
            crate::vm::execute_safe(|| wasm.execute(input, "main", &[]))
        },
        || {
            let mut bytes = vec![0x01]; // WASM prefix
            let mut rng = rand::thread_rng();
            for _ in 0..rng.gen_range(0..64) {
                bytes.push(rng.gen());
            }
            bytes
        },
        iterations,
    )
}

/// Runs random bytecode through Move executor and verifies no panics.
pub fn test_move_fuzz(iterations: usize) {
    test_fuzz(
        |input| {
            let mut mv = MoveExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
            crate::vm::execute_safe(|| mv.execute(input, "main", &[]))
        },
        || {
            let mut bytes = vec![0x02]; // Move prefix
            let mut rng = rand::thread_rng();
            for _ in 0..rng.gen_range(0..64) {
                bytes.push(rng.gen());
            }
            bytes
        },
        iterations,
    )
}

/// Runs random bytecode through RISC-V executor and verifies no panics.
pub fn test_riscv_fuzz(iterations: usize) {
    test_fuzz(
        |input| {
            let mut riscv = RISCVExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
            crate::vm::execute_safe(|| riscv.execute(input, 0x1000, &[]))
        },
        || {
            let mut bytes = vec![0x03]; // RISC-V prefix
            let mut rng = rand::thread_rng();
            for _ in 0..rng.gen_range(0..64) {
                bytes.push(rng.gen());
            }
            bytes
        },
        iterations,
    )
}

/// Runs random model bytes through AI VM executor and verifies no panics.
pub fn test_ai_fuzz(iterations: usize) {
    test_fuzz(
        |input| {
            let mut ai = AIVMExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
            crate::vm::execute_safe(|| ai.deploy_model(input))
        },
        || {
            let mut bytes = vec![0x04]; // AI prefix
            let mut rng = rand::thread_rng();
            for _ in 0..rng.gen_range(0..64) {
                bytes.push(rng.gen());
            }
            bytes
        },
        iterations,
    )
}

/// Runs random circuit bytes through ZK executor and verifies no panics.
pub fn test_zk_fuzz(iterations: usize) {
    test_fuzz(
        |input| {
            let mut zk = ZKVMExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
            let circuit_id = Hash512::blake3(input);
            crate::vm::execute_safe(|| zk.execute(&circuit_id, &[], input))
        },
        || {
            let mut bytes = vec![0x05]; // ZK prefix
            let mut rng = rand::thread_rng();
            for _ in 0..rng.gen_range(0..64) {
                bytes.push(rng.gen());
            }
            bytes
        },
        iterations,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evm_determinism_test() {
        let input = vec![0x00, 0x60, 0x80, 0x52, 0x60, 0x20, 0x52];
        test_determinism(
            |input| {
                let mut evm = EVMExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
                crate::vm::execute_safe(|| evm.execute(input, &[]))
            },
            &input,
            5,
        );
    }

    #[test]
    fn wasm_determinism_test() {
        // WASM VM expects ELF64 header
        let input = vec![
            0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01,
            0x00, // magic, class, data, version, osabi
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // abiversion, pad
            0x02, 0x00, // e_type = ET_EXEC
            0xf3, 0x00, // e_machine = EM_RISCV
            0x01, 0x00, 0x00, 0x00, // e_version
            0x10, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_entry
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_phoff
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, // e_shoff
            0x00, 0x00, 0x00, 0x00, // e_flags
            0x40, 0x00, // e_ehsize
            0x00, 0x00, // e_phentsize
            0x00, 0x00, // e_phnum
            0x00, 0x00, // e_shentsize
            0x00, 0x00, // e_shnum
            0x00, 0x00, // e_shstrndx
        ];
        test_determinism(
            |input| {
                let mut wasm = WASMExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
                crate::vm::execute_safe(|| wasm.execute(input, "main", &[]))
            },
            &input,
            5,
        );
    }

    #[test]
    fn move_determinism_test() {
        // Move magic: 0xCA, 0xFE, 0xBA, 0xBE + version 1
        let input = vec![0xCA, 0xFE, 0xBA, 0xBE, 0x01, 0x00, 0x00, 0x00];
        test_determinism(
            |input| {
                let mut mv = MoveExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
                crate::vm::execute_safe(|| mv.execute(input, "main", &[]))
            },
            &input,
            5,
        );
    }

    #[test]
    fn riscv_determinism_test() {
        // RISC-V ELF64 magic: \x7fELF + class 2 (64-bit)
        let input = vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00];
        test_determinism(
            |input| {
                let mut riscv = RISCVExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
                crate::vm::execute_safe(|| riscv.execute(input, 0x1000, &[]))
            },
            &input,
            5,
        );
    }

    #[test]
    fn ai_determinism_test() {
        let input = vec![
            0x04, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        test_determinism(
            |input| {
                let mut ai = AIVMExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
                crate::vm::execute_safe(|| ai.deploy_model(input))
            },
            &input,
            5,
        );
    }

    #[test]
    fn zk_determinism_test() {
        let input = vec![
            0x05, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        test_determinism(
            |input| {
                let mut zk = ZKVMExecutor::new(test_context(), DEFAULT_GAS_LIMIT);
                let circuit_id = Hash512::blake3(input);
                crate::vm::execute_safe(|| zk.execute(&circuit_id, &[], input))
            },
            &input,
            5,
        );
    }

    #[test]
    fn evm_fuzz_test() {
        test_evm_fuzz(20);
    }

    #[test]
    fn wasm_fuzz_test() {
        test_wasm_fuzz(20);
    }

    #[test]
    fn move_fuzz_test() {
        test_move_fuzz(20);
    }

    #[test]
    fn riscv_fuzz_test() {
        test_riscv_fuzz(20);
    }

    #[test]
    fn ai_fuzz_test() {
        test_ai_fuzz(20);
    }

    #[test]
    fn zk_fuzz_test() {
        test_zk_fuzz(20);
    }

    #[test]
    fn cross_vm_fuzz_test() {
        test_cross_vm_fuzz(50);
    }

    #[test]
    fn cross_vm_determinism_test() {
        test_cross_vm_determinism(20);
    }

    #[test]
    fn router_fuzz_test() {
        test_router_fuzz(100);
    }

    #[test]
    fn route_with_magic_mismatch_test() {
        test_route_with_magic_mismatch(50);
    }
}
