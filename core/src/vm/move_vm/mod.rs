//! Move VM for the Atoll Protocol.
//!
//! Phase 1: Move bytecode validation and execution simulation.
//! Phase 2: Full Move VM with resource-oriented programming.

use crate::primitives::hash::Hash512;
use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMType};

/// Move VM executor
pub struct MoveExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
}

impl MoveExecutor {
    pub fn new(context: ExecutionContext, gas_limit: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
        }
    }

    /// Executes Move bytecode
    pub fn execute(
        &mut self,
        bytecode: &[u8],
        function: &str,
        args: &[Vec<u8>],
    ) -> Result<ExecutionResult, String> {
        // Phase 1: Basic Move bytecode validation
        // Phase 2: Full Move VM with resource-oriented type system

        if bytecode.is_empty() {
            return Err("Empty Move bytecode".to_string());
        }

        // Charge gas for module validation
        self.gas_meter.charge(bytecode.len() as u64)?;

        // Charge gas for function lookup
        self.gas_meter.charge(function.len() as u64 * 2)?;

        // Charge gas for arguments
        for arg in args {
            self.gas_meter.charge(arg.len() as u64)?;
        }

        // Phase 1: Simulated execution
        let mut return_data = Vec::new();
        return_data.extend_from_slice(b"move_exec_result");

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
        VMType::Move
    }
}

impl Default for MoveExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 10_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_valid_bytecode() {
        let mut move_vm = MoveExecutor::default();
        let bytecode = vec![0x01, 0x02, 0x03, 0x04];
        let result = move_vm.execute(&bytecode, "main", &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn move_empty_bytecode() {
        let mut move_vm = MoveExecutor::default();
        let result = move_vm.execute(&[], "main", &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Empty Move bytecode");
    }

    #[test]
    fn move_with_arguments() {
        let mut move_vm = MoveExecutor::default();
        let bytecode = vec![0x01, 0x02];
        let args = vec![vec![1], vec![2]];
        let result = move_vm.execute(&bytecode, "add", &args);
        assert!(result.is_ok());
    }

    #[test]
    fn move_vm_type() {
        assert_eq!(MoveExecutor::vm_type(), VMType::Move);
    }

    #[test]
    fn move_out_of_gas() {
        let mut move_vm = MoveExecutor::new(ExecutionContext::default(), 5);
        let bytecode = vec![0x01, 0x02, 0x03, 0x04, 0x05, 0x06];
        let result = move_vm.execute(&bytecode, "main", &[]);
        assert!(result.is_err());
    }

    #[test]
    fn move_remaining_gas() {
        let mut move_vm = MoveExecutor::new(ExecutionContext::default(), 1000);
        let bytecode = vec![0x01];
        move_vm.execute(&bytecode, "main", &[]).unwrap();
        assert!(move_vm.remaining_gas() < 1000);
    }
}
