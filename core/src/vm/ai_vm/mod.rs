//! AI Virtual Machine for the Atoll Protocol.
//!
//! Executes AI model inference on-chain using ONNX runtime.
//! Results are deterministic and verifiable.
//!
//! Phase 1: ONNX model loading and deterministic inference simulation.
//! Phase 2: Full ONNX runtime with gas metering and verification proofs.

use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMError, VMType};

/// AI VM executor for on-chain AI inference
pub struct AIVMExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
    pub max_model_size: usize,
    pub max_inference_gas: u64,
}

impl AIVMExecutor {
    pub fn new(
        context: ExecutionContext,
        gas_limit: u64,
    ) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
            max_model_size: 100 * 1024 * 1024, // 100 MB
            max_inference_gas: 10_000_000,
        }
    }

    /// Deploys an AI model on-chain
    pub fn deploy_model(
        &mut self,
        model_bytes: &[u8],
    ) -> Result<ExecutionResult, VMError> {
        if model_bytes.len() > self.max_model_size {
            return Err(VMError::UnsupportedOperation);
        }

        // Validate ONNX model (Phase 1: basic check)
        if model_bytes.len() < 12 {
            return Err(VMError::InvalidInstruction);
        }

        // Charge gas for model deployment
        let deploy_gas = model_bytes.len() as u64 / 1024 * 100; // 100 gas per KB
        self.gas_meter.charge(deploy_gas)?;

        // Charge gas for model validation
        self.gas_meter.charge(10_000)?;

        let model_hash = crate::primitives::hash::Hash512::blake3(model_bytes);

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_value: model_hash.to_bytes(),
            logs: vec![],
        })
    }

    /// Executes AI model inference
    pub fn execute_inference(
        &mut self,
        model_hash: &crate::primitives::hash::Hash512,
        input_data: &[u8],
    ) -> Result<ExecutionResult, VMError> {
        // Charge gas for model lookup
        self.gas_meter.charge(1_000)?;

        // Charge gas for inference based on input size
        let inference_gas = (input_data.len() as u64 / 1024 + 1) * 100;
        self.gas_meter
            .charge(inference_gas.min(self.max_inference_gas))?;

        // Phase 1: Simulated deterministic inference
        let mut output = Vec::new();
        output.extend_from_slice(b"ai_inference_result");

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_value: output,
            logs: vec![],
        })
    }

    /// Verifies an AI inference proof
    pub fn verify_inference(
        &mut self,
        _model_hash: &crate::primitives::hash::Hash512,
        _input_data: &[u8],
        _output: &[u8],
        proof: &[u8],
    ) -> Result<bool, VMError> {
        // Phase 1: Simulated verification
        // Phase 2: Full zk-SNARK verification of AI inference

        if proof.is_empty() {
            return Err(VMError::InvalidInstruction);
        }

        self.gas_meter.charge(5_000)?;

        Ok(true)
    }

    /// Returns the remaining gas
    pub fn remaining_gas(&self) -> u64 {
        self.gas_meter.remaining()
    }

    /// Returns the VM type
    pub fn vm_type() -> VMType {
        VMType::AI
    }
}

impl Default for AIVMExecutor {
    fn default() -> Self {
        Self::new(ExecutionContext::default(), 50_000_000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::primitives::hash::Hash512;

    #[test]
    fn ai_deploy_model() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        ];
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_ok());
    }

    #[test]
    fn ai_model_too_large() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = vec![0u8; 101 * 1024 * 1024]; // 101 MB > 100 MB limit
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::UnsupportedOperation);
    }

    #[test]
    fn ai_invalid_model() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = vec![0x00, 0x01];
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn ai_execute_inference() {
        let mut ai_vm = AIVMExecutor::default();
        let model_hash = Hash512::zero();
        let input = vec![1, 2, 3, 4, 5];
        let result = ai_vm.execute_inference(&model_hash, &input);
        assert!(result.is_ok());
    }

    #[test]
    fn ai_verify_inference() {
        let mut ai_vm = AIVMExecutor::default();
        let model_hash = Hash512::zero();
        let input = vec![1, 2, 3];
        let output = vec![4, 5, 6];
        let proof = vec![0x01];
        let result = ai_vm.verify_inference(&model_hash, &input, &output, &proof);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn ai_verify_empty_proof() {
        let mut ai_vm = AIVMExecutor::default();
        let model_hash = Hash512::zero();
        let input = vec![1];
        let output = vec![2];
        let proof = vec![];
        let result = ai_vm.verify_inference(&model_hash, &input, &output, &proof);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn ai_vm_type() {
        assert_eq!(AIVMExecutor::vm_type(), VMType::AI);
    }

    #[test]
    fn ai_out_of_gas() {
        let mut ai_vm = AIVMExecutor::new(ExecutionContext::default(), 5);
        let model_bytes = vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        ];
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_err());
    }
}
