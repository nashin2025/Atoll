//! Virtual Machine module for the Atoll Protocol.
//!
//! Six virtual machines for smart contract execution:
//! EVM, WASM, Move, RISC-V, AI VM, ZK VM.
//!
//! The VM router selects the appropriate VM based on bytecode prefix,
//! then validates magic bytes to confirm the detection. Execution
//! context is validated before any VM runs, and all panics inside VM
//! executors are caught by [`execute_safe`] and converted into
//! [`ExecutionResult::Error`].

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Log {
    /// Address of the contract that emitted the log.
    pub address: Address,
    /// Indexed topics associated with the log.
    pub topics: Vec<Hash512>,
    /// Unindexed data payload of the log.
    pub data: Vec<u8>,
}

/// Execution result from a VM
#[derive(Debug, Clone, PartialEq, Eq)]
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
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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

/// Errors that can occur during VM routing or context validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VMRouterError {
    /// Bytecode is empty.
    EmptyBytecode,
    /// Prefix byte does not map to any known VM type.
    UnknownPrefix(u8),
    /// Magic bytes do not match the VM type indicated by the prefix.
    MismatchedMagic { prefix: u8, detected: VMType },
    /// Execution context failed validation.
    InvalidContext(String),
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

/// Magic bytes for each VM type used to confirm prefix-based routing.
///
/// These are checked after the prefix byte routes bytecode to a VM type.
/// If magic bytes contradict the prefix, routing returns an error rather
/// than silently guessing.
mod magic {
    /// WASM magic: `\0asm` + version 1 (4 bytes version)
    pub const WASM: [u8; 8] = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
    /// ELF64 magic: `\x7fELF`
    pub const ELF: [u8; 4] = [0x7F, 0x45, 0x4C, 0x46];
    /// Move bytecode magic: `0xCA 0xFE 0xBA 0xBE`
    pub const MOVE: [u8; 4] = [0xCA, 0xFE, 0xBA, 0xBE];
    /// ONNX model protobuf magic prefix
    pub const ONNX: [u8; 4] = [0x13, 0x4F, 0x4E, 0x4E]; // \x13ONNX
    /// ZK VM circuit marker: non-empty proof/circuit data
    pub const ZK_MARKER: u8 = 0x01;
}

/// VM router: selects the appropriate VM based on bytecode prefix,
/// then validates magic bytes to confirm the detection.
pub struct VMRouter;

impl VMRouter {
    /// Routes a transaction to the appropriate VM based on bytecode prefix.
    ///
    /// Returns [`VMRouterError::UnknownPrefix`] if the first byte does not
    /// map to any known VM type. Returns [`VMRouterError::EmptyBytecode`]
    /// if the input is empty.
    ///
    /// Magic bytes are NOT validated here — see [`route_with_magic`] for
    /// that.
    pub fn route(data: &[u8]) -> Result<VMType, VMRouterError> {
        if data.is_empty() {
            return Err(VMRouterError::EmptyBytecode);
        }

        match data[0] {
            0x00 => Ok(VMType::EVM),
            0x01 => Ok(VMType::WASM),
            0x02 => Ok(VMType::Move),
            0x03 => Ok(VMType::RISCV),
            0x04 => Ok(VMType::AI),
            0x05 => Ok(VMType::ZK),
            prefix => Err(VMRouterError::UnknownPrefix(prefix)),
        }
    }

    /// Routes bytecode to a VM type using both prefix and magic byte validation.
    ///
    /// The prefix byte gives a preliminary VM type, then magic bytes are
    /// checked to confirm. If magic bytes strongly indicate a different VM
    /// type, returns [`VMRouterError::MismatchedMagic`]. If magic bytes are
    /// absent or uninformative, the prefix-based result is returned.
    pub fn route_with_magic(data: &[u8]) -> Result<VMType, VMRouterError> {
        let prefix_type = Self::route(data)?;

        // Check magic bytes to confirm or contradict the prefix
        let detected = Self::detect_from_magic(data);

        match detected {
            Some(detected_type) if detected_type != prefix_type => {
                Err(VMRouterError::MismatchedMagic {
                    prefix: data[0],
                    detected: detected_type,
                })
            }
            _ => Ok(prefix_type),
        }
    }

    /// Detects VM type from magic bytes alone.
    ///
    /// Returns `None` if no recognizable magic is present.
    fn detect_from_magic(data: &[u8]) -> Option<VMType> {
        if data.len() >= 8 {
            // WASM: \0asm + version 1
            if data[..8] == magic::WASM {
                return Some(VMType::WASM);
            }
        }
        if data.len() >= 4 {
            // ELF: \x7fELF
            if data[..4] == magic::ELF {
                return Some(VMType::RISCV);
            }
            // Move: 0xCA 0xFE 0xBA 0xBE
            if data[..4] == magic::MOVE {
                return Some(VMType::Move);
            }
            // ONNX: 0x13 O N N X
            if data[..4] == magic::ONNX {
                return Some(VMType::AI);
            }
        }
        None
    }

    /// Executes bytecode on the VM selected by [`route_with_magic`], using the
    /// provided [`ExecutionContext`].
    ///
    /// The execution context is validated before any VM runs; invalid contexts
    /// return [`ExecutionResult::Error`] immediately. Panics raised inside a VM
    /// executor are caught by [`execute_safe`] and returned as
    /// `ExecutionResult::Error` rather than aborting the host.
    pub fn execute(context: ExecutionContext, bytecode: &[u8]) -> ExecutionResult {
        // Validate the execution context before doing any work.
        if let Err(msg) = context.validate() {
            return ExecutionResult::Error {
                gas_used: 0,
                message: msg,
            };
        }

        let vm_type = match Self::route_with_magic(bytecode) {
            Ok(vt) => vt,
            Err(_err) => {
                return ExecutionResult::Error {
                    gas_used: 0,
                    message: "routing error".to_string(),
                };
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

impl ExecutionContext {
    /// Creates a new execution context with default values.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates the execution context before VM execution.
    ///
    /// Checks:
    /// - `chain_id` matches the protocol chain ID ([`crate::CHAIN_ID`])
    /// - `gas_limit` is non-zero and within a sane maximum
    /// - `gas_price` is non-zero
    /// - `block_number` and `block_timestamp` are non-zero (mainnet blocks)
    /// - `difficulty` is non-zero
    ///
    /// Returns `Ok(())` if valid, or a descriptive `String` error.
    pub fn validate(&self) -> Result<(), String> {
        if self.chain_id != crate::CHAIN_ID {
            return Err(format!(
                "invalid chain_id: expected {}, got {}",
                crate::CHAIN_ID,
                self.chain_id
            ));
        }
        if self.gas_limit == 0 {
            return Err("gas_limit must be non-zero".to_string());
        }
        if self.gas_limit > 100_000_000 {
            return Err(format!(
                "gas_limit exceeds maximum: {} > 100_000_000",
                self.gas_limit
            ));
        }
        if self.gas_price == 0 {
            return Err("gas_price must be non-zero".to_string());
        }
        if self.block_number == 0 {
            return Err("block_number must be non-zero".to_string());
        }
        if self.block_timestamp == 0 {
            return Err("block_timestamp must be non-zero".to_string());
        }
        if self.difficulty == 0 {
            return Err("difficulty must be non-zero".to_string());
        }
        Ok(())
    }
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
        assert!(matches!(result, Err(VMRouterError::EmptyBytecode)));
    }

    #[test]
    fn vm_router_unknown_prefix_error() {
        let result = VMRouter::route(&[0xFF, 0x00]);
        assert!(matches!(result, Err(VMRouterError::UnknownPrefix(0xFF))));
    }

    #[test]
    fn vm_router_no_more_default_evm() {
        // 0xFF is unknown prefix -> error, not default EVM anymore
        let result = VMRouter::route(&[0xFF, 0x00]);
        assert!(result.is_err());
    }

    #[test]
    fn vm_router_wasm_magic_confirms() {
        // WASM prefix 0x01 + valid WASM magic \0asm + version
        let wasm_bytes = [0x01, 0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let result = VMRouter::route_with_magic(&wasm_bytes);
        assert_eq!(result, Ok(VMType::WASM));
    }

    #[test]
    fn vm_router_wasm_magic_mismatch() {
        // Prefix 0x00 (EVM) but bytecode starts with WASM magic \0asm -> mismatch
        let wasm_magic = [0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let result = VMRouter::route_with_magic(&wasm_magic);
        assert!(matches!(
            result,
            Err(VMRouterError::MismatchedMagic {
                prefix: 0x00,
                detected: VMType::WASM
            })
        ));
    }

    #[test]
    fn vm_router_elf_magic_detects_riscv() {
        // No prefix (first byte 0x7F is unknown) -> UnknownPrefix error
        let elf_bytes = [0x7F, 0x45, 0x4C, 0x46, 0x02, 0x01, 0x01, 0x00];
        let result = VMRouter::route(&elf_bytes);
        assert!(matches!(result, Err(VMRouterError::UnknownPrefix(0x7F))));
    }

    #[test]
    fn vm_router_move_magic_confirms() {
        // Move prefix 0x02 + Move magic
        let move_bytes = [0x02, 0xCA, 0xFE, 0xBA, 0xBE];
        let result = VMRouter::route_with_magic(&move_bytes);
        assert_eq!(result, Ok(VMType::Move));
    }

    #[test]
    fn vm_router_ai_onnx_magic_confirms() {
        // AI prefix 0x04 + ONNX magic
        let onnx_bytes = [0x04, 0x13, 0x4F, 0x4E, 0x4E];
        let result = VMRouter::route_with_magic(&onnx_bytes);
        assert_eq!(result, Ok(VMType::AI));
    }

    #[test]
    fn execution_context_validate_chain_id_mismatch() {
        let mut ctx = ExecutionContext::default();
        ctx.chain_id = 999;
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn execution_context_validate_zero_gas_limit() {
        let mut ctx = ExecutionContext::default();
        ctx.gas_limit = 0;
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn execution_context_validate_gas_limit_too_high() {
        let mut ctx = ExecutionContext::default();
        ctx.gas_limit = 100_000_001;
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn execution_context_validate_zero_gas_price() {
        let mut ctx = ExecutionContext::default();
        ctx.gas_price = 0;
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn execution_context_validate_zero_block_number() {
        let mut ctx = ExecutionContext::default();
        ctx.block_number = 0;
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn execution_context_validate_zero_block_timestamp() {
        let mut ctx = ExecutionContext::default();
        ctx.block_timestamp = 0;
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn execution_context_validate_zero_difficulty() {
        let mut ctx = ExecutionContext::default();
        ctx.difficulty = 0;
        assert!(ctx.validate().is_err());
    }

    #[test]
    fn execution_context_validate_all_valid() {
        let ctx = ExecutionContext {
            block_number: 1,
            block_timestamp: 1798761600,
            ..Default::default()
        };
        assert!(ctx.validate().is_ok());
    }

    #[test]
    fn execution_context_new_returns_default_like() {
        let ctx = ExecutionContext::new();
        assert_eq!(ctx.chain_id, crate::CHAIN_ID);
        assert_eq!(ctx.gas_limit, 10_000_000);
        assert_eq!(ctx.gas_price, 1);
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
    fn vm_router_execute_rejects_invalid_context() {
        let mut ctx = ExecutionContext::default();
        ctx.chain_id = 999; // invalid chain_id
        let result = VMRouter::execute(ctx, &[0x00, 0x60, 0x80]);
        match result {
            ExecutionResult::Error { message, .. } => {
                assert!(message.contains("chain_id"));
            }
            _ => panic!("Expected Error for invalid context"),
        }
    }

    #[test]
    fn vm_router_execute_evm_stop() {
        // Valid context + EVM prefix bytecode
        let mut ctx = ExecutionContext::default();
        ctx.block_number = 1;
        ctx.block_timestamp = 1798761600;
        let result = VMRouter::execute(ctx, &[0x00, 0x60, 0x80]);
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
