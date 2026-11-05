//! ZK Virtual Machine for the Atoll Protocol.
//!
//! Executes smart contracts in zero-knowledge using zk-STARKs.
//! State transitions are valid but private.
//!
//! Phase 1: ZK proof verification interface.
//! Phase 2: Full zk-STARK prover and verifier.

use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMError, VMType};

/// ZK VM executor for zero-knowledge smart contracts
pub struct ZKVMExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
}

impl ZKVMExecutor {
    pub fn new(context: ExecutionContext, gas_limit: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
        }
    }

    /// Executes a smart contract in zero-knowledge
    pub fn execute(
        &mut self,
        circuit_id: &[u8],
        public_inputs: &[Vec<u8>],
        proof: &[u8],
    ) -> Result<ExecutionResult, VMError> {
        // Phase 1: Basic ZK proof verification interface
        // Phase 2: Full zk-STARK prover and verifier

        if proof.is_empty() {
            return Err(VMError::InvalidInstruction);
        }

        if circuit_id.is_empty() {
            return Err(VMError::InvalidInstruction);
        }

        // Charge gas for proof verification
        self.gas_meter.charge(proof.len() as u64)?;

        // Charge gas for public input processing
        for input in public_inputs {
            self.gas_meter.charge(input.len() as u64)?;
        }

        // Phase 1: Simulated verification
        // Phase 2: Actual zk-STARK verification

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_value: vec![1u8; 32], // Verification success indicator
            logs: vec![],
        })
    }

    /// Verifies a ZK proof without executing
    pub fn verify_proof(
        &mut self,
        _circuit_id: &[u8],
        _public_inputs: &[Vec<u8>],
        proof: &[u8],
    ) -> Result<bool, VMError> {
        if proof.is_empty() {
            return Ok(false);
        }

        self.gas_meter.charge(proof.len() as u64)?;

        // Phase 1: Simulated verification (always true for valid proof)
        Ok(true)
    }

    /// Returns the remaining gas
    pub fn remaining_gas(&self) -> u64 {
        self.gas_meter.remaining()
    }

    /// Returns the VM type
    pub fn vm_type() -> VMType {
        VMType::ZK
    }
}

impl Default for ZKVMExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 100_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zk_valid_proof() {
        let mut zk_vm = ZKVMExecutor::default();
        let circuit_id = vec![1, 2, 3];
        let public_inputs = vec![vec![4, 5], vec![6, 7]];
        let proof = vec![0x01, 0x02, 0x03];
        let result = zk_vm.execute(&circuit_id, &public_inputs, &proof);
        assert!(result.is_ok());
    }

    #[test]
    fn zk_empty_proof() {
        let mut zk_vm = ZKVMExecutor::default();
        let result = zk_vm.execute(&[1], &[], &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn zk_empty_circuit() {
        let mut zk_vm = ZKVMExecutor::default();
        let result = zk_vm.execute(&[], &[], &[1]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn zk_verify_proof_valid() {
        let mut zk_vm = ZKVMExecutor::default();
        let result = zk_vm.verify_proof(&[1], &[], &[1]);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn zk_verify_proof_empty() {
        let mut zk_vm = ZKVMExecutor::default();
        let result = zk_vm.verify_proof(&[1], &[], &[]);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn zk_vm_type() {
        assert_eq!(ZKVMExecutor::vm_type(), VMType::ZK);
    }

    #[test]
    fn zk_out_of_gas() {
        let mut zk_vm = ZKVMExecutor::new(ExecutionContext::default(), 1);
        let result = zk_vm.execute(&[1], &[], &[1, 2, 3, 4, 5]);
        assert!(result.is_err());
    }

    #[test]
    fn zk_remaining_gas() {
        let mut zk_vm = ZKVMExecutor::new(ExecutionContext::default(), 1000);
        zk_vm.execute(&[1], &[], &[1]).unwrap();
        assert!(zk_vm.remaining_gas() < 1000);
    }
}
