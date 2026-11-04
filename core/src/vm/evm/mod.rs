//! EVM (Ethereum Virtual Machine) compatibility for the Atoll Protocol.
//!
//! Phase 1: Basic EVM opcode execution and gas metering.
//! Phase 2: Full EVM compatibility with precompiles and state access.

use crate::primitives::hash::Hash512;
use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMType};

/// EVM executor
pub struct EVMExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
}

impl EVMExecutor {
    pub fn new(context: ExecutionContext, gas_limit: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
        }
    }

    /// Executes EVM bytecode
    pub fn execute(&mut self, bytecode: &[u8], input: &[u8]) -> Result<ExecutionResult, String> {
        // Charge base gas for execution
        self.gas_meter.charge(bytecode.len() as u64 * 2)?;

        // Phase 1: Simple opcode interpreter
        // Phase 2: Full EVM opcode set with precompiles

        let mut pc: usize = 0;
        let mut return_data: Vec<u8> = Vec::new();

        while pc < bytecode.len() {
            let opcode = bytecode[pc];
            pc += 1;

            match opcode {
                0x00 => {
                    // STOP
                    return Ok(ExecutionResult::Success {
                        gas_used: self.gas_meter.gas_used,
                        return_data,
                        state_root: Hash512::zero(),
                    });
                }
                0x01 => {
                    // ADD
                    self.gas_meter.charge(3)?;
                    if pc + 2 > bytecode.len() {
                        return Err("Invalid ADD operands".to_string());
                    }
                    pc += 2;
                }
                0x02 => {
                    // MUL
                    self.gas_meter.charge(5)?;
                    if pc + 2 > bytecode.len() {
                        return Err("Invalid MUL operands".to_string());
                    }
                    pc += 2;
                }
                0x10 => {
                    // LT
                    self.gas_meter.charge(3)?;
                }
                0x11 => {
                    // GT
                    self.gas_meter.charge(3)?;
                }
                0x12 => {
                    // EQ
                    self.gas_meter.charge(3)?;
                }
                0x30 => {
                    // ADDRESS - push caller address
                    self.gas_meter.charge(2)?;
                    return_data.extend_from_slice(&self.context.caller.0);
                }
                0x31 => {
                    // BALANCE
                    self.gas_meter.charge(100)?;
                }
                0x32 => {
                    // ORIGIN
                    self.gas_meter.charge(2)?;
                    return_data.extend_from_slice(&self.context.origin.0);
                }
                0x33 => {
                    // CALLER
                    self.gas_meter.charge(2)?;
                    return_data.extend_from_slice(&self.context.caller.0);
                }
                0x34 => {
                    // CALLVALUE
                    self.gas_meter.charge(2)?;
                }
                0x35 => {
                    // CALLDATALOAD
                    self.gas_meter.charge(3)?;
                }
                0x36 => {
                    // CALLDATASIZE
                    self.gas_meter.charge(2)?;
                }
                0x37 => {
                    // CALLDATACOPY
                    self.gas_meter.charge(3)?;
                }
                0x3B => {
                    // CODESIZE
                    self.gas_meter.charge(2)?;
                    return_data.extend_from_slice(&(bytecode.len() as u32).to_le_bytes());
                }
                0x3C => {
                    // CODECOPY
                    self.gas_meter.charge(3)?;
                }
                0x5B => {
                    // JUMPDEST
                    self.gas_meter.charge(1)?;
                }
                0x5F => {
                    // PUSH0
                    self.gas_meter.charge(2)?;
                }
                0x60..=0x7F => {
                    // PUSH1-PUSH32
                    let push_size = (opcode as usize) - 0x5F;
                    if pc + push_size > bytecode.len() {
                        return Err("PUSH operand out of bounds".to_string());
                    }
                    self.gas_meter.charge(3)?;
                    pc += push_size;
                }
                0x80..=0x8F => {
                    // DUP1-DUP16
                    self.gas_meter.charge(3)?;
                }
                0x90..=0x9F => {
                    // SWAP1-SWAP16
                    self.gas_meter.charge(3)?;
                }
                0xF0 => {
                    // CREATE
                    self.gas_meter.charge(32000)?;
                }
                0xF1 => {
                    // CALL
                    self.gas_meter.charge(40)?;
                }
                0xF2 => {
                    // CALLCODE
                    self.gas_meter.charge(40)?;
                }
                0xF3 => {
                    // RETURN
                    return Ok(ExecutionResult::Success {
                        gas_used: self.gas_meter.gas_used,
                        return_data,
                        state_root: Hash512::zero(),
                    });
                }
                0xFD => {
                    // REVERT
                    return Ok(ExecutionResult::Revert {
                        gas_used: self.gas_meter.gas_used,
                        reason: "Execution reverted".to_string(),
                    });
                }
                0xFE => {
                    // INVALID
                    return Err("Invalid opcode".to_string());
                }
                0xFF => {
                    // SELFDESTRUCT
                    self.gas_meter.charge(5000)?;
                }
                _ => {
                    // Unknown opcode - skip in Phase 1
                }
            }
        }

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
        VMType::EVM
    }
}

impl Default for EVMExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 10_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evm_stop_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x00], &[]).unwrap();
        match result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(gas_used > 0);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_add_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x01, 0x01, 0x02, 0x00], &[]).unwrap();
        assert!(result.gas_used() > 0);
    }

    #[test]
    fn evm_address_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0x30, 0x00], &[]).unwrap();
        match result {
            ExecutionResult::Success { return_data, .. } => {
                assert_eq!(return_data.len(), 20);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_return_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0xF3], &[]).unwrap();
        match result {
            ExecutionResult::Success { gas_used, .. } => {
                assert!(gas_used > 0);
            }
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn evm_revert_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0xFD], &[]).unwrap();
        match result {
            ExecutionResult::Revert { reason, .. } => {
                assert_eq!(reason, "Execution reverted");
            }
            _ => panic!("Expected Revert"),
        }
    }

    #[test]
    fn evm_invalid_opcode() {
        let mut evm = EVMExecutor::default();
        let result = evm.execute(&[0xFE], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn evm_out_of_gas() {
        let mut evm = EVMExecutor::new(ExecutionContext::default(), 5);
        let result = evm.execute(&[0x01, 0x01, 0x02, 0x01, 0x01, 0x02, 0x00], &[]);
        assert!(result.is_err());
    }

    #[test]
    fn evm_vm_type() {
        assert_eq!(EVMExecutor::vm_type(), VMType::EVM);
    }

    #[test]
    fn evm_remaining_gas() {
        let mut evm = EVMExecutor::new(ExecutionContext::default(), 1000);
        evm.execute(&[0x00], &[]).unwrap();
        assert!(evm.remaining_gas() < 1000);
    }

    /// Helper to get gas_used from result
    trait GasUsed {
        fn gas_used(&self) -> u64;
    }

    impl GasUsed for ExecutionResult {
        fn gas_used(&self) -> u64 {
            match self {
                ExecutionResult::Success { gas_used, .. } => *gas_used,
                ExecutionResult::Revert { gas_used, .. } => *gas_used,
                ExecutionResult::Error { gas_used, .. } => *gas_used,
            }
        }
    }
}
