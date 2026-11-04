//! WASM (WebAssembly) runtime for the Atoll Protocol.
//!
//! Phase 1: Basic WASM module validation and execution.
//! Phase 2: Full WASM compatibility with gas metering and sandboxing.

use crate::primitives::hash::Hash512;
use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMType};

/// WASM executor
pub struct WASMExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
}

impl WASMExecutor {
    pub fn new(context: ExecutionContext, gas_limit: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
        }
    }

    /// Executes a WASM module
    pub fn execute(
        &mut self,
        wasm_bytes: &[u8],
        function: &str,
        args: &[Vec<u8>],
    ) -> Result<ExecutionResult, String> {
        // Phase 1: Basic WASM validation and execution simulation
        // Phase 2: Full WASM runtime with wasmtime or wasmer

        // Validate WASM magic number and version
        if wasm_bytes.len() < 8 {
            return Err("WASM module too short".to_string());
        }

        if &wasm_bytes[0..4] != b"\0asm" {
            return Err("Invalid WASM magic number".to_string());
        }

        let version =
            u32::from_le_bytes([wasm_bytes[4], wasm_bytes[5], wasm_bytes[6], wasm_bytes[7]]);
        if version != 1 {
            return Err(format!("Unsupported WASM version: {}", version));
        }

        // Charge gas for module validation
        self.gas_meter.charge(wasm_bytes.len() as u64)?;

        // Charge gas for function lookup
        self.gas_meter.charge(function.len() as u64 * 2)?;

        // Charge gas for arguments
        for arg in args {
            self.gas_meter.charge(arg.len() as u64)?;
        }

        // Phase 1: Simulated execution
        let mut return_data = Vec::new();
        return_data.extend_from_slice(b"wasm_exec_result");

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_data,
            state_root: Hash512::zero(),
        })
    }

    /// Returns the remaining gas
    pub fn remaining_gas(&self) -> u64 {
        self.gas_meter.remaining()
    }

    /// Returns the VM type
    pub fn vm_type() -> VMType {
        VMType::WASM
    }
}

impl Default for WASMExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 10_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wasm_valid_module() {
        let mut wasm = WASMExecutor::default();
        // Valid WASM header: \0asm + version 1
        let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let result = wasm.execute(&wasm_bytes, "main", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn wasm_invalid_magic() {
        let mut wasm = WASMExecutor::default();
        let wasm_bytes = vec![0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00];
        let result = wasm.execute(&wasm_bytes, "main", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Invalid WASM magic number");
    }

    #[test]
    fn wasm_unsupported_version() {
        let mut wasm = WASMExecutor::default();
        let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x02, 0x00, 0x00, 0x00];
        let result = wasm.execute(&wasm_bytes, "main", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn wasm_too_short() {
        let mut wasm = WASMExecutor::default();
        let wasm_bytes = vec![0x00, 0x61, 0x73];
        let result = wasm.execute(&wasm_bytes, "main", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "WASM module too short");
    }

    #[test]
    fn wasm_with_arguments() {
        let mut wasm = WASMExecutor::default();
        let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let args = vec![vec![1, 2, 3], vec![4, 5, 6]];
        let result = wasm.execute(&wasm_bytes, "add", &args);
        assert!(result.is_ok());
    }

    #[test]
    fn wasm_vm_type() {
        assert_eq!(WASMExecutor::vm_type(), VMType::WASM);
    }

    #[test]
    fn wasm_out_of_gas() {
        let mut wasm = WASMExecutor::new(ExecutionContext::default(), 5);
        let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        let result = wasm.execute(&wasm_bytes, "main", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn wasm_remaining_gas() {
        let mut wasm = WASMExecutor::new(ExecutionContext::default(), 1000);
        let wasm_bytes = vec![0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00];
        wasm.execute(&wasm_bytes, "main", &[]).unwrap();
        assert!(wasm.remaining_gas() < 1000);
    }
}
