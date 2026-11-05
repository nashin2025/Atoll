//! AI Virtual Machine for the Atoll Protocol.
//!
//! Executes AI model inference on-chain using ONNX-compatible models.
//! Results are deterministic and verifiable with zk-SNARK proofs.
//!
//! Phase 1: Model deployment and deterministic inference simulation.
//! Phase 2: Full ONNX runtime with zk-SNARK verification proofs.
//!
//! Production hardening additions:
//! - Model registry with content-addressable storage (BLAKE3 hash)
//! - Model size limits and input validation
//! - Deterministic inference output based on model_hash + input
//! - Proof structure validation (non-empty, size limits, format check)
//! - Gas metering for deployment, inference, and verification
//! - Panic safety: all external inputs validated before processing

use crate::primitives::hash::Hash512;
use crate::vm::{ExecutionContext, ExecutionResult, GasMeter, VMError, VMType};

/// Maximum model size: 100 MB
const MAX_MODEL_SIZE: usize = 100 * 1024 * 1024;
/// Maximum inference gas per request
const MAX_INFERENCE_GAS: u64 = 10_000_000;
/// Minimum proof size for verification
const MIN_PROOF_SIZE: usize = 32;
/// Maximum proof size for verification
const MAX_PROOF_SIZE: usize = 10 * 1024; // 10 KB
/// Gas per KB for model deployment
const GAS_PER_KB_DEPLOY: u64 = 100;
/// Base gas for model validation
const GAS_MODEL_VALIDATION: u64 = 10_000;
/// Base gas for model lookup
const GAS_MODEL_LOOKUP: u64 = 1_000;
/// Base gas for inference verification
const GAS_INFERENCE_VERIFY: u64 = 5_000;

/// Deployed model record
#[derive(Debug, Clone)]
struct DeployedModel {
    /// Model content hash (BLAKE3-512)
    hash: Hash512,
    /// Model size in bytes
    size: usize,
    /// Deployment gas cost
    deploy_gas: u64,
}

/// AI VM executor for on-chain AI inference
pub struct AIVMExecutor {
    pub gas_meter: GasMeter,
    pub context: ExecutionContext,
    /// Deployed models indexed by hash
    model_registry: Vec<DeployedModel>,
    pub max_model_size: usize,
    pub max_inference_gas: u64,
}

impl AIVMExecutor {
    pub fn new(context: ExecutionContext, gas_limit: u64) -> Self {
        Self {
            gas_meter: GasMeter::new(gas_limit, context.gas_price),
            context,
            model_registry: Vec::new(),
            max_model_size: MAX_MODEL_SIZE,
            max_inference_gas: MAX_INFERENCE_GAS,
        }
    }

    /// Deploys an AI model on-chain with content-addressable storage
    pub fn deploy_model(&mut self, model_bytes: &[u8]) -> Result<ExecutionResult, VMError> {
        // Validate model is non-empty and has minimum ONNX header size
        if model_bytes.len() < 12 {
            return Err(VMError::InvalidInstruction);
        }

        // Validate model size limit
        if model_bytes.len() > self.max_model_size {
            return Err(VMError::UnsupportedOperation);
        }

        // Compute model hash (content-addressable)
        let model_hash = Hash512::blake3(model_bytes);

        // Check for duplicate deployment
        if self.model_registry.iter().any(|m| m.hash == model_hash) {
            return Err(VMError::UnsupportedOperation);
        }

        // Charge gas for model deployment (per KB)
        let deploy_gas = (model_bytes.len() as u64 / 1024 + 1) * GAS_PER_KB_DEPLOY;
        self.gas_meter.charge(deploy_gas)?;

        // Charge gas for model validation
        self.gas_meter.charge(GAS_MODEL_VALIDATION)?;

        // Register the model
        self.model_registry.push(DeployedModel {
            hash: model_hash,
            size: model_bytes.len(),
            deploy_gas,
        });

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_value: model_hash.to_bytes(),
            logs: vec![],
        })
    }

    /// Executes AI model inference deterministically
    pub fn execute_inference(
        &mut self,
        model_hash: &Hash512,
        input_data: &[u8],
    ) -> Result<ExecutionResult, VMError> {
        // Validate model is deployed
        if !self.model_registry.iter().any(|m| m.hash == *model_hash) {
            return Err(VMError::InvalidInstruction);
        }

        // Validate input is non-empty
        if input_data.is_empty() {
            return Err(VMError::InvalidInstruction);
        }

        // Charge gas for model lookup
        self.gas_meter.charge(GAS_MODEL_LOOKUP)?;

        // Charge gas for inference based on input size
        let inference_gas = (input_data.len() as u64 / 1024 + 1) * 100;
        self.gas_meter
            .charge(inference_gas.min(self.max_inference_gas))?;

        // Deterministic inference: output derived from model_hash + input
        let mut output = Vec::new();
        output.extend_from_slice(&model_hash.0);
        output.extend_from_slice(&(input_data.len() as u64).to_le_bytes());
        // Deterministic hash of (model_hash || input) for output content
        let combined: Vec<u8> = model_hash
            .0
            .iter()
            .chain(input_data.iter())
            .copied()
            .collect();
        let output_hash = Hash512::blake3(&combined);
        output.extend_from_slice(&output_hash.to_bytes()[..16]);

        Ok(ExecutionResult::Success {
            gas_used: self.gas_meter.gas_used,
            return_value: output,
            logs: vec![],
        })
    }

    /// Verifies an AI inference proof
    pub fn verify_inference(
        &mut self,
        model_hash: &Hash512,
        input_data: &[u8],
        output: &[u8],
        proof: &[u8],
    ) -> Result<bool, VMError> {
        // Validate model is deployed
        if !self.model_registry.iter().any(|m| m.hash == *model_hash) {
            return Err(VMError::InvalidInstruction);
        }

        // Validate proof size bounds
        if proof.len() < MIN_PROOF_SIZE || proof.len() > MAX_PROOF_SIZE {
            return Err(VMError::InvalidInstruction);
        }

        // Validate output is non-empty
        if output.is_empty() {
            return Err(VMError::InvalidInstruction);
        }

        // Charge gas for verification
        self.gas_meter.charge(GAS_INFERENCE_VERIFY)?;

        // Deterministic verification: recompute expected output and compare
        let mut combined: Vec<u8> = model_hash
            .0
            .iter()
            .chain(input_data.iter())
            .copied()
            .collect();
        let expected_hash = Hash512::blake3(&combined);
        let expected_output: Vec<u8> = model_hash
            .0
            .iter()
            .chain(&(input_data.len() as u64).to_le_bytes())
            .chain(&expected_hash.to_bytes()[..16])
            .copied()
            .collect();

        // Proof must match expected output structure
        let proof_valid = proof.len() == expected_output.len()
            && proof
                .iter()
                .zip(expected_output.iter())
                .all(|(a, b)| a == b);

        Ok(proof_valid)
    }

    /// Returns the remaining gas
    #[must_use]
    pub fn remaining_gas(&self) -> u64 {
        self.gas_meter.remaining()
    }

    /// Returns the VM type
    #[must_use]
    pub fn vm_type() -> VMType {
        VMType::AI
    }

    /// Returns the number of deployed models
    #[must_use]
    pub fn model_count(&self) -> usize {
        self.model_registry.len()
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

    fn make_test_model() -> Vec<u8> {
        vec![
            0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B,
        ]
    }

    #[test]
    fn ai_deploy_model() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = make_test_model();
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_ok());
        assert_eq!(ai_vm.model_count(), 1);
    }

    #[test]
    fn ai_deploy_duplicate_model_rejected() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = make_test_model();
        ai_vm.deploy_model(&model_bytes).unwrap();
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::UnsupportedOperation);
    }

    #[test]
    fn ai_model_too_large() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = vec![0u8; MAX_MODEL_SIZE + 1];
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::UnsupportedOperation);
    }

    #[test]
    fn ai_empty_model_rejected() {
        let mut ai_vm = AIVMExecutor::default();
        let result = ai_vm.deploy_model(&[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
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
        let model_bytes = make_test_model();
        ai_vm.deploy_model(&model_bytes).unwrap();
        let model_hash = Hash512::blake3(&model_bytes);
        let input = vec![1, 2, 3, 4, 5];
        let result = ai_vm.execute_inference(&model_hash, &input);
        assert!(result.is_ok());
    }

    #[test]
    fn ai_inference_undeployed_model_rejected() {
        let mut ai_vm = AIVMExecutor::default();
        let model_hash = Hash512::zero();
        let input = vec![1, 2, 3];
        let result = ai_vm.execute_inference(&model_hash, &input);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn ai_inference_empty_input_rejected() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = make_test_model();
        ai_vm.deploy_model(&model_bytes).unwrap();
        let model_hash = Hash512::blake3(&model_bytes);
        let result = ai_vm.execute_inference(&model_hash, &[]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn ai_inference_deterministic() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = make_test_model();
        ai_vm.deploy_model(&model_bytes).unwrap();
        let model_hash = Hash512::blake3(&model_bytes);
        let input = vec![1, 2, 3, 4, 5];

        let r1 = ai_vm.execute_inference(&model_hash, &input).unwrap();
        let r2 = ai_vm.execute_inference(&model_hash, &input).unwrap();
        match r1 {
            ExecutionResult::Success {
                return_value: v1, ..
            } => match r2 {
                ExecutionResult::Success {
                    return_value: v2, ..
                } => {
                    assert_eq!(v1, v2);
                }
                _ => panic!("Expected Success"),
            },
            _ => panic!("Expected Success"),
        }
    }

    #[test]
    fn ai_verify_inference() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = make_test_model();
        ai_vm.deploy_model(&model_bytes).unwrap();
        let model_hash = Hash512::blake3(&model_bytes);
        let input = vec![1, 2, 3];
        let output = match ai_vm.execute_inference(&model_hash, &input).unwrap() {
            ExecutionResult::Success { return_value, .. } => return_value,
            _ => panic!("Expected Success"),
        };
        let proof = output.clone();
        let result = ai_vm.verify_inference(&model_hash, &input, &output, &proof);
        assert!(result.is_ok());
        assert!(result.unwrap());
    }

    #[test]
    fn ai_verify_empty_proof_rejected() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = make_test_model();
        ai_vm.deploy_model(&model_bytes).unwrap();
        let model_hash = Hash512::blake3(&model_bytes);
        let input = vec![1];
        let output = vec![2];
        let proof = vec![];
        let result = ai_vm.verify_inference(&model_hash, &input, &output, &proof);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::InvalidInstruction);
    }

    #[test]
    fn ai_verify_wrong_proof_rejected() {
        let mut ai_vm = AIVMExecutor::default();
        let model_bytes = make_test_model();
        ai_vm.deploy_model(&model_bytes).unwrap();
        let model_hash = Hash512::blake3(&model_bytes);
        let input = vec![1, 2, 3];
        let output = match ai_vm.execute_inference(&model_hash, &input).unwrap() {
            ExecutionResult::Success { return_value, .. } => return_value,
            _ => panic!("Expected Success"),
        };
        let wrong_proof = vec![0xFF; output.len()];
        let result = ai_vm.verify_inference(&model_hash, &input, &output, &wrong_proof);
        assert!(result.is_ok());
        assert!(!result.unwrap());
    }

    #[test]
    fn ai_vm_type() {
        assert_eq!(AIVMExecutor::vm_type(), VMType::AI);
    }

    #[test]
    fn ai_out_of_gas() {
        let mut ai_vm = AIVMExecutor::new(ExecutionContext::default(), 5);
        let model_bytes = make_test_model();
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), VMError::OutOfGas);
    }

    #[test]
    fn ai_remaining_gas() {
        let mut ai_vm = AIVMExecutor::new(ExecutionContext::default(), 20_000);
        let model_bytes = make_test_model();
        let result = ai_vm.deploy_model(&model_bytes);
        assert!(result.is_ok());
        assert!(ai_vm.remaining_gas() < 20_000);
    }
}
