//! Unified gas metering for the Atoll Protocol.
//!
//! Provides a shared [`GasMeter`] used by all six VMs, a standard gas cost
//! table, refund support, per-VM calibration factors, and operation gas
//! estimation.

use crate::vm::{VMError, VMType};

/// Standard gas costs for common on-chain operations.
///
/// These values are shared across all VMs so that gas metering is
/// consistent regardless of which VM executes a contract.
pub const GAS_BASE_TX: u64 = 21_000;
pub const GAS_STORAGE_WRITE: u64 = 20_000;
pub const GAS_STORAGE_READ: u64 = 200;
pub const GAS_MEMORY_EXPANSION_PER_WORD: u64 = 3;
pub const GAS_SIGNATURE_VERIFY: u64 = 3_000;
pub const GAS_HASH: u64 = 30;
pub const GAS_BALANCE_READ: u64 = 100;

/// Calibration multiplier for each VM type.
///
/// Different VMs have different instruction costs relative to a common
/// baseline. Calibration factors adjust the base gas cost so that the
/// same computational effort costs the same gas regardless of which VM
/// executes it. Values are applied as multipliers to base costs:
/// `effective_gas = base_gas * calibration_factor / 1000`.
///
/// Default factor is 1000 (1.0x, no adjustment).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VMCalibration {
    /// EVM calibration factor (default 1000 = 1.0x).
    pub evm: u16,
    /// WASM calibration factor (default 950 = 0.95x, slightly cheaper).
    pub wasm: u16,
    /// Move calibration factor (default 1050 = 1.05x).
    pub move_vm: u16,
    /// RISC-V calibration factor (default 1100 = 1.10x).
    pub riscv: u16,
    /// AI VM calibration factor (default 1500 = 1.50x, inference is expensive).
    pub ai: u16,
    /// ZK VM calibration factor (default 2000 = 2.00x, proof verification is expensive).
    pub zk: u16,
}

impl VMCalibration {
    /// Returns the calibration factor for the given VM type.
    pub fn factor_for(&self, vm_type: VMType) -> u16 {
        match vm_type {
            VMType::EVM => self.evm,
            VMType::WASM => self.wasm,
            VMType::Move => self.move_vm,
            VMType::RISCV => self.riscv,
            VMType::AI => self.ai,
            VMType::ZK => self.zk,
        }
    }
}

/// Default calibration: all VMs at 1.0x (factor 1000).
impl Default for VMCalibration {
    fn default() -> Self {
        Self {
            evm: 1000,
            wasm: 950,
            move_vm: 1050,
            riscv: 1100,
            ai: 1500,
            zk: 2000,
        }
    }
}

/// A gas-chargeable operation used by [`GasMeter::estimate_gas`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GasOp {
    BaseTx,
    StorageWrite,
    StorageRead,
    MemoryExpansion { words: u64 },
    SignatureVerify,
    Hash,
    BalanceRead,
}

/// Gas metering for VM execution.
///
/// Tracks the gas limit, gas used, and gas price for a single execution.
/// Charging fails with [`VMError::OutOfGas`] when the limit would be
/// exceeded, and no gas is consumed on a failed charge.
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

    /// Deducts `gas` from the meter.
    ///
    /// Returns [`VMError::OutOfGas`] if the charge would exceed the gas
    /// limit. No gas is consumed on failure.
    pub fn charge(&mut self, gas: u64) -> Result<(), VMError> {
        if self.gas_used.saturating_add(gas) > self.gas_limit {
            return Err(VMError::OutOfGas);
        }
        self.gas_used += gas;
        Ok(())
    }

    /// Deducts `gas` from the meter, applying a VM-specific calibration
    /// factor.
    ///
    /// The effective gas charged is `gas * calibration_factor / 1000`.
    /// Returns [`VMError::OutOfGas`] if the calibrated charge would exceed
    /// the gas limit. No gas is consumed on failure.
    pub fn charge_calibrated(
        &mut self,
        gas: u64,
        vm_type: VMType,
        calibration: &VMCalibration,
    ) -> Result<(), VMError> {
        let factor = calibration.factor_for(vm_type);
        let effective = gas.saturating_mul(factor as u64).saturating_div(1000);
        self.charge(effective)
    }

    /// Refunds `gas` back to the meter, reducing [`gas_used`](Self::gas_used).
    ///
    /// Refunds are clamped to the gas already used, so `gas_used` never
    /// drops below zero and never exceeds the limit. Returns the amount of
    /// gas actually refunded.
    pub fn refund(&mut self, gas: u64) -> u64 {
        let refunded = gas.min(self.gas_used);
        self.gas_used -= refunded;
        refunded
    }

    /// Returns the gas remaining before the limit is reached.
    pub fn remaining(&self) -> u64 {
        self.gas_limit.saturating_sub(self.gas_used)
    }

    /// Returns the total cost of the gas used so far (`gas_used * gas_price`).
    pub fn total_cost(&self) -> u128 {
        self.gas_used as u128 * self.gas_price
    }

    /// Estimates the gas cost of a common operation.
    pub fn estimate_gas(op: GasOp) -> u64 {
        match op {
            GasOp::BaseTx => GAS_BASE_TX,
            GasOp::StorageWrite => GAS_STORAGE_WRITE,
            GasOp::StorageRead => GAS_STORAGE_READ,
            GasOp::MemoryExpansion { words } => GAS_MEMORY_EXPANSION_PER_WORD * words,
            GasOp::SignatureVerify => GAS_SIGNATURE_VERIFY,
            GasOp::Hash => GAS_HASH,
            GasOp::BalanceRead => GAS_BALANCE_READ,
        }
    }

    /// Estimates the gas cost of a common operation with VM calibration
    /// applied.
    pub fn estimate_gas_calibrated(op: GasOp, vm_type: VMType, calibration: &VMCalibration) -> u64 {
        let base = Self::estimate_gas(op);
        let factor = calibration.factor_for(vm_type);
        base.saturating_mul(factor as u64).saturating_div(1000)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gas_meter_charge_and_remaining() {
        let mut meter = GasMeter::new(100, 1);
        assert!(meter.charge(50).is_ok());
        assert_eq!(meter.gas_used, 50);
        assert_eq!(meter.remaining(), 50);
        assert!(meter.charge(60).is_err());
        assert_eq!(meter.gas_used, 50); // no gas consumed on failure
    }

    #[test]
    fn gas_meter_charge_out_of_gas_error() {
        let mut meter = GasMeter::new(10, 1);
        assert_eq!(meter.charge(11), Err(VMError::OutOfGas));
        assert_eq!(meter.gas_used, 0);
    }

    #[test]
    fn gas_meter_charge_exactly_limit() {
        let mut meter = GasMeter::new(100, 1);
        assert!(meter.charge(100).is_ok());
        assert_eq!(meter.remaining(), 0);
        assert!(meter.charge(1).is_err());
    }

    #[test]
    fn gas_meter_refund_reduces_gas_used() {
        let mut meter = GasMeter::new(100, 1);
        meter.charge(70).unwrap();
        assert_eq!(meter.gas_used, 70);

        let refunded = meter.refund(30);
        assert_eq!(refunded, 30);
        assert_eq!(meter.gas_used, 40);
        assert_eq!(meter.remaining(), 60);
    }

    #[test]
    fn gas_meter_refund_clamped_to_used() {
        let mut meter = GasMeter::new(100, 1);
        meter.charge(10).unwrap();
        // Refunding more than used is clamped; gas_used cannot go below 0.
        let refunded = meter.refund(50);
        assert_eq!(refunded, 10);
        assert_eq!(meter.gas_used, 0);
        assert_eq!(meter.remaining(), 100);
    }

    #[test]
    fn gas_meter_total_cost() {
        let mut meter = GasMeter::new(1000, 10);
        meter.charge(100).unwrap();
        assert_eq!(meter.total_cost(), 1000);
    }

    #[test]
    fn gas_cost_table_values() {
        assert_eq!(GAS_BASE_TX, 21_000);
        assert_eq!(GAS_STORAGE_WRITE, 20_000);
        assert_eq!(GAS_STORAGE_READ, 200);
        assert_eq!(GAS_MEMORY_EXPANSION_PER_WORD, 3);
        assert_eq!(GAS_SIGNATURE_VERIFY, 3_000);
        assert_eq!(GAS_HASH, 30);
        assert_eq!(GAS_BALANCE_READ, 100);
    }

    #[test]
    fn estimate_gas_known_operations() {
        assert_eq!(GasMeter::estimate_gas(GasOp::BaseTx), 21_000);
        assert_eq!(GasMeter::estimate_gas(GasOp::StorageWrite), 20_000);
        assert_eq!(GasMeter::estimate_gas(GasOp::StorageRead), 200);
        assert_eq!(GasMeter::estimate_gas(GasOp::SignatureVerify), 3_000);
        assert_eq!(GasMeter::estimate_gas(GasOp::Hash), 30);
        assert_eq!(GasMeter::estimate_gas(GasOp::BalanceRead), 100);
        assert_eq!(
            GasMeter::estimate_gas(GasOp::MemoryExpansion { words: 10 }),
            3 * 10
        );
    }

    #[test]
    fn estimate_gas_scales_linearly_for_memory() {
        assert_eq!(
            GasMeter::estimate_gas(GasOp::MemoryExpansion { words: 1 }),
            GAS_MEMORY_EXPANSION_PER_WORD
        );
        assert_eq!(
            GasMeter::estimate_gas(GasOp::MemoryExpansion { words: 1_000 }),
            GAS_MEMORY_EXPANSION_PER_WORD * 1_000
        );
    }

    #[test]
    fn gas_meter_cannot_exceed_limit_after_refund_and_recharge() {
        let mut meter = GasMeter::new(100, 1);
        meter.charge(80).unwrap();
        meter.refund(40);
        assert_eq!(meter.gas_used, 40);
        // Recharging must still respect the limit: 40 + 60 = 100 exactly.
        assert!(meter.charge(60).is_ok());
        assert_eq!(meter.gas_used, 100);
        assert_eq!(meter.remaining(), 0);
        assert!(meter.charge(1).is_err());
    }

    // --- Cross-VM calibration tests ---

    #[test]
    fn vm_calibration_default_factors() {
        let cal = VMCalibration::default();
        assert_eq!(cal.factor_for(VMType::EVM), 1000);
        assert_eq!(cal.factor_for(VMType::WASM), 950);
        assert_eq!(cal.factor_for(VMType::Move), 1050);
        assert_eq!(cal.factor_for(VMType::RISCV), 1100);
        assert_eq!(cal.factor_for(VMType::AI), 1500);
        assert_eq!(cal.factor_for(VMType::ZK), 2000);
    }

    #[test]
    fn charge_calibrated_applies_factor() {
        let cal = VMCalibration::default();
        // EVM: 1000/1000 = 1.0x, no change
        let mut m = GasMeter::new(1000, 1);
        assert!(m.charge_calibrated(100, VMType::EVM, &cal).is_ok());
        assert_eq!(m.gas_used, 100);

        // WASM: 950/1000 = 0.95x, 100 -> 95
        let mut m = GasMeter::new(1000, 1);
        assert!(m.charge_calibrated(100, VMType::WASM, &cal).is_ok());
        assert_eq!(m.gas_used, 95);

        // ZK: 2000/1000 = 2.0x, 100 -> 200
        let mut m = GasMeter::new(1000, 1);
        assert!(m.charge_calibrated(100, VMType::ZK, &cal).is_ok());
        assert_eq!(m.gas_used, 200);
    }

    #[test]
    fn charge_calibrated_out_of_gas_respects_limit() {
        let cal = VMCalibration::default();
        // ZK: 60 * 2000/1000 = 120, exceeds limit of 100
        let mut m = GasMeter::new(100, 1);
        assert_eq!(
            m.charge_calibrated(60, VMType::ZK, &cal),
            Err(VMError::OutOfGas)
        );
        assert_eq!(m.gas_used, 0);
    }

    #[test]
    fn estimate_gas_calibrated_applies_factor() {
        let cal = VMCalibration::default();
        // EVM: 21000 * 1000/1000 = 21000
        assert_eq!(
            GasMeter::estimate_gas_calibrated(GasOp::BaseTx, VMType::EVM, &cal),
            21_000
        );
        // WASM: 21000 * 950/1000 = 19950
        assert_eq!(
            GasMeter::estimate_gas_calibrated(GasOp::BaseTx, VMType::WASM, &cal),
            19_950
        );
        // ZK: 21000 * 2000/1000 = 42000
        assert_eq!(
            GasMeter::estimate_gas_calibrated(GasOp::BaseTx, VMType::ZK, &cal),
            42_000
        );
    }

    #[test]
    fn cross_vm_consistency_same_base_gas_produces_proportional_costs() {
        let cal = VMCalibration::default();
        let base = GasMeter::estimate_gas(GasOp::BaseTx); // 21_000

        let evm_cost = GasMeter::estimate_gas_calibrated(GasOp::BaseTx, VMType::EVM, &cal);
        let wasm_cost = GasMeter::estimate_gas_calibrated(GasOp::BaseTx, VMType::WASM, &cal);
        let zk_cost = GasMeter::estimate_gas_calibrated(GasOp::BaseTx, VMType::ZK, &cal);

        // EVM is baseline (1.0x)
        assert_eq!(evm_cost, base);
        // WASM is cheaper (0.95x)
        assert!(wasm_cost < evm_cost);
        assert_eq!(wasm_cost, base * 950 / 1000);
        // ZK is more expensive (2.0x)
        assert!(zk_cost > evm_cost);
        assert_eq!(zk_cost, base * 2000 / 1000);

        // Same base gas across VM types yields proportional costs
        // (deterministic cross-VM gas consistency)
        assert_eq!(
            evm_cost as f64 / cal.factor_for(VMType::EVM) as f64 * 1000.0,
            wasm_cost as f64 / cal.factor_for(VMType::WASM) as f64 * 1000.0
        );
    }

    #[test]
    fn calibration_factor_zero_means_free() {
        let mut cal = VMCalibration::default();
        cal.evm = 0; // EVM is free
        let mut m = GasMeter::new(1000, 1);
        assert!(m.charge_calibrated(10_000, VMType::EVM, &cal).is_ok());
        assert_eq!(m.gas_used, 0);
    }

    #[test]
    fn gas_used_never_exceeds_limit_with_calibration() {
        let cal = VMCalibration::default();
        let mut m = GasMeter::new(100, 1);

        // Charge many times with ZK (2.0x) — each charge of 1 costs 2
        for _ in 0..50 {
            assert!(m.charge_calibrated(1, VMType::ZK, &cal).is_ok());
        }
        assert_eq!(m.gas_used, 100); // exactly at limit
        assert!(m.charge_calibrated(1, VMType::ZK, &cal).is_err()); // would exceed
    }
}
