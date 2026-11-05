//! Virtual Machine module for the Atoll Protocol.
//!
//! Six virtual machines for smart contract execution:
//! EVM, WASM, Move, RISC-V, AI VM, ZK VM.
//!
//! The VM router selects the appropriate VM based on contract bytecode prefix.

pub mod ai_vm;
pub mod evm;
pub mod fuzzing;
pub mod gas;
pub mod move_vm;
pub mod riscv;
pub mod wasm;
pub mod zk_vm;

use crate::primitives::address::Address;
use crate::primitives::hash::Hash512;

/// VM type enum for the VM router
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMType {
    EVM,
    WASM,
    Move,
    RISCV,
    AI,
    ZK,
}

/// A VM execution log (event) emitted during contract execution.
#[derive(Debug, Clone)]
pub struct Log {
    /// Address of the contract that emitted the log.
    pub address: Address,
    /// Indexed topics associated with the log.
    pub topics: Vec<Hash512>,
    /// Unindexed data payload of the log.
    pub data: Vec<u8>,
}

/// Execution result from a VM
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Success {
        gas_used: u64,
        return_value: Vec<u8>,
        logs: Vec<Log>,
    },
    Revert {
        gas_used: u64,
        reason: String,
    },
    Error {
        gas_used: u64,
        message: String,
    },
}

/// Errors that can occur during VM execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VMError {
    OutOfGas,
    InvalidOpcode,
    StackOverflow,
    StackUnderflow,
    InvalidJump,
    InvalidMemoryAccess,
    InvalidInstruction,
    UnsupportedOperation,
    Panic,
}

impl std::fmt::Display for VMError {
    fn fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        match self {
            VMError::OutOfGas => write!(f, "out of gas"),
            VMError::InvalidOpcode => write!(f, "invalid opcode"),
            VMError::StackOverflow => write!(f, "stack overflow"),
            VMError::StackUnderflow => write!(f, "stack underflow"),
            VMError::InvalidJump => write!(f, "invalid jump"),
            VMError::InvalidMemoryAccess => write!(f, "invalid memory access"),
            VMError::InvalidInstruction => write!(f, "invalid instruction"),
            VMError::UnsupportedOperation => write!(f, "unsupported operation"),
            VMError::Panic => write!(f, "VM execution panicked"),
        }
    }
}

/// Gas metering for VM execution.
///
/// The [`GasMeter`] type, standard gas cost table, and `estimate_gas` live in
/// [`crate::vm::gas`] and are re-exported here for convenience.
pub use gas::GasMeter;

/// Execute a closure safely, catching panics and converting errors into
/// [`ExecutionResult::Error`] instead of propagating a panic to the host.
pub fn execute_safe<F, E>(f: F) -> ExecutionResult
where
    F: FnOnce() -> Result<ExecutionResult, E>,
    E: std::fmt::Display,
{
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(Ok(result)) => result,
        Ok(Err(err)) => ExecutionResult::Error {
            gas_used: 0,
            message: err.to_string(),
        },
        Err(_panic) => ExecutionResult::Error {
            gas_used: 0,
            message: "VM execution panicked".to_string(),
        },
    }
}

/// VM router: selects the appropriate VM based on bytecode prefix
pub struct VMRouter;

impl VMRouter {
    /// Routes a transaction to the appropriate VM based on bytecode prefix
    pub fn route(data: &[u8]) -> Result<VMType, String> {
        if data.is_empty() {
            return Err("Empty bytecode".to_string());
        }

        // Phase 1: Simple prefix-based routing
        // Phase 2: More sophisticated detection (magic bytes, headers, etc.)

        match data[0] {
            0x00 => Ok(VMType::EVM),   // EVM bytecode starts with 0x00
            0x01 => Ok(VMType::WASM),  // WASM magic number starts with 0x00 0x61 0x73 0x6d
            0x02 => Ok(VMType::Move),  // Move bytecode
            0x03 => Ok(VMType::RISCV), // RISC-V binary
            0x04 => Ok(VMType::AI),    // AI model (ONNX)
            0x05 => Ok(VMType::ZK),    // ZK VM circuit
            _ => Ok(VMType::EVM),      // Default to EVM for compatibility
        }
    }

    /// Executes bytecode on the VM selected by [`route`], using the provided
    /// [`ExecutionContext`].
    ///
    /// Panics raised inside a VM executor are caught by [`execute_safe`] and
    /// returned as `ExecutionResult::Error` rather than aborting the host.
    pub fn execute(
        context: ExecutionContext,
        bytecode: &[u8],
    ) -> ExecutionResult {
        let vm_type = match Self::route(bytecode) {
            Ok(vt) => vt,
            Err(err) => {
                return ExecutionResult::Error {
                    gas_used: 0,
                    message: err,
                }
            }
        };

        let result = match vm_type {
            VMType::EVM => {
                let mut evm = evm::EVMExecutor::new(context.clone(), context.gas_limit);
                execute_safe(|| evm.execute(bytecode, &[]))
            }
            VMType::WASM => {
                let mut wasm = wasm::WASMExecutor::new(context.clone(), context.gas_limit);
                execute_safe(|| wasm.execute(bytecode, "main", &[]))
            }
            VMType::Move => {
                let mut mv = move_vm::MoveExecutor::new(context.clone(), context.gas_limit);
                execute_safe(|| mv.execute(bytecode, "main", &[]))
            }
            VMType::RISCV => {
                let mut riscv = riscv::RISCVExecutor::new(context.clone(), context.gas_limit);
                execute_safe(|| riscv.execute(bytecode, 0x1000, &[]))
            }
            VMType::AI => {
                let mut ai = ai_vm::AIVMExecutor::new(context.clone(), context.gas_limit);
                execute_safe(|| ai.deploy_model(bytecode))
            }
            VMType::ZK => {
                let mut zk = zk_vm::ZKVMExecutor::new(context.clone(), context.gas_limit);
                let circuit_id = Hash512::blake3(bytecode);
                execute_safe(|| zk.execute(&circuit_id, &[], bytecode))
            }
        };

        result
    }
}

/// Transaction context for VM execution
#[derive(Debug, Clone)]
pub struct ExecutionContext {
    pub caller: Address,
    pub origin: Address,
    pub gas_price: u128,
    pub gas_limit: u64,
    pub block_number: u64,
    pub block_timestamp: u64,
    pub chain_id: u64,
    pub difficulty: u128,
    /// When true, state-changing operations must be rejected.
    pub is_static: bool,
}

impl Default for ExecutionContext {
    fn default() -> Self {
        Self {
            caller: Address::zero(),
            origin: Address::zero(),
            gas_price: 1,
            gas_limit: 10_000_000,
            block_number: 0,
            block_timestamp: 0,
            chain_id: 777777,
            difficulty: 1,
            is_static: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vm_router_evm_prefix() {
        let result = VMRouter::route(&[0x00, 0x60, 0x80]);
        assert_eq!(result, Ok(VMType::EVM));
    }

    #[test]
    fn vm_router_wasm_prefix() {
        let result = VMRouter::route(&[0x01, 0x61, 0x73, 0x6d]);
        assert_eq!(result, Ok(VMType::WASM));
    }

    #[test]
    fn vm_router_move_prefix() {
        let result = VMRouter::route(&[0x02, 0x00, 0x00]);
        assert_eq!(result, Ok(VMType::Move));
    }

    #[test]
    fn vm_router_riscv_prefix() {
        let result = VMRouter::route(&[0x03, 0x00, 0x00]);
        assert_eq!(result, Ok(VMType::RISCV));
    }

    #[test]
    fn vm_router_ai_prefix() {
        let result = VMRouter::route(&[0x04, 0x00, 0x00]);
        assert_eq!(result, Ok(VMType::AI));
    }

    #[test]
    fn vm_router_zk_prefix() {
        let result = VMRouter::route(&[0x05, 0x00, 0x00]);
        assert_eq!(result, Ok(VMType::ZK));
    }

    #[test]
    fn vm_router_empty_data_error() {
        let result = VMRouter::route(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn vm_router_default_evm() {
        let result = VMRouter::route(&[0xFF, 0x00]);
        assert_eq!(result, Ok(VMType::EVM));
    }

    #[test]
    fn gas_meter_charge() {
        let mut meter = GasMeter::new(100, 1);
        assert!(meter.charge(50).is_ok());
        assert_eq!(meter.gas_used, 50);
        assert_eq!(meter.remaining(), 50);
        assert!(meter.charge(60).is_err()); // Would exceed limit
    }

    #[test]
    fn gas_meter_charge_out_of_gas_is_vm_error() {
        let mut meter = GasMeter::new(10, 1);
        assert_eq!(meter.charge(11), Err(VMError::OutOfGas));
        assert_eq!(meter.gas_used, 0); // No gas consumed on failure
    }

    #[test]
    fn gas_meter_total_cost() {
        let mut meter = GasMeter::new(1000, 10);
        meter.charge(100).unwrap();
        assert_eq!(meter.total_cost(), 1000);
    }

    #[test]
    fn execution_context_default_has_is_static_false() {
        let ctx = ExecutionContext::default();
        assert!(!ctx.is_static);
    }

    #[test]
    fn execute_safe_returns_success_on_ok() {
        let result = execute_safe::<_, String>(|| {
            Ok(ExecutionResult::Success {
                gas_used: 10,
                return_value: vec![1, 2, 3],
                logs: vec![],
            })
        });
        match result {
            ExecutionResult::Success { gas_used, .. } => assert_eq!(gas_used, 10),
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn execute_safe_converts_error_to_error_result() {
        let result = execute_safe::<_, String>(|| Err("boom".to_string()));
        match result {
            ExecutionResult::Error { message, .. } => assert_eq!(message, "boom"),
            _ => panic!("Expected Error"),
        }
    }

    #[test]
    fn execute_safe_catches_panic() {
        let result = execute_safe::<_, String>(|| -> Result<ExecutionResult, String> {
            panic!("deliberate test panic");
        });
        match result {
            ExecutionResult::Error { message, .. } => {
                assert!(
                    message.contains("panicked"),
                    "unexpected message: {}",
                    message
                );
            }
            _ => panic!("Expected Error from caught panic"),
        }
    }

    #[test]
    fn vm_error_display_messages() {
        assert_eq!(VMError::OutOfGas.to_string(), "out of gas");
        assert_eq!(VMError::InvalidOpcode.to_string(), "invalid opcode");
        assert_eq!(VMError::Panic.to_string(), "VM execution panicked");
        assert_eq!(
            VMError::UnsupportedOperation.to_string(),
            "unsupported operation"
        );
    }

    #[test]
    fn vm_router_execute_returns_error_on_empty_bytecode() {
        let result = VMRouter::execute(ExecutionContext::default(), &[]);
        match result {
            ExecutionResult::Error { message, .. } => {
                assert!(!message.is_empty());
            }
            _ => panic!("Expected Error for empty bytecode"),
        }
    }

    #[test]
    fn vm_router_execute_evm_stop() {
        let result = VMRouter::execute(ExecutionContext::default(), &[0x00, 0x60, 0x80]);
        match result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(gas_used > 0);
            }
            ExecutionResult::Error { message, .. } => {
                panic!("EVM execution errored: {}", message)
            }
            _ => panic!("Expected Success"),
        }
    }
}
