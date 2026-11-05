//! RISC-V VM for the Atoll Protocol.
//!
//! Phase 1: RISC-V instruction decoding and execution simulation.
//! Phase 2: Full RV64IMC support with privileged modes.

use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMError, VMType};

/// RISC-V executor
pub struct RISCVExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
}

impl RISCVExecutor {
    pub fn new(context: ExecutionContext, gas_limit: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
        }
    }

    /// Executes a RISC-V binary
    pub fn execute(
        &mut self,
        binary: &[u8],
        entry_point: u64,
        args: &[Vec<u8>],
    ) -> Result<ExecutionResult, VMError> {
        // Phase 1: Basic RISC-V ELF validation
        // Phase 2: Full RV64IMC instruction set with privileged modes

        if binary.len() < 8 {
            return Err(VMError::InvalidInstruction);
        }

        // Validate ELF magic number
        if &binary[0..4] != b"\x7fELF" {
            return Err(VMError::InvalidInstruction);
        }

        // Charge gas for binary validation
        self.gas_meter.charge(binary.len() as u64)?;

        // Charge gas for entry point validation
        self.gas_meter.charge(8)?;

        // Charge gas for arguments
        for arg in args {
            self.gas_meter.charge(arg.len() as u64)?;
        }

        // Phase 1: Simulated execution
        let mut return_data = Vec::new();
        return_data.extend_from_slice(&entry_point.to_le_bytes());

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_value: return_data,
            logs: vec![],
        })
    }

    /// Returns the remaining gas
    pub fn remaining_gas(&self) -> u64 {
        self.gas_meter.remaining()
    }

    /// Returns the VM type
    pub fn vm_type() -> VMType {
        VMType::RISCV
    }
}

impl Default for RISCVExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 10_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn riscv_valid_elf() {
        let mut riscv = RISCVExecutor::default();
        // Valid ELF header: \x7fELF
        let binary = vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00];
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn riscv_invalid_elf() {
        let mut riscv = RISCVExecutor::default();
        let binary = vec![0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn riscv_too_short() {
        let mut riscv = RISCVExecutor::default();
        let binary = vec![0x7f, 0x45];
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn riscv_with_arguments() {
        let mut riscv = RISCVExecutor::default();
        let binary = vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00];
        let args = vec![vec![1], vec![2]];
        let result = riscv.execute(&binary, 0x2000, &args);
        assert!(result.is_ok());
    }

    #[test]
    fn riscv_vm_type() {
        assert_eq!(RISCVExecutor::vm_type(), VMType::RISCV);
    }

    #[test]
    fn riscv_out_of_gas() {
        let mut riscv = RISCVExecutor::new(ExecutionContext::default(), 5);
        let binary = vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00];
        let result = riscv.execute(&binary, 0x1000, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn riscv_remaining_gas() {
        let mut riscv = RISCVExecutor::new(ExecutionContext::default(), 1000);
        let binary = vec![0x7f, 0x45, 0x4c, 0x46, 0x02, 0x01, 0x01, 0x00];
        riscv.execute(&binary, 0x1000, &[]).unwrap();
        assert!(riscv.remaining_gas() < 1000);
    }
}
