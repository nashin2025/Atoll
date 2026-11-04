//! Virtual Machine module for the Atoll Protocol.
//!
//! Six virtual machines for smart contract execution:
//! EVM, WASM, Move, RISC-V, AI VM, ZK VM.
//!
//! The VM router selects the appropriate VM based on contract bytecode prefix.

pub mod ai_vm;
pub mod evm;
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

/// Execution result from a VM
#[derive(Debug, Clone)]
pub enum ExecutionResult {
    Success {
        gas_used: u64,
        return_data: Vec<u8>,
        state_root: Hash512,
    },
    Revert {
        gas_used: u64,
        reason: String,
    },
    Error {
        gas_used: u64,
        error: String,
    },
}

/// Gas metering for VM execution
#[derive(Debug, Clone, Default)]
pub struct GasMeter {
    pub gas_limit: u64,
    pub gas_used: u64,
    pub gas_price: u128,
}

impl GasMeter {
    pub fn new(gas_limit: u64, gas_price: u128) -> Self {
        Self {
            gas_limit,
            gas_used: 0,
            gas_price,
        }
    }

    pub fn charge(&mut self, gas: u64) -> Result<(), String> {
        if self.gas_used + gas > self.gas_limit {
            return Err("Out of gas".to_string());
        }
        self.gas_used += gas;
        Ok(())
    }

    pub fn remaining(&self) -> u64 {
        self.gas_limit.saturating_sub(self.gas_used)
    }

    pub fn total_cost(&self) -> u128 {
        self.gas_used as u128 * self.gas_price
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
    fn gas_meter_total_cost() {
        let mut meter = GasMeter::new(1000, 10);
        meter.charge(100).unwrap();
        assert_eq!(meter.total_cost(), 1000);
    }
}
