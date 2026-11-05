//! Unified gas metering for the Atoll Protocol.
//!
//! Provides a shared [`GasMeter`] used by all six VMs, a standard gas cost
//! table, refund support, and operation gas estimation.

use crate::vm::VMError;

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
}
